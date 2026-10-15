// Finite synthetic source: the Rust host owns acquisition time and frame leases.
import {workerSource} from '/provider/worker-tools.mjs';
const base = location.origin+'/';
const text = (id,value)=>{const e=document.getElementById(id);if(e)e.textContent=value;};
document.querySelector('h1').textContent='WIGGLECONTROL // REAL-CLOCK FIXTURE';
let model,runId;
const post=(path,fields={})=>fetch(path,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({run_id:runId,...fields}),signal:AbortSignal.timeout(2000)});
const run = async()=>{
  const config = await (await fetch('/provider/config')).json();
  runId=config.run_id;
  const bytes = await (await fetch('/hand_landmarker.task')).arrayBuffer();
  const hash = [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))]
    .map(v=>v.toString(16).padStart(2,'0')).join('');
  if(hash!==config.model_sha256)throw Error('Provider model digest differs');
  text('capture-status','Preparing GPU worker · synthetic frames only');
  model=await workerSource(base,bytes,config.delegate,config.max_hands);
  // Compile/warm the graph on the displayed public reference before real-clock
  // acquisition begins. Warmup points never enter Rust meaning.
  const warmResponse=await fetch('/frame',{cache:'no-store'});
  const warmState=JSON.parse(warmResponse.headers.get('X-Wiggle-State'));
  if(!warmState.ui_fixture)throw Error('Warmup requires an explicit synthetic reference');
  const warmBlob=await warmResponse.blob();const warmStart=performance.now();
  for(let i=0;i<3;i++){
    const image=await createImageBitmap(warmBlob);
    try{await model.infer(image,i*1000);}finally{image.close();}
  }
  const warmup_ms=performance.now()-warmStart;
  window.wiggleProviderTrace=[];
  const ready=await post('/provider/ready');
  if(!ready.ok)throw Error('Worker readiness was rejected');
  let received=0,dropped=0;
  while(true){
    const began=performance.now();
    const response=await fetch('/provider/frame',{cache:'no-store',signal:AbortSignal.timeout(2000)});
    if(response.status===410)break;
    if(response.status===204){await new Promise(r=>setTimeout(r,10));continue;}
    if(!response.ok)throw Error('Frame lease rejected: '+await response.text());
    const ticket=JSON.parse(response.headers.get('X-Wiggle-Source'));
    if(ticket.run_id!==runId)throw Error('Source run changed');
    const blob=await response.blob();const fetched=performance.now();
    const image=await createImageBitmap(blob);const decoded=performance.now();
    let inferred;
    try{inferred=await model.infer(image,ticket.source_us);}finally{image.close();}
    const completed=performance.now();
    const hands=inferred.hands.map(h=>{
      if(h.handedness.length!==1)throw Error('Unsupported handedness categories');
      const c=h.handedness[0];
      if(!['Left','Right'].includes(c.categoryName))throw Error('Unknown handedness category');
      return {landmarks:h.landmarks,world_landmarks:h.world_landmarks,
        right_probability:c.categoryName==='Right'?c.score:1-c.score};
    });
    const returned=await post('/provider/result',{sequence:ticket.sequence,hands});
    window.wiggleProviderTrace.push({sequence:ticket.sequence,fetch_ms:fetched-began,
      decode_ms:decoded-fetched,worker_roundtrip_ms:completed-decoded,
      worker_inference_ms:inferred.inference_ms,post_ms:performance.now()-completed,
      status:returned.status});
    if(returned.status===410){dropped++;continue;}
    if(!returned.ok)throw Error('Meaning receipt rejected: '+await returned.text());
    const receipt=await returned.json();received++;
    window.wiggleProviderResult={received,dropped,warmup_ms,last:receipt,state:'running'};
  }
  window.wiggleProviderResult={...window.wiggleProviderResult,received,dropped,state:'complete'};
};
void(async()=>{
  try{await run();}
  catch(error){window.wiggleProviderResult={state:'error',error:String(error)};
    text('capture-status','Synthetic provider error · '+error.message);
    try{await post('/provider/stop');}catch{}
  }finally{
    let acknowledged=false;
    try{if(model){await model.close();acknowledged=true;}}
    catch(error){window.wiggleProviderResult={...window.wiggleProviderResult,close_error:String(error)};}
    finally{
      try{await post('/provider/closed',{acknowledged});}catch{}
    }
  }
})();
