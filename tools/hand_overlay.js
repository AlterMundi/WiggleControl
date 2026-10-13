// Metadata-only feedback over ordinary trusted system input; no camera image fetch.
if (location.protocol === 'http:' && location.pathname === '/qualification') {
  document.body.classList.add('feedback-mode');
  const canvas=document.createElement('canvas');canvas.id='hand-overlay';document.body.append(canvas);
  const ctx=canvas.getContext('2d');
  const indicator=document.createElement('div');indicator.id='feedback-indicator';indicator.innerHTML='<span id="feedback-light"></span><b id="feedback-label">PREPARING</b><b id="feedback-time">--:--</b><button id="feedback-stop">STOP INPUT</button>';document.querySelector('header').append(indicator);
  const box=document.createElement('div');box.id='hand-status';box.innerHTML='<strong id="hand-phase">Waiting for hand</strong><p id="hand-reason">Move the pointer to position the hand overlay.</p><p>Press intents <b id="hand-presses">0</b> · frame <b id="hand-age">—</b></p>';
  document.getElementById('summary').parentElement.insertBefore(box,document.getElementById('summary'));
  let stopped=false,ready=false,deadline=null,state=null;
  const set=(id,text)=>document.getElementById(id).textContent=text;
  const edges=[[0,1],[1,2],[2,3],[3,4],[0,5],[5,6],[6,7],[7,8],[5,9],[9,10],[10,11],[11,12],[9,13],[13,14],[14,15],[15,16],[13,17],[0,17],[17,18],[18,19],[19,20]];
  const phases={pressed:'PINCH PRESSED',closing:'Pinch closing',open:'Open / ready',open_first:'Separate thumb and index',not_eligible:'Hand not ready',scrolling:'Scroll gesture'};
  const reasons={open_before_press:'Separate thumb and index before the next pinch.',confirming:'Keep the pinch closed across fresh frames.',close_threshold:'Bring thumb and index closer.',stale_frame:'Frame too old.',stale_input:'No fresh input.',tracking_lost:'Hand tracking lost.',hand_not_engaged:'Posture rejected; keep fingers visible.',pinch_not_eligible:'Posture not eligible for pinch.',low_confidence:'Low model confidence.',scroll_gesture:'Scroll mode; pinch click blocked.'};
  function clear(){ctx.clearRect(0,0,canvas.width,canvas.height);}
  function draw(){
    canvas.width=innerWidth;canvas.height=innerHeight;
    const anchor=window.inputEvidence.last;
    if(!state?.landmarks||!anchor||!state.fresh)return;
    const p=state.landmarks,center=[(p[5][0]+p[17][0])/2,(p[5][1]+p[17][1])/2];
    const points=p.map(([x,y])=>[anchor.x-(x-center[0]),anchor.y+(y-center[1])]);
    ctx.globalAlpha=.85;ctx.lineWidth=3;ctx.strokeStyle='#87d7ee';
    for(const [a,b]of edges){ctx.beginPath();ctx.moveTo(...points[a]);ctx.lineTo(...points[b]);ctx.stroke();}
    for(const point of points){ctx.beginPath();ctx.arc(...point,4,0,Math.PI*2);ctx.fillStyle='#a8dac6';ctx.fill();}
    ctx.strokeStyle=state.pinch_state==='pressed'?'#a8dac6':'#dcb3c3';ctx.lineWidth=5;ctx.beginPath();ctx.moveTo(...points[4]);ctx.lineTo(...points[8]);ctx.stroke();
    ctx.beginPath();ctx.arc(anchor.x,anchor.y,12,0,Math.PI*2);ctx.strokeStyle='#f4c283';ctx.stroke();
  }
  document.addEventListener('pointermove',e=>{if(e.isTrusted&&!stopped)draw();});
  function end(label){stopped=true;clear();set('feedback-label',label);indicator.classList.remove('testing');document.getElementById('feedback-stop').disabled=true;set('hand-phase','Stopped');}
  document.getElementById('feedback-stop').onclick=async()=>{try{await fetch('/stop',{method:'POST',cache:'no-store'});}finally{end('INPUT STOPPED');}};
  setInterval(()=>{if(stopped||deadline===null)return;const ms=Math.max(0,deadline-performance.now()),s=Math.ceil(ms/1000);set('feedback-time',String(Math.floor(s/60)).padStart(2,'0')+':'+String(s%60).padStart(2,'0'));if(ms===0)end('TEST ENDED');},100);
  async function poll(){if(stopped)return;try{
    const started=performance.now(),response=await fetch('/state',{cache:'no-store',signal:AbortSignal.timeout(1500)});
    if(stopped)return;if(response.status===204)return;if(!response.ok)throw Error('State unavailable');state=await response.json();
    draw();
    if(!ready){await new Promise(requestAnimationFrame);const ack=await fetch('/ready',{method:'POST',cache:'no-store'});ready=ack.ok;}
    if(state.test_phase==='testing'){deadline=started+state.remaining_ms;indicator.classList.add('testing');set('feedback-label','TESTING');}
    set('hand-phase',phases[state.pinch_state]||state.pinch_state);box.classList.toggle('pressed',state.fresh&&state.pinch_state==='pressed');
    set('hand-reason',!window.inputEvidence.last?'Move your hand to locate the system pointer.':reasons[state.blocked_reason]||'Pinch recognized; separate fingers to release.');set('hand-presses',state.presses);set('hand-age',state.age_ms.toFixed(0)+' ms');
  }catch(error){if(!stopped){const expired=deadline!==null&&performance.now()>=deadline;set('feedback-time',expired?'00:00':'--:--');end(expired?'TEST ENDED':'FEEDBACK DISCONNECTED');}}
  finally{if(!stopped)setTimeout(poll,70);}}
  poll();
}
