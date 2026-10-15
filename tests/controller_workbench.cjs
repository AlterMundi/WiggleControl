const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync('tools/synthetic/controller.html', 'utf8').match(/<script>([\s\S]*?)<\/script>/)[1];
const nodes = new Map(), scripts = [];
const element = id => {
  if (!nodes.has(id)) nodes.set(id, {textContent:'',hidden:false,disabled:false,remove(){}});
  return nodes.get(id);
};
const window = {addEventListener(){}};
let now = 0, nextTimer = 0;
const timers = new Map();
const advance = () => {
  const [id, timer] = timers.entries().next().value;
  timers.delete(id); now += timer.delay; timer.callback();
};
vm.runInNewContext(source, {window, document:{getElementById:element,
  createElement:()=>({remove(){}}),head:{append:s=>scripts.push(s.src)}},
  Date, performance:{now:()=>now}, setInterval:()=>1,
  setTimeout(callback,delay){timers.set(++nextTimer,{callback,delay});return nextTimer},
  clearTimeout(id){timers.delete(id)}});
window.receiveRunState({runId:'known',seq:1,status:'complete',known_crop:true,
  metrics_text:'16 reconstructions; detection and gesture output unknown',replay_available:false,
  frame:7,total:8,skin_gap_m:.002,model_gap_m:.025,contacts:null,image:'near/fixture.png'});
assert.match(element('frameText').textContent,/known-crop reconstruction/);
assert.doesNotMatch(element('frameText').textContent,/detected hand/);
assert.match(element('frameText').textContent,/unreported contacts/);
assert.match(element('limitation').textContent,/No detector/);
assert.equal(element('replay').disabled,true);
assert.match(element('replayState').textContent,/Independent crops/);
assert.equal(scripts.length,0,'Independent static samples cannot replay an older temporal result');
assert.match(element('metrics').textContent,/detection and gesture output unknown/);
window.receiveRunState({runId:'temporal',seq:1,status:'complete',frame:40,total:41,
  hands:1,detectedFrames:41,processingMedianMs:30,ambiguousFrames:0,identityCount:1});
assert.equal(element('replay').disabled,false);
assert.match(element('frameText').textContent,/1 detected hand/);
assert.match(element('metrics').textContent,/41\/41 frames with detections/);
assert.match(scripts[0],/^sequence-data.js/);
window.receiveSequenceResults({frames:[
  {frame:0,hands:1,elapsed_ms:0,timing_scope:'host-clock'},
  {frame:1,hands:1,elapsed_ms:85,timing_scope:'host-clock'},
  {frame:2,hands:1,elapsed_ms:1085,timing_scope:'host-clock'}]});
element('replay').onclick();
advance();
assert.match(element('limitation').textContent,/Measured host-clock/);
assert.equal(timers.values().next().value.delay,85);
advance();
assert.equal(timers.values().next().value.delay,1000,'Retain a measured stall instead of playing at20Hz');
advance();
assert.equal(now,1085);
assert.match(element('replayState').textContent,/Replay complete/);
assert.equal(timers.size,0);
console.log('Controller workbench evidence and measured replay timing checks passed');
