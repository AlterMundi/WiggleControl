use std::{fs, path::PathBuf};
use wigglecontrol::{
    calibration::Profile, provider_calibration::Run, touch_gestures::Workspace, Point,
};

fn fixture(name: &str) -> (PathBuf, Run) {
    let root = std::env::temp_dir().join(format!(
        "wiggle-provider-calibration-{name}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("profile.json");
    wigglecontrol::profile_store::adopt(
        &path,
        Profile {
            version: 1,
            output: [1920, 1080],
            workspace: Workspace::default(),
        },
    )
    .unwrap();
    let run = Run::new(&path, [1920, 1080]).unwrap();
    (root, run)
}
fn sample(run: &mut Run, point: Point, time: u64) {
    run.observe(
        Some(point),
        time,
        time,
        "Calibration: hand visible; hold steady",
    );
}
fn poses(run: &mut Run) -> u64 {
    let mut at = 1_000_000;
    for (step, point) in [(0, Point { x: 0.2, y: 0.3 }), (1, Point { x: 0.8, y: 0.7 })] {
        sample(run, point, at);
        run.acknowledge(step, at).unwrap();
        for n in 0..50 {
            sample(run, point, at + 15_000_000 + n * 100_000);
        }
        run.tick(at + 20_000_000);
        at += 30_000_000;
    }
    assert_eq!(run.state(at)["review"]["stage"], "review");
    assert_eq!(run.state(at)["guide"]["step"], 2);
    assert!(
        !run.finished(),
        "Two diagonal inputs must not end before four-corner review"
    );
    at
}
#[test]
fn all_four_corners_and_camera_closure_precede_explicit_save_and_keep_a_previous_profile() {
    let (root, mut run) = fixture("save");
    let path = root.join("profile.json");
    let old = fs::read(&path).unwrap();
    assert_eq!(run.state(0)["phase"], "idle");
    assert!(run.acknowledge(0, 0).is_err());
    assert!(run.save().is_err());
    run.start().unwrap();
    assert!(
        run.acknowledge(0, 0).is_err(),
        "A painted screen before a source frame cannot start the step clock"
    );
    let mut at = poses(&mut run);
    for (step, point) in [
        Point { x: 0.2, y: 0.3 },
        Point { x: 0.8, y: 0.3 },
        Point { x: 0.8, y: 0.7 },
        Point { x: 0.2, y: 0.7 },
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            run.acknowledge(step + 3, at).is_err(),
            "Reject a painted acknowledgment for another step"
        );
        run.acknowledge(step + 2, at).unwrap();
        for n in 0..6 {
            sample(&mut run, point, at + n * 100_000);
        }
        at += 2_000_000;
        assert_eq!(
            fs::read(&path).unwrap(),
            old,
            "Review cannot silently replace the saved profile"
        );
    }
    assert_eq!(run.state(at)["phase"], "complete");
    assert!(!run.camera_closed());
    assert!(
        run.save().is_err(),
        "Even verified corners cannot save while capture remains open"
    );
    run.confirm_camera_closed();
    run.save().unwrap();
    assert!(run.done());
    assert_eq!(run.state(at)["phase"], "saved");
    let adopted = wigglecontrol::profile_store::load(&path).unwrap();
    assert!((adopted.workspace.min.x - 0.23).abs() < 1e-9);
    assert_eq!(
        serde_json::to_vec(&wigglecontrol::profile_store::previous_profile(&path).unwrap())
            .unwrap(),
        serde_json::to_vec(&serde_json::from_slice::<Profile>(&old).unwrap()).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_attempt_retries_only_after_closure_and_a_new_start_then_cancel_preserves_saved_bytes() {
    let (root, mut run) = fixture("retry");
    let old = fs::read(root.join("profile.json")).unwrap();
    run.start().unwrap();
    sample(&mut run, Point { x: 0.2, y: 0.3 }, 1_000_000);
    run.acknowledge(0, 1_000_000).unwrap();
    // Camera continues producing observations but the hand is out of frame.
    for n in 0..20 {
        run.observe(
            None,
            2_000_000 + n * 1_000_000,
            2_000_000 + n * 1_000_000,
            "Calibration: hand partly outside camera; move inward",
        );
    }
    assert_eq!(run.state(21_000_000)["phase"], "retry");
    assert!(run.retry().is_err());
    assert!(run.save().is_err());
    run.confirm_camera_closed();
    run.retry().unwrap();
    assert_eq!(run.state(22_000_000)["phase"], "idle");
    assert!(!run.running(), "Retry does not request acquisition");
    assert!(run.acknowledge(0, 22_000_000).is_err());
    run.start().unwrap();
    run.limit_reached();
    assert_eq!(run.state(23_000_000)["phase"], "retry");
    run.confirm_camera_closed();
    run.retry().unwrap();
    run.start().unwrap();
    run.cancel().unwrap();
    assert!(run.done());
    assert_eq!(fs::read(root.join("profile.json")).unwrap(), old);
    assert!(!root.join("profile.json.draft").exists());
    assert!(!root.join("profile.json.verified").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_frames_reset_corner_dwell_and_never_reuse_a_diagonal_as_verified_review() {
    let (root, mut run) = fixture("loss");
    run.start().unwrap();
    let at = poses(&mut run);
    run.acknowledge(2, at).unwrap();
    for n in 0..4 {
        sample(&mut run, Point { x: 0.2, y: 0.3 }, at + n * 100_000);
    }
    run.tick(at + 600_000);
    assert_eq!(run.state(at + 600_000)["guide"]["phase"], "move");
    sample(&mut run, Point { x: 0.2, y: 0.3 }, at + 700_000);
    assert_eq!(run.state(at + 700_000)["guide"]["step"], 2);
    assert!(run.save().is_err());
    run.cancel().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn regular_delayed_observations_survive_interframe_ticks_and_verify_all_corners() {
    let (root, mut run) = fixture("delayed-cadence");
    let old = fs::read(root.join("profile.json")).unwrap();
    run.start().unwrap();
    // Source cadence100ms plus70ms inference: each observation is fresh, but
    // the previous source is169ms old just before the next result arrives.
    let mut at = 1_000_000;
    for (step, point) in [(0, Point { x: 0.2, y: 0.3 }), (1, Point { x: 0.8, y: 0.7 })] {
        run.observe(
            Some(point),
            at - 70_000,
            at,
            "Calibration: hand visible; hold steady",
        );
        run.acknowledge(step, at).unwrap();
        for n in 1..200 {
            let receipt = at + n * 100_000;
            run.tick(receipt - 1_000);
            run.observe(
                Some(point),
                receipt - 70_000,
                receipt,
                "Calibration: hand visible; hold steady",
            );
        }
        run.tick(at + 20_000_000);
        assert_eq!(run.state(at + 20_000_000)["guide"]["step"], step + 1);
        at += 21_000_000;
    }
    assert_eq!(run.state(at)["review"]["stage"], "review");
    for (step, point) in [
        Point { x: 0.2, y: 0.3 },
        Point { x: 0.8, y: 0.3 },
        Point { x: 0.8, y: 0.7 },
        Point { x: 0.2, y: 0.7 },
    ]
    .into_iter()
    .enumerate()
    {
        run.acknowledge(step + 2, at).unwrap();
        for n in 0..6 {
            let receipt = at + n * 100_000;
            run.tick(receipt - 1_000);
            run.observe(
                Some(point),
                receipt - 70_000,
                receipt,
                "Calibration: hand visible; hold steady",
            );
        }
        at += 1_000_000;
    }
    assert_eq!(run.state(at)["phase"], "complete");
    assert!(
        run.save().is_err(),
        "Timing tolerance does not waive camera closure"
    );
    assert_eq!(fs::read(root.join("profile.json")).unwrap(), old);
    run.cancel().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn regularly_delivered_stale_sources_still_cannot_supply_diagonal_samples() {
    let (root, mut run) = fixture("stale-delivery");
    run.start().unwrap();
    let at = 1_000_000;
    run.observe(
        Some(Point { x: 0.2, y: 0.3 }),
        at - 150_000,
        at,
        "Calibration: frame delayed",
    );
    run.acknowledge(0, at).unwrap();
    for n in 1..=200 {
        let receipt = at + n * 100_000;
        run.tick(receipt - 1_000);
        run.observe(
            Some(Point { x: 0.2, y: 0.3 }),
            receipt - 150_000,
            receipt,
            "Calibration: frame delayed",
        );
    }
    assert_eq!(run.state(at + 20_000_000)["phase"], "retry");
    assert_eq!(run.state(at + 20_000_000)["review"]["samples"], 0);
    assert!(run.save().is_err());
    run.cancel().unwrap();
    fs::remove_dir_all(root).unwrap();
}
