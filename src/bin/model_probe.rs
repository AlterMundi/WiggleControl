//! Offline exact-model compatibility and synthetic inference timing, no camera.
use std::{error::Error, time::Instant};
use tract_onnx::prelude::*;
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !(args.len() == 3 || (args.len() == 4 && args[3] == "--no-opt")) {
        return Err("Usage: wigglecontrol-model-probe MODEL.onnx INPUT_SIDE RUNS [--no-opt] (synthetic zero RGB crop; no camera)".into());
    }
    let side: usize = args[1].parse()?;
    let runs: usize = args[2].parse()?;
    if ![192, 224].contains(&side) || !(1..=100).contains(&runs) {
        return Err("side must be 192/224; runs 1..100".into());
    }
    let load_start = Instant::now();
    let inferred = tract_onnx::onnx()
        .model_for_path(&args[0])?
        .with_input_fact(0, f32::fact([1, side, side, 3]).into())?;
    let typed = if args.len() == 4 {
        inferred.into_typed()?
    } else {
        inferred.into_optimized()?
    };
    let model = typed.into_runnable()?;
    eprintln!("Loaded plan in {:.3}s", load_start.elapsed().as_secs_f64());
    let tensor: Tensor = tract_ndarray::Array4::<f32>::zeros((1, side, side, 3)).into();
    let warm = model.run(tvec!(tensor.clone().into()))?;
    for (i, output) in warm.iter().enumerate() {
        println!("output {i}: {:?} {:?}", output.datum_type(), output.shape());
        if output.datum_type() == f32::datum_type()
            && !output.to_array_view::<f32>()?.iter().all(|v| v.is_finite())
        {
            return Err("nonfinite output".into());
        }
    }
    let mut timings = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let _ = model.run(tvec!(tensor.clone().into()))?;
        timings.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    timings.sort_by(f64::total_cmp);
    println!(
        "Synthetic model-only milliseconds: p50={:.3} p95={:.3} runs={runs}",
        timings[runs / 2],
        timings[(runs * 95).div_ceil(100).saturating_sub(1).min(runs - 1)]
    );
    println!("Not a hand accuracy test, camera pipeline, tracking implementation or capture-to-input benchmark.");
    Ok(())
}
