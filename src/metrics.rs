//! Whole-session Linux CPU accounting; memory peaks remain separate, never summed.
use serde::Serialize;
use std::time::Instant;
#[derive(Clone, Copy)]
struct Usage {
    cpu_seconds: f64,
    peak_rss_kib: u64,
}
fn usage(who: libc::c_int) -> Option<Usage> {
    let mut value: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(who, &mut value) } != 0 {
        return None;
    }
    let seconds = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1_000_000.0;
    Some(Usage {
        cpu_seconds: seconds(value.ru_utime) + seconds(value.ru_stime),
        peak_rss_kib: u64::try_from(value.ru_maxrss).ok()?,
    })
}
fn cpu_percent(before: f64, after: f64, wall: f64) -> Option<f64> {
    if !before.is_finite()
        || !after.is_finite()
        || !wall.is_finite()
        || after < before
        || wall <= 0.0
    {
        return None;
    }
    Some((after - before) / wall * 100.0)
}
pub struct Meter {
    start: Instant,
    own: Option<Usage>,
    children: Option<Usage>,
}
#[derive(Serialize)]
pub struct Resources {
    pub measured_wall_seconds: f64,
    /// 100% is one core, including all local runtime threads and reaped writers.
    pub process_tree_cpu_percent_one_core: Option<f64>,
    /// Lifetime peaks, not simultaneous aggregate memory or allocation estimates.
    pub runtime_peak_rss_kib: Option<u64>,
    pub reaped_children_peak_rss_kib: Option<u64>,
}
impl Meter {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            own: usage(libc::RUSAGE_SELF),
            children: usage(libc::RUSAGE_CHILDREN),
        }
    }
    /// Call after child wait and camera-thread join so their CPU is included.
    pub fn finish(&self) -> Resources {
        let own = usage(libc::RUSAGE_SELF);
        let children = usage(libc::RUSAGE_CHILDREN);
        let wall = self.start.elapsed().as_secs_f64();
        let cpu =
            self.own
                .zip(self.children)
                .zip(own.zip(children))
                .and_then(|((a, b), (c, d))| {
                    cpu_percent(
                        a.cpu_seconds + b.cpu_seconds,
                        c.cpu_seconds + d.cpu_seconds,
                        wall,
                    )
                });
        Resources {
            measured_wall_seconds: wall,
            process_tree_cpu_percent_one_core: cpu,
            runtime_peak_rss_kib: own.map(|u| u.peak_rss_kib),
            reaped_children_peak_rss_kib: children.map(|u| u.peak_rss_kib),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_includes_multiple_cores_and_unknown_counter_regression() {
        assert_eq!(cpu_percent(5.0, 9.0, 2.0), Some(200.0));
        assert_eq!(cpu_percent(5.0, 4.0, 2.0), None);
        assert_eq!(cpu_percent(5.0, 6.0, 0.0), None);
        assert_eq!(cpu_percent(5.0, f64::NAN, 2.0), None);
    }
}
