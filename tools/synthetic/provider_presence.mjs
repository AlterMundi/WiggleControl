// Experimental bridge for the exact inspected Tasks Vision1.1.1 bundle.
// No second inference or SDK fork. Internal names/layout are not a public SDK API.
// CalculatorGraphConfig.output_stream is repeated string field15. See the pinned
// upstream graph sources in docs/research/provider-presence-scores-2026-10.md.
export const BUNDLE_SHA256='a84921e02321e582484c1f8126d344b5f33b791fa09c640a52e20ee8632068f1';

export function appendGraphOutputs(bytes, streams) {
  if(!(bytes instanceof Uint8Array)||bytes.length>16*1024*1024||!streams.length
      ||streams.length>3||new Set(streams).size!==streams.length)throw Error('Invalid graph export');
  const fields=streams.map(stream=>{
    if(typeof stream!=='string'||stream.length>1024||!/^[-a-zA-Z0-9_/.]+$/.test(stream))throw Error('Invalid graph stream');
    const text=new TextEncoder().encode(stream);let size=text.length;const length=[];
    do{const byte=size&127;size>>>=7;length.push(byte|(size?128:0));}while(size);
    return Uint8Array.from([122,...length,...text]);
  });
  const result=new Uint8Array(bytes.length+fields.reduce((n,f)=>n+f.length,0));
  result.set(bytes);let offset=bytes.length;
  for(const field of fields){result.set(field,offset);offset+=field.length;}
  return result;
}

const sameBytes=(a,b)=>a.length===b.length&&a.every((v,i)=>v===b[i]);
const validProto=p=>p instanceof Uint8Array&&p.length>0&&p.length<=4096;

export function normalizePresenceFlags(values) {
  return values.map(value=>{
    if(typeof value==='boolean')return value;
    // This exact compiled bridge emits native bools as numeric0/1.
    if(value===0)return false;
    if(value===1)return true;
    throw Error('Invalid native presence flag');
  });
}

// Missing or ambiguous quality never becomes a higher confidence. Scores follow
// same-frame serialized landmarks, not array order or a previous frame's owner.
export function alignPresenceScores(snapshot, timestamp, handCount, maxHands=2, floor=.5) {
  if(!Number.isFinite(timestamp)||!Number.isInteger(handCount)||handCount<0||handCount>maxHands
      ||![1,2].includes(maxHands)||!Number.isFinite(floor)||floor<.01||floor>1)throw Error('Invalid presence request');
  const missing=reason=>({scores:Array(handCount).fill(null),reason});
  if(handCount===0)return {scores:[],reason:null};
  const {scores,flags,raw,final}=snapshot;
  if(![scores,flags,raw,final].every(s=>s?.timestamp===timestamp))return missing('presence_timestamp');
  if(![scores,flags,raw,final].every(s=>Array.isArray(s.values))
      ||scores.values.length!==flags.values.length||scores.values.length>maxHands
      ||flags.values.some(f=>typeof f!=='boolean')
      ||raw.values.length!==flags.values.filter(Boolean).length||final.values.length!==handCount
      ||[...raw.values,...final.values].some(p=>!validProto(p))
      ||scores.values.some((s,i)=>!Number.isFinite(s)||s<0||s>1||(flags.values[i]&&s<floor))) {
    return missing('presence_shape');
  }
  const admitted=scores.values.filter((_,i)=>flags.values[i]);
  let ambiguous=false;
  const aligned=final.values.map(bytes=>{
    const candidates=raw.values.flatMap((raw,i)=>sameBytes(raw,bytes)?[i]:[]);
    if(candidates.length!==1){ambiguous=true;return null;}
    return admitted[candidates[0]];
  });
  // A second identical final result also cannot inherit one raw hand's score.
  for(let i=0;i<final.values.length;i++)for(let j=0;j<i;j++)if(sameBytes(final.values[i],final.values[j])){
    aligned[i]=aligned[j]=null;ambiguous=true;
  }
  return {scores:aligned,reason:ambiguous?'presence_correspondence':null};
}

const rawMessage=value=>Array.isArray(value)?value:value?.B;
function stream(node, field, tag) {
  const values=node[field]?.filter(s=>typeof s==='string'&&s.startsWith(tag+':'));
  if(values?.length!==1)throw Error('Unexpected pinned graph stream');
  return values[0].split(':').at(-1);
}

