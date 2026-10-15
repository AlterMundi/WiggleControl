// Camera-free checks of the actual page's countdown and terminal presentation.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const elements = new Map();
const element = id => {
  if (!elements.has(id)) elements.set(id, {textContent: '', className: '', disabled: false, value:'auto', replaceChildren:()=>{}});
  return elements.get(id);
};
let now = 1000, tick, clears = 0;
const canvas = {width:640, height:480, classList:{add:()=>{}}, getContext:()=>({clearRect:()=>clears++})};
const context = vm.createContext({document:{querySelector:()=>canvas,getElementById:element},
  Option:function(text,value){this.text=text;this.value=value}, performance:{now:()=>now}, setInterval:fn=>tick=fn, setTimeout:()=>{},
  AbortSignal:{timeout:()=>null}, fetch:async()=>({status:204})});
const script = fs.readFileSync('tools/preview.html', 'utf8').split('<script>')[1].split('</script>')[0];
vm.runInContext(script, context);
vm.runInContext(`showController({controller:{mode:'pointer',running:true,button_down:true,contacts:[],primary_hand:1,claimed_hand:null,pointer_ready:true},perception:[],input_available:true},true)`, context);
assert.equal(element('held-input').textContent,'Button held');
vm.runInContext(`showController({controller:{mode:'pointer',running:true,button_down:true,contacts:[],primary_hand:1,claimed_hand:null,pointer_ready:true},perception:[],input_available:true},false)`, context);
assert.equal(element('held-input').textContent,'View delayed');
vm.runInContext(`showController({controller:{mode:'two_contacts',running:true,button_down:false,contacts:[],awaiting_open_hands:[],primary_hand:null,claimed_hand:null},perception:[],perception_ambiguity_reason:'pair_overlap',input_available:false},true)`,context);
assert.match(element('guide').textContent,/overlap.*Separate and open/);
vm.runInContext(`showController({controller:{mode:'two_contacts',running:true,button_down:false,contacts:[],awaiting_open_hands:[3,4],primary_hand:null,claimed_hand:null},perception:[{id:3},{id:4}],input_available:false},true)`,context);
assert.match(element('guide').textContent,/Open your hands before pinching again/);
vm.runInContext(`showController({controller:{mode:'two_contacts',running:true,button_down:false,contacts:[],awaiting_open_hands:[],primary_hand:null,claimed_hand:null},perception:[{id:3},{id:4}],input_available:false},true)`,context);
assert.match(element('guide').textContent,/Pinch each visible hand/);
vm.runInContext('deadline=2500; stopped=false;', context);
element('test-indicator').className='testing'; element('presses').textContent='4';
tick(); assert.equal(element('countdown').textContent,'00:02');
now=2000; tick(); assert.equal(element('countdown').textContent,'00:01');
now=2500; tick(); assert.equal(element('countdown').textContent,'00:00');
assert.equal(element('test-label').textContent,'TEST ENDED');
assert.equal(element('test-indicator').className,'');
assert.equal(element('presses').textContent,'4');
assert.equal(element('held-input').textContent,'Feedback ended');assert.equal(element('stop').disabled,true); assert.equal(clears,1);
console.log('preview timer: passed');
