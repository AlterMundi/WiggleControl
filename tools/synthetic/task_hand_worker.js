// Classic worker: pinned Tasks' Wasm loader uses importScripts/global ModuleFactory.
// Parent owns acquisition/time and one in-flight request; no camera or input here.
let model = null;
let last = -1;
let accepting = true;
self.onmessage = async event => {
  const {id, kind} = event.data;
  try {
    if (kind === 'init') {
      const {baseUrl, modelBytes, delegate, maxHands=2, presenceFloor=.5} = event.data;
      if (model || !accepting || !/^http:\/\/127\.0\.0\.1:\d+\/$/.test(baseUrl)
          || !['CPU','GPU'].includes(delegate) || !Number.isInteger(maxHands)
          || maxHands<1 || maxHands>2 || !Number.isFinite(presenceFloor)
          || presenceFloor<.01 || presenceFloor>1) throw Error('Invalid worker initialization');
      const {FilesetResolver, HandLandmarker} = await import(baseUrl+'package/vision_bundle.mjs');
      const files = await FilesetResolver.forVisionTasks(baseUrl+'package/wasm');
      model = await HandLandmarker.createFromOptions(files, {
        baseOptions:{modelAssetBuffer:new Uint8Array(modelBytes), delegate},
        canvas:new OffscreenCanvas(1,1), runningMode:'VIDEO', numHands:maxHands,
        minHandDetectionConfidence:.5, minHandPresenceConfidence:presenceFloor, minTrackingConfidence:.5
      });
      self.postMessage({id, ok:true});
    } else if (kind === 'frame') {
      const {image, source_us} = event.data;
      try {
        const stamp = Math.floor(source_us/1000);
        if (!model || !accepting || !Number.isSafeInteger(source_us) || stamp <= last
            || image.width > 4096 || image.height > 4096) throw Error('Invalid worker frame');
        last = stamp;
        const begin = performance.now();
        const output = model.detectForVideo(image, stamp);
        const inference_ms = performance.now()-begin;
        const hands = output.landmarks.map((points,i)=>{
          const world = output.worldLandmarks[i].map(p=>[p.x,p.y,p.z]);
          return {array_index:i, stable_id_available:false,
            landmarks:points.map(p=>[p.x*image.width,p.y*image.height]),
            image_depths:points.map(p=>p.z*image.width),
            world_landmarks:world, handedness:output.handedness[i],
            world_pinch_distance_m:Math.hypot(...world[4].map((v,j)=>v-world[8][j]))};
        });
        self.postMessage({id, hands, inference_ms});
      } finally { image.close(); }
    } else if (kind === 'close') {
      accepting = false;
      model?.close();model = null;
      self.postMessage({id, ok:true});self.close();
    } else {throw Error('Unknown worker request');}
  } catch(error) {
    self.postMessage({id, error:String(error), stack:error.stack});
  }
};
