import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const source=fs.readFileSync('tools/synthetic/provider_fixture_client.mjs','utf8')
 .replace("import {workerSource} from '/provider/worker-tools.mjs';",'');
const script=source.slice(0,source.indexOf('void(async()=>'))+'globalThis.runForTest=run;';
{
 const context=vm.createContext({location:{origin:'http://127.0.0.1:8876'},window:{wiggleProviderTrace:[],wiggleProviderTraceDiscarded:0}});
 vm.runInContext(script,context);
 vm.runInContext('for(let sequence=0;sequence<12000;sequence++)appendTiming({sequence});',context);
 const trace=context.window.wiggleProviderTrace;
 assert(trace.length<=1024&&trace.length>=896);
 assert.equal(trace.length+context.window.wiggleProviderTraceDiscarded,12000);
 assert.equal(trace.at(-1).sequence,11999);
 assert.equal(trace[0].sequence,context.window.wiggleProviderTraceDiscarded);
}
for(const changedRun of [false,true]){
 const calls=[],timers=[];let prepared=false;
 const context=vm.createContext({
  location:{origin:'http://127.0.0.1:8876'},
  document:{documentElement:{dataset:{providerWorker:'external-start'}},querySelector:()=>({textContent:''}),getElementById(){throw Error('Detached worker must not depend on a control button');}},
  AbortSignal:{timeout:()=>null},setTimeout:fn=>timers.push(fn),
  fetch:async path=>{
   calls.push(path);
   if(path==='/provider/config')return {ok:true,json:async()=>({run_id:prepared&&changedRun?'other':'run',active:true,prepared,manual_start:true})};
   throw Error('model boundary');
  },
 });
 vm.runInContext(script,context);const pending=context.runForTest();await new Promise(setImmediate);
 assert.deepEqual(calls,['/provider/config','/provider/config']);
 assert.equal(timers.length,1);prepared=true;timers.shift()();
 await assert.rejects(pending,changedRun?/ended before Start/:/model boundary/);
 assert.equal(calls.includes('/provider/prepare'),false,'Only the human control page requests Start');
 assert.equal(calls.includes('/hand_landmarker.task'),!changedRun);
 assert.equal(calls.includes('/frame'),false,'Detector never polls feedback frames');
}

const elements=new Map(),calls=[],timers=[];let prepared=false,active=true;
const element=id=>{if(!elements.has(id))elements.set(id,{textContent:'',disabled:true});return elements.get(id)};
const context=vm.createContext({document:{getElementById:element},AbortSignal:{timeout:()=>null},setTimeout:fn=>timers.push(fn),
 fetch:async(path,options)=>{
  calls.push({path,options});
  if(path==='/provider/config')return {ok:true,json:async()=>({run_id:'run',active,prepared,camera_opened:prepared&&active,system_input:prepared&&active})};
  assert.equal(JSON.parse(options.body).run_id,'run');
  if(path==='/provider/prepare')prepared=true;
  else if(path==='/provider/stop')active=false;
  else throw Error('Unexpected request');
  return {ok:true};
 },
});
const control=fs.readFileSync('tools/provider_control.html','utf8');
vm.runInContext(control.split('<script>')[1].split('</script>')[0],context);
await new Promise(setImmediate);assert.equal(element('start').disabled,false);
assert.equal(calls.length,1,'Opening controls does not request preparation');
await element('start').onclick();await vm.runInContext('poll()',context);
assert.match(element('state').textContent,/Camera active.*Mouse enabled/);
await element('stop').onclick();await vm.runInContext('poll()',context);
assert.match(element('state').textContent,/Session ended.*Camera off/);
assert.equal(element('start').disabled,true);
assert(calls.every(c=>!c.path.includes('/frame')),'Control page never retrieves camera/feedback images');
console.log('Detached detector waits for explicit Start; controls never load frames/models and preserve Stop/run binding');

// A task can prepare its complete UI before exposing manual Start. All mode and
// countdown feedback comes from the cheap config endpoint, without image demand.
{
 const elements=new Map(),calls=[];
 const element=id=>{if(!elements.has(id))elements.set(id,{textContent:'',disabled:true});return elements.get(id)};
 const config={run_id:'detail',active:true,prepared:false,camera_opened:false,system_input:false,
  trial_seconds:90,remaining_ms:null,detail_configured:true,detail_active:false,right_click:true,mode:null};
 const context=vm.createContext({document:{getElementById:element},location:{hash:'#setup'},
  AbortSignal:{timeout:()=>null},setTimeout:()=>{},fetch:async(path,options)=>{
   calls.push(path);
   if(path==='/provider/config')return {ok:true,json:async()=>({...config})};
   assert.equal(JSON.parse(options.body).run_id,'detail');
   if(path==='/provider/prepare')config.prepared=true;
   else if(path==='/provider/stop')config.active=false;
   else throw Error('Unexpected acquisition or feedback request');
   return {ok:true};
  }});
 vm.runInContext(control.split('<script>')[1].split('</script>')[0],context);
 await new Promise(setImmediate);
 assert.equal(element('start').disabled,true);
 assert.match(element('operation').textContent,/Start locked/);
 await element('start').onclick();assert.equal(calls.includes('/provider/prepare'),false);
 vm.runInContext('wiggleControlSetupReady=true',context);await vm.runInContext('poll()',context);
 assert.equal(element('start').disabled,false);
 await element('start').onclick();await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/Clock not started/);
 assert.match(element('gesture-help').textContent,/middle pinch: right/);
 assert.match(element('gesture-help').textContent,/ring and little/);
 config.escape_configured=true;await vm.runInContext('poll()',context);
 assert.match(element('gesture-help').textContent,/thumb out for Escape/);
 config.system_input=true;config.remaining_ms=89_800;
 await vm.runInContext('poll()',context);
 assert.match(element('state').textContent,/Opening camera.*Mouse enabled/);
 assert.match(element('operation').textContent,/Opening camera.*90 seconds/);
 config.camera_opened=true;config.mode='pointer';config.remaining_ms=87_123;
 await vm.runInContext('poll()',context);assert.match(element('operation').textContent,/SCREEN.*88 seconds/);
 config.detail_active=true;await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/DETAIL.*88 seconds/);
 config.escape_feedback={candidate:'thumb_out',armed:true,hold_elapsed_us:350000,hold_required_us:700000};
 await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/ESC hold 50%/);
 config.escape_feedback={candidate:null,armed:false};await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/Escape: open hand/);
 assert.match(element('lifetime-help').textContent,/90 seconds of acquisition/);
 config.remaining_ms=null;await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/Waiting for acquisition clock/);
 assert.doesNotMatch(element('operation').textContent,/Until Stop/);
 config.trial_seconds=null;await vm.runInContext('poll()',context);
 assert.match(element('operation').textContent,/DETAIL.*Until Stop/);
 config.active=false;config.camera_opened=true;config.system_input=null;
 await vm.runInContext('poll()',context);
 assert.match(element('state').textContent,/Session ending.*Camera active.*Mouse state unknown/);
 assert.match(element('operation').textContent,/Waiting for camera\/input closure/);
 assert.equal(element('start').disabled,true);assert.equal(element('stop').disabled,false);
 config.camera_opened=false;config.system_input=false;
 await vm.runInContext('poll()',context);assert.equal(element('operation').textContent,'Session ended');
 assert.equal(element('start').disabled,true);
 assert(calls.every(path=>['/provider/config','/provider/prepare','/provider/stop'].includes(path)));
}
console.log('Finite countdown/detail status stay image-free; task setup cannot start acquisition');
