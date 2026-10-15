// Camera-free optional provider comparison; all assets stay on loopback.
// A browser backend is not stable hand identity or delivered system input.
export async function runBrowserHandSequence(baseUrl, manifest) {
  if (!/^http:\/\/127\.0\.0\.1:\d+\/$/.test(baseUrl)) throw Error('Expected receiving loopback assets');
  if (manifest.package_version !== '1.1.1') throw Error('Expected pinned Tasks Vision1.1.1');
  const hash = async bytes => [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(v => v.toString(16).padStart(2, '0')).join('');
  const modelBytes = await (await fetch(baseUrl+'hand_landmarker.task')).arrayBuffer();
  if (await hash(modelBytes) !== manifest.model_sha256) throw Error('Model digest differs');
  const {FilesetResolver, HandLandmarker} = await import(baseUrl+'package/vision_bundle.mjs');
  const files = await FilesetResolver.forVisionTasks(baseUrl+'package/wasm');
  const results = [];
  const runId = crypto.randomUUID();
  let seq = 0;
  const publish = state => window.receiveRunState?.({...state, runId, seq:++seq});
  for (const delegate of ['CPU', 'GPU']) {
    for (const scene of manifest.scenes) {
      if (scene.frames.length < 9 || scene.frames.length > 121) throw Error('Unbounded scene');
      publish({status:'running', phase:'browser model initialization', label:delegate+' / '+scene.name+' / camera off', frame:0, total:scene.frames.length});
      const initialization = performance.now();
      const canvas = document.createElement('canvas');
      const model = await HandLandmarker.createFromOptions(files, {
        baseOptions:{modelAssetBuffer:new Uint8Array(modelBytes), delegate}, canvas,
        runningMode:'VIDEO', numHands:2, minHandDetectionConfidence:.5,
        minHandPresenceConfidence:.5, minTrackingConfidence:.5
      });
      const initializationMs = performance.now()-initialization;
      const predictions = [];
      let last = -1;
      const started = performance.now();
      try {
        for (let index=0; index<scene.frames.length; index++) {
          if (performance.now()-started > 60000) throw Error('Finite scene timeout');
          const telemetry = await (await fetch(baseUrl+'telemetry')).json();
          if (telemetry.temperature_c >= 80) throw Error('Receiving temperature budget reached');
          const frame = scene.frames[index];
          const timeMs = Math.floor(frame.source_us/1000);
          if (timeMs <= last) throw Error('Non-monotonic source time');
          last = timeMs;
          const begin = performance.now();
          const bytes = await (await fetch(baseUrl+scene.name+'/'+frame.image)).arrayBuffer();
          if (await hash(bytes) !== frame.sha256) throw Error('Image digest differs');
          const url = URL.createObjectURL(new Blob([bytes], {type:'image/png'}));
          const image = new Image();
          try {
            image.src = url; await image.decode();
            const inference = performance.now();
            const output = model.detectForVideo(image, timeMs);
            const elapsed = performance.now();
            const hands = output.landmarks.map((points, i) => {
              const world = output.worldLandmarks[i].map(p => [p.x,p.y,p.z]);
              return {array_index:i, stable_id_available:false,
                landmarks:points.map(p=>[p.x*image.width,p.y*image.height]),
                world_landmarks:world, handedness:output.handedness[i],
                world_pinch_distance_m:Math.hypot(...world[4].map((v,j)=>v-world[8][j]))};
            });
            const row = {frame:index, source_us:frame.source_us, image:frame.image, hands,
              image_sha256:frame.sha256, decode_hash_fetch_ms:inference-begin,
              inference_ms:elapsed-inference, processing_ms:elapsed-begin, telemetry,
              camera_opened:false, system_input:false, contacts:null, ambiguous:null};
            predictions.push(row);
            publish({status:'running', phase:'actual browser '+delegate+' inference',
              label:delegate+' / '+scene.name+' / no gesture or owner evaluation', frame:index,
              total:scene.frames.length, image:baseUrl+scene.name+'/'+frame.image,
              hands:hands.length, contacts:null, model_gap_m:hands[0]?.world_pinch_distance_m,
              skin_gap_m:scene.reference.frames[index].distal_skin?.surface_gap_m});
          } finally { URL.revokeObjectURL(url); }
          await new Promise(resolve=>requestAnimationFrame(resolve));
        }
      } finally { model.close(); canvas.remove(); }
      results.push({delegate, scene:scene.name, initialization_ms:initializationMs,
        reference:scene.reference, predictions, camera_opened:false, system_input:false});
      window.wiggleBrowserProgress = {completed:results.length, results};
    }
  }
  const result = {schema:'wigglecontrol.browser-provider-comparison/v1', package_version:manifest.package_version,
    model_sha256:manifest.model_sha256, results, camera_opened:false, system_input:false,
    limits:'Requested CPU/GPU delegates on retained images; no stable provider identity, gesture engine, capture or physical acceptance.'};
  const median = rows => {const a=rows.map(r=>r.processing_ms).sort((a,b)=>a-b);return a[Math.floor(a.length/2)];};
  const lastScene = manifest.scenes.at(-1);
  const lastFrame = lastScene.frames.at(-1);
  const lastPrediction = results.at(-1).predictions.at(-1);
  publish({status:'complete', phase:'retained', label:'Browser CPU/GPU retained-image comparison',
    frame:lastScene.frames.length-1, total:lastScene.frames.length,
    image:baseUrl+lastScene.name+'/'+lastFrame.image,
    model_gap_m:lastPrediction.hands[0]?.world_pinch_distance_m,
    skin_gap_m:lastScene.reference.frames.at(-1).distal_skin?.surface_gap_m,
    backend:'Tasks Vision1.1.1 / requested CPU and GPU', hands:lastPrediction.hands.length,
    contacts:null, replay_available:false,
    metrics_text:results.map(r=>r.delegate+'/'+r.scene+': '+median(r.predictions).toFixed(1)+'ms').join(' · ')});
  return result;
}
