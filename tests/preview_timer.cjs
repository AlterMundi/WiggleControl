// Camera-free checks of the actual page's countdown and terminal presentation.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const elements = new Map();
const element = id => {
  if (!elements.has(id)) elements.set(id, {textContent: '', className: '', disabled: false});
  return elements.get(id);
};
let now = 1000, tick, clears = 0;
const canvas = {width:640, height:480, getContext:()=>({clearRect:()=>clears++})};
const context = vm.createContext({document:{querySelector:()=>canvas,getElementById:element},
  performance:{now:()=>now}, setInterval:fn=>tick=fn, setTimeout:()=>{},
  AbortSignal:{timeout:()=>null}, fetch:async()=>({status:204})});
const script = fs.readFileSync('tools/preview.html', 'utf8').split('<script>')[1].split('</script>')[0];
vm.runInContext(script, context);
vm.runInContext('deadline=2500; stopped=false;', context);
element('test-indicator').className='testing'; element('presses').textContent='4';
tick(); assert.equal(element('countdown').textContent,'00:02');
now=2000; tick(); assert.equal(element('countdown').textContent,'00:01');
now=2500; tick(); assert.equal(element('countdown').textContent,'00:00');
assert.equal(element('test-label').textContent,'TEST ENDED');
assert.equal(element('test-indicator').className,'');
assert.equal(element('presses').textContent,'4');
assert.equal(element('stop').disabled,true); assert.equal(clears,1);
console.log('preview timer: passed');
