import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const source=fs.readFileSync('tools/provider_calibration.html','utf8').split('<script>')[1].split('</script>')[0].replace(/poll\(\);\s*$/,'');
const elements=new Map(),requests=[],revoked=[],timers=new Map();let next=0,frameCount=0,blobResolve;
const el=id=>{if(!elements.has(id))elements.set(id,{hidden:false,style:{setProperty(k,v){this[k]=v}},attrs:{},textContent:'',removeAttribute(k){delete this[k]},setAttribute(k,v){this.attrs[k]=v},toggleAttribute(k,v){if(v)this.attrs[k]='';else delete this.attrs[k]}});return elements.get(id)};
const calibration={phase:'idle',guide:null,review:{corner_target:null},target:null,has_frame:false,camera_closed:true,saved:false};
const config={width:1920,height:1080,run_id:'camera-test',active:true,prepared:false,camera_opened:false,system_input:false,calibration};
let frame={provider_run_id:'camera-test',camera_opened:true,synthetic_frame:false,fresh:true,age_ms:60,freshness_ms:300};
const context=vm.createContext({document:{getElementById:el,body:{classList:{toggle(){}}},documentElement:{}},AbortSignal:{timeout:()=>null},performance:{now:()=>0},innerWidth:1366,innerHeight:712,screen:{width:1366,height:768},requestAnimationFrame:f=>f(),
 URL:{createObjectURL(){return 'blob:frame-'+(++frameCount)},revokeObjectURL:u=>revoked.push(u)},
 setTimeout:fn=>{const id=++next;timers.set(id,fn);return id},clearTimeout:id=>timers.delete(id),
 fetch:async(path,options)=>{
  requests.push(path);
  if(path==='/provider/config')return {ok:true,json:async()=>structuredClone(config)};
  if(path==='/frame')return {ok:true,status:200,headers:{get:()=>JSON.stringify(frame)},blob:async()=>blobResolve?new Promise(r=>{blobResolve.resolve=r}):{}};
  return {ok:true};
 }});
vm.runInContext(source,context);
await vm.runInContext('poll()',context);assert(!requests.includes('/frame'),'Idle cannot acquire preview frames');
assert.equal(el('camera-panel').style['--camera-aspect'],'1920/1080');assert.match(el('camera-label').textContent,/1920 × 1080/);
config.prepared=true;calibration.phase='waiting';calibration.camera_closed=false;calibration.guide={phase:'waiting',step:0};
await vm.runInContext('poll()',context);assert(!requests.includes('/frame'),'Preparation cannot show a warmup image as a camera frame');
config.camera_opened=true;calibration.has_frame=true;
await vm.runInContext('poll()',context);await new Promise(setImmediate);
assert.equal(el('camera-image').hidden,false);assert.equal(el('camera-image').src,'blob:frame-1');
assert(!requests.includes('/provider/ready'),'Preview cannot start the detector or camera');
config.camera_opened=false;calibration.camera_closed=true;
await vm.runInContext('poll()',context);assert.equal(el('camera-image').hidden,true);assert(!('src' in el('camera-image')));assert.deepEqual(revoked,['blob:frame-1']);
// A response that arrives after closure must not restore its private image.
config.camera_opened=true;calibration.camera_closed=false;blobResolve={};
await vm.runInContext('poll()',context);await new Promise(setImmediate);
config.camera_opened=false;calibration.camera_closed=true;
await vm.runInContext('poll()',context);blobResolve.resolve({});blobResolve=null;await new Promise(setImmediate);
assert.equal(el('camera-image').hidden,true);assert.equal(frameCount,1);
// Reject a previous run, a synthetic reference, or an aged camera snapshot.
for(const change of [{provider_run_id:'old-run'},{synthetic_frame:true},{fresh:false},{age_ms:300}]){
 frame={provider_run_id:'camera-test',camera_opened:true,synthetic_frame:false,fresh:true,age_ms:60,freshness_ms:300,...change};
 config.camera_opened=true;calibration.camera_closed=false;
 await vm.runInContext('poll()',context);await new Promise(setImmediate);assert.equal(el('camera-image').hidden,true);
 config.camera_opened=false;calibration.camera_closed=true;await vm.runInContext('poll()',context);
}
calibration.guide={phase:'waiting',step:2};calibration.phase='waiting';calibration.camera_closed=false;
const before=requests.filter(p=>p==='/provider/calibration/step').length;
await vm.runInContext('poll()',context);
assert.equal(requests.filter(p=>p==='/provider/calibration/step').length,before,'Corner review waits for actual fullscreen');
context.document.fullscreenElement={};
await vm.runInContext('poll()',context);
assert.equal(requests.filter(p=>p==='/provider/calibration/step').length,before+1);
assert(requests.every(p=>['/provider/config','/provider/calibration/step','/frame'].includes(p)),'Showing feedback never requests capture or injection');
console.log('Calibration camera: observed-only preview, closure/late-frame cleanup, run/freshness gates and fullscreen corner acknowledgment');
