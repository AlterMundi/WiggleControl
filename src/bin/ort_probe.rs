//! Native CPU runtime model compatibility; synthetic crops, no sensor access.
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use std::{error::Error, time::Instant};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 && args.len() != 5 {
        return Err("Usage: wigglecontrol-ort-probe MODEL.onnx SIDE RUNS [INPUT.f32 NEW_REPORT.json]; set ORT_DYLIB_PATH to pinned CPU runtime".into());
    }
    let side: usize = args[1].parse()?;
    let runs: usize = args[2].parse()?;
    if ![192, 224].contains(&side) || !(1..=100).contains(&runs) {
        return Err("side 192/224; runs 1..100".into());
    }
    let start = Instant::now();
    let mut model = Session::builder()?
        .with_intra_threads(1)?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file(&args[0])?;
    println!(
        "Loaded model {:.3}s, CPU intra_threads=1",
        start.elapsed().as_secs_f64()
    );
    let values = if args.len() == 5 {
        if std::fs::metadata(&args[3])?.len() != (side * side * 3 * 4) as u64 {
            return Err("unexpected float32 RGB input size".into());
        }
        let bytes = std::fs::read(&args[3])?;
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("four-byte chunk")))
            .collect();
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.000001).contains(v))
        {
            return Err("invalid float32 RGB input".into());
        }
        Some(values)
    } else {
        None
    };
    let input = Tensor::from_array((
        [1usize, side, side, 3],
        values.unwrap_or_else(|| vec![0.0f32; side * side * 3]),
    ))?;
    let mut outputs = Vec::new();
    {
        let result = model.run(ort::inputs![input.view()])?;
        for (name, output) in result.iter() {
            let (shape, values) = output.try_extract_tensor::<f32>()?;
            println!("{name}: {shape:?}");
            if !values.iter().all(|n| n.is_finite()) {
                return Err("nonfinite model output".into());
            }
            outputs.push(serde_json::json!({"name":name,"shape":shape.as_ref(),"values":values}));
        }
    }
    let mut timings = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let _ = model.run(ort::inputs![input.view()])?;
        timings.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    timings.sort_by(f64::total_cmp);
    println!(
        "Synthetic model-only milliseconds: p50={:.3} p95={:.3} runs={runs}",
        timings[runs / 2],
        timings[(runs * 95).div_ceil(100).saturating_sub(1).min(runs - 1)]
    );
    println!("No hand accuracy, tracking, capture-to-event or sustained power qualification.");
    if let Some(path) = args.get(4) {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&serde_json::to_vec_pretty(&serde_json::json!({
            "schema":"wigglecontrol.ort-tensor-probe/v1","camera_opened":false,"system_input":false,
            "input_shape":[1,side,side,3],"input_dtype":"little-endian float32",
            "outputs":outputs,"runs":runs,"timings_ms":timings,"cpu_intra_threads":1
        }))?)?;
    }
    Ok(())
}