export async function installPresenceScores(model, bundleSha256, maxHands=2, floor=.5) {
  if(bundleSha256!==BUNDLE_SHA256||![1,2].includes(maxHands)||!Number.isFinite(floor)||floor<.01||floor>1
      ||typeof model.ka!=='function'||typeof model.h?.setGraph!=='function')throw Error('Unsupported presence backend');
  const graph=model.ka().B;
  if(!Array.isArray(graph?.[0])||!Array.isArray(graph[14]))throw Error('Unsupported graph layout');
  // Avoid deep toJSON: unrelated expanded calculator options trigger a retained
  // serializer assertion. Read only known name/calculator/input/output fields.
  const nodes=graph[0].map(rawMessage);
  const find=name=>{
    const found=nodes.filter(n=>n?.[1]===name);
    if(found.length!==1)throw Error('Ambiguous presence graph');
    return found[0];
  };
  const scoreNode=find('EndLoopFloatCalculator'),flagNode=find('EndLoopBooleanCalculator');
  const rawNode=find('EndLoopNormalizedLandmarkListVectorCalculator');
  const dedup=find('HandLandmarksDeduplicationCalculator');
  if(![flagNode,rawNode].every(n=>stream(n,2,'BATCH_END')===stream(scoreNode,2,'BATCH_END'))
      ||stream(dedup,2,'MULTI_LANDMARKS')!==stream(rawNode,3,'ITERABLE')
      ||stream(dedup,3,'MULTI_LANDMARKS')!=='hand_landmarks')throw Error('Unexpected presence/dedup topology');
  const streams=[stream(scoreNode,3,'ITERABLE'),stream(flagNode,3,'ITERABLE'),stream(rawNode,3,'ITERABLE')];
  if(streams.some(s=>graph[14].includes(s)))throw Error('Presence observer already installed');
  const snapshot={};
  const h=model.h;
  const remember=key=>(values,timestamp)=>{snapshot[key]={values:[...values],timestamp};};
  h.attachFloatVectorListener(streams[0],remember('scores'));
  h.attachBoolVectorListener(streams[1],(values,timestamp)=>{
    snapshot.rawFlags={values:[...values],timestamp};
    try{snapshot.flags={values:normalizePresenceFlags(values),timestamp};}
    catch{snapshot.flags=null;}
  });
  h.attachProtoVectorListener(streams[2],(values,timestamp)=>{
    snapshot.raw={values:values.map(p=>new Uint8Array(p)),timestamp};
  });
  for(const [i,key]of ['scores','flags','raw'].entries())h.attachEmptyPacketListener(streams[i],()=>{snapshot[key]=null;});
  // Extend the ORIGINAL hierarchical graph when the SDK regenerates it. An
  // expanded graph contains cache IDs that cannot survive a fresh graph service.
  const originalSet=h.setGraph;let applied=false;
  h.setGraph=function(bytes,binary){
    if(applied||!binary)throw Error('Presence graph is fixed for this model lifetime');
    applied=true;return originalSet.call(this,appendGraphOutputs(bytes,streams),true);
  };
  await model.setOptions({minHandPresenceConfidence:floor});
  const listeners=h.j?.simpleListeners,empty=h.j?.emptyPacketListeners;
  if(typeof listeners?.hand_landmarks!=='function'||typeof empty?.hand_landmarks!=='function')throw Error('Pinned result listeners unavailable');
  // Keep the SDK's exact parser and timestamp callback. Copy its serialized
  // packets only for correspondence to the pre-deduplication stream.
  const original= listeners.hand_landmarks;
  let pending=[];
  listeners.hand_landmarks=(data,done,timestamp)=>{
    if(done){snapshot.final={values:pending,timestamp};pending=[];}
    else pending.push(new Uint8Array(data));
    original(data,done,timestamp);
  };
  const originalEmpty=empty.hand_landmarks;
  empty.hand_landmarks=timestamp=>{snapshot.final={values:[],timestamp};pending=[];originalEmpty(timestamp);};
  return {
    sources:streams,
    read(timestamp,handCount){return alignPresenceScores(snapshot,timestamp,handCount,maxHands,floor);},
    diagnostics(){return {scores:snapshot.scores,flags:snapshot.flags,native_flags:snapshot.rawFlags,raw_count:snapshot.raw?.values.length,
      final_count:snapshot.final?.values.length,raw_timestamp:snapshot.raw?.timestamp,final_timestamp:snapshot.final?.timestamp};}
  };
}
