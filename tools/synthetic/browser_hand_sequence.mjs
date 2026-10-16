// Camera-free optional provider comparison; all assets stay on loopback.
// A browser backend is not stable hand identity or delivered system input.
export async function workerSource(baseUrl, modelBytes, delegate, maxHands=2, presenceFloor=.5) {
  if(!Number.isInteger(maxHands)||maxHands<1||maxHands>2)throw Error('Expected one or two provider hands');
  if(!Number.isFinite(presenceFloor)||presenceFloor<.01||presenceFloor>1)throw Error('Invalid provider presence floor');
  if (location.origin !== new URL(baseUrl).origin) throw Error('Worker mode requires same-origin HTTP hosting');
  const worker = new Worker(new URL('task_hand_worker.js',baseUrl));
  let id = 0;
  const pending = new Map();
  const fail = error => {for (const p of pending.values()) {clearTimeout(p.timeout);p.reject(error);}pending.clear();};
  worker.onerror = event => fail(Error(event.message||'Worker error'));
  worker.onmessage = event => {
    const message = event.data;
    const p = pending.get(message.id);
    if (!p) return;
    pending.delete(message.id);clearTimeout(p.timeout);
    message.error ? p.reject(Error(message.error)) : p.resolve(message);
  };
  const call = (kind, fields={}, transfer=[])=>new Promise((resolve,reject)=>{
    if (pending.size) {reject(Error('Only one worker request may be in flight'));return;}
    const request = ++id;
    const timeout = setTimeout(()=>{
      pending.delete(request);reject(Error('Finite worker request timeout'));
    },30000);
    pending.set(request,{resolve,reject,timeout});
    worker.postMessage({id:request,kind,...fields},transfer);
  });
  try {await call('init',{baseUrl,modelBytes:modelBytes.slice(0),delegate,maxHands,presenceFloor});}
  catch(error) {worker.terminate();throw error;}
  return {
    async infer(image,source_us) {
      const bitmap = image instanceof ImageBitmap ? image : await createImageBitmap(image);
      return call('frame',{image:bitmap,source_us},[bitmap]);
    },
    async close() {
      try {await call('close');}
      finally {worker.terminate();fail(Error('Worker closed'));}
    }
  };
}
export async function runBrowserHandSequence(baseUrl, manifest) {
  if (!/^http:\/\/127\.0\.0\.1:\d+\/$/.test(baseUrl)) throw Error('Expected receiving loopback assets');
  if (manifest.package_version !== '1.1.1') throw Error('Expected pinned Tasks Vision1.1.1');
  const execution = manifest.execution ?? 'main';
  if (!['main','worker'].includes(execution)) throw Error('Invalid execution path');
  const hash = async bytes => [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))]
    .map(v => v.toString(16).padStart(2, '0')).join('');
  const modelBytes = await (await fetch(baseUrl+'hand_landmarker.task')).arrayBuffer();
  if (await hash(modelBytes) !== manifest.model_sha256) throw Error('Model digest differs');
  const {FilesetResolver, HandLandmarker} = await import(baseUrl+'package/vision_bundle.mjs');
  const files = await FilesetResolver.forVisionTasks(baseUrl+'package/wasm');
  const results = [];
  const runId = manifest.run_id ?? crypto.randomUUID();
  if (!/^[0-9a-f-]{36}$/.test(runId)) throw Error('Invalid projection run ID');
  let seq = 0;
  const publish = state => window.receiveRunState?.({...state, runId, seq:++seq});
  for (const delegate of ['CPU', 'GPU']) {
    for (const scene of manifest.scenes) {
      if (scene.frames.length < 9 || scene.frames.length > 121) throw Error('Unbounded scene');
      publish({status:'running', phase:'browser model initialization', label:delegate+' / '+scene.name+' / camera off', frame:0, total:scene.frames.length});
      const initialization = performance.now();
      const canvas = document.createElement('canvas');
      const model = execution === 'worker' ? await workerSource(baseUrl, modelBytes, delegate) : await HandLandmarker.createFromOptions(files, {
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
            let animationFrames = 0;
            let counting = execution === 'worker';
            const heartbeat = ()=>{if(counting){animationFrames++;requestAnimationFrame(heartbeat);}};
            if (counting) requestAnimationFrame(heartbeat);
            let output;
            try {output = execution === 'worker' ? await model.infer(image,frame.source_us) : model.detectForVideo(image, timeMs);}
            finally {counting=false;}
            const elapsed = performance.now();
            const hands = execution === 'worker' ? output.hands : output.landmarks.map((points, i) => {
              const world = output.worldLandmarks[i].map(p => [p.x,p.y,p.z]);
              return {array_index:i, stable_id_available:false,
                landmarks:points.map(p=>[p.x*image.width,p.y*image.height]),
                image_depths:points.map(p=>p.z*image.width),
                world_landmarks:world, handedness:output.handedness[i],
                world_pinch_distance_m:Math.hypot(...world[4].map((v,j)=>v-world[8][j]))};
            });
            const row = {frame:index, source_us:frame.source_us, image:frame.image, hands,
              image_sha256:frame.sha256, decode_hash_fetch_ms:inference-begin,
              inference_ms:elapsed-inference, processing_ms:elapsed-begin, telemetry,
              worker_inference_ms:output.inference_ms??null,
              main_animation_frames_during_inference:animationFrames,
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
      } finally { await model.close(); canvas.remove(); }
      results.push({delegate, execution, scene:scene.name, initialization_ms:initializationMs,
        reference:scene.reference, predictions, camera_opened:false, system_input:false});
      window.wiggleBrowserProgress = {completed:results.length, results};
    }
  }
  const result = {schema:'wigglecontrol.browser-provider-comparison/v1', package_version:manifest.package_version,
    model_sha256:manifest.model_sha256, execution, run_id:runId, results, camera_opened:false, system_input:false,
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
