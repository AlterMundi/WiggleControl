// Finite synthetic source: the Rust host owns acquisition time and frame leases.
import {workerSource} from '/provider/worker-tools.mjs';
const base = location.origin+'/';
const text = (id,value)=>{const e=document.getElementById(id);if(e)e.textContent=value;};
document.querySelector('h1').textContent='WIGGLECONTROL // REAL-CLOCK FIXTURE';
let model;
const run = async()=>{
  const config = await (await fetch('/provider/config')).json();
  const bytes = await (await fetch('/hand_landmarker.task')).arrayBuffer();
  const hash = [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))]
    .map(v=>v.toString(16).padStart(2,'0')).join('');
  if(hash!==config.model_sha256)throw Error('Provider model digest differs');
  text('capture-status','Preparing GPU worker · synthetic frames only');
  model=await workerSource(base,bytes,config.delegate);
  const ready=await fetch('/provider/ready',{method:'POST'});
  if(!ready.ok)throw Error('Worker readiness was rejected');
  let received=0,dropped=0;
  while(true){
    const response=await fetch('/provider/frame',{cache:'no-store',signal:AbortSignal.timeout(2000)});
    if(response.status===410)break;
    if(response.status===204){await new Promise(r=>setTimeout(r,10));continue;}
    if(!response.ok)throw Error('Frame lease rejected: '+await response.text());
    const ticket=JSON.parse(response.headers.get('X-Wiggle-Source'));
    const image=await createImageBitmap(await response.blob());
    let inferred;
    try{inferred=await model.infer(image,ticket.source_us);}finally{image.close();}
    const hands=inferred.hands.map(h=>{
      if(h.handedness.length!==1)throw Error('Unsupported handedness categories');
      const c=h.handedness[0];
      if(!['Left','Right'].includes(c.categoryName))throw Error('Unknown handedness category');
      return {landmarks:h.landmarks,world_landmarks:h.world_landmarks,
        right_probability:c.categoryName==='Right'?c.score:1-c.score};
    });
    const returned=await fetch('/provider/result',{method:'POST',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({sequence:ticket.sequence,hands}),signal:AbortSignal.timeout(2000)});
    if(returned.status===410){dropped++;continue;}
    if(!returned.ok)throw Error('Meaning receipt rejected: '+await returned.text());
    const receipt=await returned.json();received++;
    window.wiggleProviderResult={received,dropped,last:receipt,state:'running'};
  }
  window.wiggleProviderResult={...window.wiggleProviderResult,received,dropped,state:'complete'};
};
void(async()=>{
  try{await run();}
  catch(error){window.wiggleProviderResult={state:'error',error:String(error)};
    text('capture-status','Synthetic provider error · '+error.message);
    try{await fetch('/stop',{method:'POST'});}catch{}
  }finally{
    let acknowledged=false;
    try{if(model){await model.close();acknowledged=true;}}
    catch(error){window.wiggleProviderResult={...window.wiggleProviderResult,close_error:String(error)};}
    finally{
      try{await fetch('/provider/closed',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({acknowledged})});}catch{}
    }
  }
})();
