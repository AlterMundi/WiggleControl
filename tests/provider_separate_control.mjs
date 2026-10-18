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
  if(path==='/provider/config')return {ok:true,json:async()=>({run_id:'run',active,prepared,camera_opened:prepared,system_input:prepared})};
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
