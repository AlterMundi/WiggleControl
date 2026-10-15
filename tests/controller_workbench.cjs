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
vm.runInNewContext(source, {window, document:{getElementById:element,
  createElement:()=>({remove(){}}),head:{append:s=>scripts.push(s.src)}},
  Date, setInterval:()=>1, clearInterval(){}});
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
console.log('Controller workbench known-crop/temporal evidence checks passed');
