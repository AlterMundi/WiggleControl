// Execute the real page against transport faults, Stop receipts and slow decode.
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const elements=new Map(),scheduled=[];let now=1000,ready=0;
const element=id=>{if(!elements.has(id))elements.set(id,{textContent:'',className:'',disabled:false,value:'auto',replaceChildren(){}});return elements.get(id)};
const drawing={clearRect(){},save(){},restore(){},translate(){},scale(){},drawImage(){}};
const canvas={width:640,height:480,classList:{add(){}},getContext:()=>drawing};
const context=vm.createContext({document:{querySelector:()=>canvas,getElementById:element},
 Option:function(text,value){this.text=text;this.value=value},performance:{now:()=>now},setInterval(){},setTimeout:(fn,ms)=>scheduled.push(ms),
 requestAnimationFrame:fn=>fn(),AbortSignal:{timeout:()=>null},fetch:async()=>({status:204}),
 createImageBitmap:async()=>({close(){}})});
vm.runInContext(fs.readFileSync('tools/preview.html','utf8').split('<script>')[1].split('</script>')[0],context);
(async()=>{
 await new Promise(setImmediate);
 context.fetch=async()=>{throw Error('temporary transport loss')};
 await vm.runInContext('poll()',context);
 assert.equal(vm.runInContext('stopped',context),false);
 assert.equal(element('stop').disabled,false);
 assert.equal(element('held-input').textContent,'View delayed');
 assert.match(element('capture-status').textContent,/unconfirmed/);
 assert.equal(scheduled.at(-1),500);
 await element('stop').onclick();
 assert.equal(vm.runInContext('stopped',context),false);
 assert.match(element('status').textContent,/not acknowledged/);
 const state={sequence:1,ui_fixture:true,width:640,height:480,age_ms:0,fresh:true,freshness_ms:300,
  landmarks:null,confidence:null,eligible:false,pinch_ratio:null,pinch_state:'touch',presses:0,
  decode_ms:0,inference_ms:0,confirmation_elapsed_ms:0,confirmation_required_ms:40,close_ratio:.25,open_ratio:.4,
  input_available:false,perception:[{id:4}],controller:{mode:'two_contacts',running:true,contacts:[],button_down:false,
  awaiting_open_hands:[4],pointer_ready:false,primary_hand:null,claimed_hand:null}};
 context.fetch=async path=>path==='/frame'?{status:200,ok:true,headers:{get:()=>JSON.stringify(state)},blob:async()=>({})}:(ready++,{ok:true});
 await vm.runInContext('poll()',context);
 assert.match(element('guide').textContent,/Open your hands/);
 assert.equal(vm.runInContext('stopped',context),false);
 assert.match(element('status').textContent,/not acknowledged/);
 assert.equal(ready,1);
 state.visual_macros=true;
 state.controller.macro_feedback={enabled:true,armed:true,candidate:'victory',
  hold_elapsed_us:250000,hold_required_us:500000,neutral_elapsed_us:0,neutral_required_us:200000,
  cooldown_remaining_us:0,blocked_reason:null};
 vm.runInContext('macroFeedback('+JSON.stringify(state)+',true)',context);
 assert.match(element('overlay-note').textContent,/victory.*50%/);
 state.controller.macro_feedback.candidate=null;
 state.controller.macro_feedback.blocked_reason='input_busy';
 vm.runInContext('macroFeedback('+JSON.stringify(state)+',true)',context);
 assert.match(element('overlay-note').textContent,/Pinch has priority/);
 state.controller.last_macro_action={action:'toggle_precision',handled:true,source_us:1,mode:'precision',running:true};
 state.controller.mode='scroll';
 vm.runInContext('macroFeedback('+JSON.stringify(state)+',true)',context);
 assert.match(element('overlay-note').textContent,/applied.*Precision/);
 state.controller.last_macro_action={action:'play_pause',handled:false,source_us:1};
 vm.runInContext('macroFeedback('+JSON.stringify(state)+',true)',context);
 assert.match(element('overlay-note').textContent,/unavailable.*play pause/);
 state.controller.last_macro_action=null;
 state.controller.macro_feedback.blocked_reason='missing_pose';
 state.visual_pose={hand_id:4,pose_evidence_available:false,prediction:{category:'Open_Palm',confidence:.62}};
 vm.runInContext('macroFeedback('+JSON.stringify(state)+',true)',context);
 assert.match(element('overlay-note').textContent,/unclear.*62%/);
 vm.runInContext('frameAcknowledged=false',context);ready=0;state.sequence=2;
 context.createImageBitmap=async()=>{now+=400;return{close(){}}};
 await vm.runInContext('poll()',context);
 assert.equal(ready,0,'late image must not start a measured trial');
 assert.equal(element('held-input').textContent,'View delayed');
 assert.match(element('overlay-note').textContent,/delayed/);
 context.fetch=async()=>({ok:true});
 await element('stop').onclick();
 assert.equal(vm.runInContext('stopped',context),true);
 assert.equal(element('test-label').textContent,'STOP REQUESTED');
 assert.equal(element('stop').disabled,true);
 console.log('preview recovery: passed');
})().catch(error=>{console.error(error);process.exitCode=1});
