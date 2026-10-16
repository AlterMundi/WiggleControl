import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const calls=[];
let clicked;
const start={hidden:true,disabled:false,addEventListener(type,callback){assert.equal(type,'click');clicked=callback;}};
const context=vm.createContext({
  location:{origin:'http://127.0.0.1:8875'},
  document:{querySelector:()=>({textContent:''}),getElementById:id=>id==='start-beta'?start:{textContent:''}},
  AbortSignal:{timeout:()=>null},
  fetch:async path=>{
    calls.push(path);
    if(path==='/provider/config')return {json:async()=>({run_id:'beta',absolute_pointer:true,manual_start:true})};
    if(path==='/provider/prepare')return {ok:true};
    throw Error('asset boundary reached');
  },
});
let script=fs.readFileSync('tools/synthetic/provider_fixture_client.mjs','utf8');
script=script.replace("import {workerSource} from '/provider/worker-tools.mjs';",'');
script=script.slice(0,script.indexOf('void(async()=>'))+'globalThis.runForTest=run;';
vm.runInContext(script,context);
const pending=context.runForTest();
await new Promise(setImmediate);
assert.equal(start.hidden,false);
assert.deepEqual(calls,['/provider/config']);
assert.equal(typeof clicked,'function');
clicked();
await assert.rejects(pending,/asset boundary reached/);
assert.deepEqual(calls,['/provider/config','/provider/prepare','/hand_landmarker.task']);
assert.equal(start.disabled,true);
console.log('Manual beta startup stays before model/camera preparation until an explicit click');
