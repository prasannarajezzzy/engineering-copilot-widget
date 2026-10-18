import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import './style.css';

type Tab = 'Brief'|'Tickets'|'PRs'|'Task Flow'|'Memory'|'AI Setup';
type Provider = 'openai'|'anthropic'|'custom';
type Settings = {provider:Provider;model:string;endpoint:string};
type ProviderStatus = {settings:Settings;has_key:boolean};
const models:Record<Provider,string[]> = {
 openai:['gpt-5.1','gpt-5-mini','gpt-4.1-mini'],
 anthropic:['claude-sonnet-4-5','claude-haiku-4-5','claude-opus-4-1'],
 custom:[]
};

function Teddy({ small=false }:{ small?:boolean }){
 return <svg className={small?'teddy teddy-sm':'teddy'} viewBox="0 0 64 64" aria-hidden="true">
  <circle cx="14" cy="16" r="12" fill="#E4BC93"/>
  <circle cx="50" cy="16" r="12" fill="#E4BC93"/>
  <circle cx="14" cy="16" r="6.5" fill="#F4C9C2"/>
  <circle cx="50" cy="16" r="6.5" fill="#F4C9C2"/>
  <circle cx="32" cy="36" r="23" fill="#F3D2B0"/>
  <ellipse cx="22.5" cy="40" rx="4" ry="2.3" fill="#F0B7AE"/>
  <ellipse cx="41.5" cy="40" rx="4" ry="2.3" fill="#F0B7AE"/>
  <circle cx="24.5" cy="33.5" r="2.3" fill="#5C4033"/>
  <circle cx="39.5" cy="33.5" r="2.3" fill="#5C4033"/>
  <circle cx="25.2" cy="32.7" r="0.7" fill="#fff"/>
  <circle cx="40.2" cy="32.7" r="0.7" fill="#fff"/>
  <ellipse cx="32" cy="40" rx="3.4" ry="2.5" fill="#C4846A"/>
  <path d="M26.5 45 Q32 50.5 37.5 45" fill="none" stroke="#8C5A45" strokeWidth="1.7" strokeLinecap="round"/>
 </svg>;
}

type JiraTicket = {key:string;summary:string;description:string;status:string;comments:string[]};
type Workflow = {ticket:JiraTicket;discussion:string;plan:string;repo:string;branch:string;stage:string};
type JiraConfig = {base_url:string;email:string;project_key:string};
function TaskFlow(){
 const [config,setConfig]=useState<JiraConfig>({base_url:'',email:'',project_key:''});
 const [token,setToken]=useState('');
 const [items,setItems]=useState<JiraTicket[]>([]);
 const [ticketKey,setTicketKey]=useState('');
 const [workflow,setWorkflow]=useState<Workflow|null>(null);
 const [message,setMessage]=useState('');
 const [repo,setRepo]=useState('');
 const [inspection,setInspection]=useState('');
 const [working,setWorking]=useState(false);
 const [result,setResult]=useState('');
 useEffect(()=>{void invoke<JiraConfig|null>('jira_status').then(x=>{if(x)setConfig(x)});void invoke<Workflow|null>('workflow_status').then(setWorkflow);},[]);
 async function run<T>(action:()=>Promise<T>,done:(v:T)=>void=()=>{}){setWorking(true);setResult('');try{const r=await action();done(r);}catch(e){setResult('Error: '+String(e))}finally{setWorking(false)}}
 const action=(cmd:string,args:Record<string,unknown>={})=>run(()=>invoke<Workflow>(cmd,args),setWorkflow);
 return <div className="settings taskflow">
 <div className="sectionhead">Jira → Plan → Repository <span>GUIDED WORKFLOW</span></div>
 <p className="muted">Read Jira tasks, clarify requirements with your selected AI model, approve a plan, then create an isolated local Git branch. No source edits, commits, pushes, or Jira changes are made automatically.</p>
 <label>Jira Cloud site URL</label><input placeholder="https://company.atlassian.net" value={config.base_url} onChange={e=>setConfig(v=>({...v,base_url:e.target.value}))}/>
 <label>Jira email</label><input placeholder="you@company.com" value={config.email} onChange={e=>setConfig(v=>({...v,email:e.target.value}))}/>
 <label>Jira project key</label><input placeholder="ENG" value={config.project_key} onChange={e=>setConfig(v=>({...v,project_key:e.target.value.toUpperCase()}))}/>
 <label>Jira API token (saved in Windows credential store)</label><input type="password" autoComplete="off" placeholder="Leave blank to keep existing token" value={token} onChange={e=>setToken(e.target.value)}/>
 <div className="settingbuttons"><button disabled={working} onClick={()=>void run(()=>invoke<void>('configure_jira',{config,token}),()=>{setToken('');setResult('Jira settings saved')})}>Save Jira</button><button disabled={working} onClick={()=>void run(()=>invoke<JiraTicket[]>('list_jira_tickets'),setItems)}>Load tickets</button></div>
 {items.length>0&&<select defaultValue="" onChange={e=>setTicketKey(e.target.value)}><option value="">Choose a Jira ticket…</option>{items.map(i=><option key={i.key} value={i.key}>{i.key} — {i.summary}</option>)}</select>}
 <label>Ticket key</label><input placeholder="ENG-142" value={ticketKey} onChange={e=>setTicketKey(e.target.value.toUpperCase())}/>
 <button disabled={working||!ticketKey} onClick={()=>void action('import_jira_ticket',{key:ticketKey})}>Pull ticket and start discussion</button>
 {workflow&&<div className="workflowbox">
 <b>{workflow.ticket.key} — {workflow.ticket.summary}</b><p className="muted">{workflow.ticket.status} · {workflow.ticket.description}</p>
 {workflow.ticket.comments.length>0&&<div className="notes"><b>Jira comments</b>{workflow.ticket.comments.map((c,i)=><p key={i}>{c}</p>)}</div>}
 <div className="step">Stage: {workflow.stage.replaceAll('_',' ')}</div>
 {workflow.discussion&&<pre className="discussion">{workflow.discussion}</pre>}
 {workflow.stage==='discuss'&&<><textarea rows={3} placeholder="Clarify requirements, acceptance criteria, constraints…" value={message} onChange={e=>setMessage(e.target.value)}/><div className="settingbuttons"><button disabled={working||!message.trim()} onClick={()=>void run(()=>invoke<Workflow>('discuss_ticket',{message}),w=>{setWorkflow(w);setMessage('')})}>Discuss with AI</button><button disabled={working} onClick={()=>void action('generate_ticket_plan')}>Create plan</button></div></>}
 {workflow.plan&&<div className="plan"><b>Implementation plan</b><pre>{workflow.plan}</pre></div>}
 {workflow.stage==='plan_review'&&<div className="settingbuttons"><button disabled={working} onClick={()=>void action('approve_ticket_plan')}>Approve plan</button><button className="secondary" disabled={working} onClick={()=>void action('revise_ticket_plan')}>Revise</button></div>}
 {workflow.stage==='repo_selection'&&<><label>Local repository root path</label><input placeholder="C:\Users\you\code\my-repo" value={repo} onChange={e=>{setRepo(e.target.value);setInspection('')}}/><div className="settingbuttons"><button disabled={working||!repo} onClick={()=>void run(()=>invoke<string>('inspect_repo',{path:repo}),setInspection)}>Inspect repo</button><button disabled={working||!repo||!inspection} onClick={()=>void action('start_ticket_work',{path:repo})}>Start work on branch</button></div>{inspection&&<pre className="discussion">{inspection}</pre>}<p className="muted">Confirm this is the correct repository before starting. A clean working tree is required.</p></>}
 {workflow.stage==='workspace_ready'&&<div className="plan"><b>Workspace prepared ✓</b><p>Repository: {workflow.repo}</p><p>Branch: {workflow.branch}</p><p>Approved plan saved under .engineering-copilot/. Open this repository in Cursor or VS Code to implement and test the changes.</p></div>}
 </div>}
 {result&&<p className="feedback" role="status">{result}</p>}
 </div>
}

const tickets = [{id:'ENG-142',title:'Authentication integration',detail:'New comment from a teammate',tag:'Needs reply'}, {id:'ENG-156',title:'Retry handling',detail:'Status moved to In Review',tag:'Updated'}];
const prs = [{id:'#284',title:'Auth middleware',detail:'Potential null-check issue',tag:'Review'}, {id:'#291',title:'Queue retry policy',detail:'Ready for review',tag:'Review'}];

function App(){
 const [expanded,setExpanded]=useState(false);
 const [settings,setSettings]=useState<Settings>({provider:'openai',model:'gpt-5.1',endpoint:''});
 const [key,setKey]=useState('');
 const [savedKey,setSavedKey]=useState(false);
 const [busy,setBusy]=useState(false);
 const [feedback,setFeedback]=useState('');
 const [replies,setReplies]=useState<{role:'user'|'assistant';text:string}[]>([]);
 useEffect(()=>{void invoke<ProviderStatus>('get_provider_status').then(s=>{setSettings(s.settings);setSavedKey(s.has_key);}).catch(e=>setFeedback(String(e)));},[]);
 async function loadProvider(provider:Provider){
  const next={provider,model:models[provider][0]||'',endpoint:''};
  setSettings(next);setKey('');setFeedback('');
  // Credentials belong to each provider; the status will be refreshed when settings are saved.
  setSavedKey(false);
 }
 async function saveSettings(){setBusy(true);setFeedback('');try{
  const result=await invoke<ProviderStatus>('save_provider_settings',{settings,apiKey:key.trim()||null});
  setSettings(result.settings);setSavedKey(result.has_key);setKey('');setFeedback('Provider settings saved securely.');
 }catch(e){setFeedback(String(e));}finally{setBusy(false);}}
 async function removeKey(){setBusy(true);try{const r=await invoke<ProviderStatus>('delete_provider_key',{provider:settings.provider});setSavedKey(r.has_key);setKey('');setFeedback('Saved credential removed.');}catch(e){setFeedback(String(e));}finally{setBusy(false);}}
 async function sendAI(){const question=draft.trim();if(!question||busy)return;setBusy(true);setDraft('');setReplies(v=>[...v,{role:'user',text:question}]);try{
  const answer=await invoke<string>('ask_ai',{prompt:question});setReplies(v=>[...v,{role:'assistant',text:answer}]);
 }catch(e){setReplies(v=>[...v,{role:'assistant',text:'Error: '+String(e)}]);}finally{setBusy(false);}}

 const [tab,setTab]=useState<Tab>('Brief');
 
 const [draft,setDraft]=useState('');
 const [error,setError]=useState('');
 const [demoCount,setDemoCount]=useState(3);
 const [night,setNight]=useState(()=>{try{return localStorage.getItem('copilot-theme')==='night';}catch{return false;}});
 useEffect(()=>{try{localStorage.setItem('copilot-theme',night?'night':'day');}catch{/* theme is optional */}},[night]);
 const bubbleDrag=useRef<{pointerId:number;startX:number;startY:number;lastX:number;lastY:number;originX:number;originY:number;scale:number;moved:boolean;ready:boolean}|null>(null);
 const moveQueue=useRef<{x:number;y:number}|null>(null);
 const moveBusy=useRef(false);
 async function toggle(){if(!('__TAURI_INTERNALS__' in window)){setExpanded(v=>!v);return;}try{const value=await invoke<boolean>('toggle_panel');setExpanded(value);setError('');}catch(e){setError(String(e));}}
 async function drag(){try{await invoke('begin_drag');}catch(e){setError(String(e));}}
 async function pumpMove(){if(moveBusy.current)return;moveBusy.current=true;try{while(moveQueue.current){const next=moveQueue.current;moveQueue.current=null;await invoke('move_widget',next);}}finally{moveBusy.current=false;}}
 function queueBubble(drag:{originX:number;originY:number;scale:number;startX:number;startY:number;lastX:number;lastY:number}){const dx=(drag.lastX-drag.startX)*drag.scale;const dy=(drag.lastY-drag.startY)*drag.scale;moveQueue.current={x:Math.round(drag.originX+dx),y:Math.round(drag.originY+dy)};void pumpMove();}
 async function onBubbleDown(e:React.PointerEvent<HTMLButtonElement>){if(e.button!==0)return;e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);const session={pointerId:e.pointerId,startX:e.screenX,startY:e.screenY,lastX:e.screenX,lastY:e.screenY,originX:0,originY:0,scale:1,moved:false,ready:false};bubbleDrag.current=session;try{const origin=await invoke<{x:number;y:number;scale:number}>('widget_origin');if(bubbleDrag.current!==session)return;session.originX=origin.x;session.originY=origin.y;session.scale=origin.scale;session.ready=true;if(Math.hypot(session.lastX-session.startX,session.lastY-session.startY)>=5){session.moved=true;queueBubble(session);}}catch(err){if(bubbleDrag.current!==session)return;const moved=session.moved;bubbleDrag.current=null;if(!moved)void toggle();else setError(String(err));}}
 function onBubbleMove(e:React.PointerEvent<HTMLButtonElement>){const drag=bubbleDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;drag.lastX=e.screenX;drag.lastY=e.screenY;if(Math.hypot(e.screenX-drag.startX,e.screenY-drag.startY)<5)return;drag.moved=true;if(!drag.ready)return;queueBubble(drag);}
 function onBubbleUp(e:React.PointerEvent<HTMLButtonElement>){const drag=bubbleDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;bubbleDrag.current=null;if(!drag.moved)void toggle();}
 async function notify(){try{await invoke('test_notification',{body:'Your floating engineering assistant is ready. (Demo notification)'});}catch(e){setError(String(e));}}

 return <div className={`shell ${expanded?'expanded':'collapsed'}${night?' night':''}`}>
  {!expanded ? <button className="bubble" onPointerDown={onBubbleDown} onPointerMove={onBubbleMove} onPointerUp={onBubbleUp} onContextMenu={e=>e.preventDefault()} title="Drag to a corner of the screen. Click to expand." aria-label="Copilot"><Teddy/><span className="signal"/></button> :
  <div className="panel">
   <div className="topbar" onMouseDown={e=>{if(e.button===0 && e.target===e.currentTarget)void drag();}}>
    <span className="logo"><Teddy small/></span><div className="heading"><strong>Copilot</strong><small>Your desk companion</small></div>
    <button className={night?'iconbtn on':'iconbtn'} aria-pressed={night} title={night?'Switch to day':'Switch to night'} onClick={()=>setNight(v=>!v)}>{night?'☀':'☾'}</button>
    <button className="iconbtn" title="Drag window" onMouseDown={e=>{e.preventDefault();void drag();}}>⠿</button>
    <button className="iconbtn" title="Minimize to floating bubble" onClick={toggle}>−</button>
   </div>
   <div className="welcome"><div><span className="status"/> Ready when you are</div><button className="notify" onClick={notify}>Say hello</button></div>
   {tab!=='Brief'&&tab!=='Tickets'&&tab!=='PRs'&&<div className="greeting"><small>TODAY</small><h1>Hello there</h1><p>Jira, Slack, and GitHub will show up here when you connect them.</p></div>}
   {tab==='Brief'&&<div className="stats"><div><b>{demoCount}</b><small>Sample updates</small></div><div><b>2</b><small>Sample tickets</small></div><div><b>2</b><small>Sample PRs</small></div></div>}
   <div className="tabs">{(['Brief','Tickets','PRs','Task Flow','Memory','AI Setup'] as Tab[]).map(t=><button className={tab===t?'selected':''} onClick={()=>setTab(t)} key={t}>{t}</button>)}</div>
   <div className="content">
   {tab==='Brief'&& <><div className="sectionhead">Today's briefing <span>DEMO</span></div><div className="item"><b>● Jira: comment requires attention</b><p>ENG-142 · A teammate requested clarification.</p></div><div className="item"><b>◆ GitHub: PR awaiting review</b><p>#284 · Authentication middleware has new changes.</p></div><div className="item"><b>◌ Slack: conversation summary</b><p>No account connected yet. Phase 2 will add live events.</p></div></>}
   {tab==='Tickets'&& <><div className="sectionhead">Jira tickets <span>DEMO DATA</span></div>{tickets.map(x=><div className="item" key={x.id}><span className="code">{x.id} · {x.tag}</span><b>{x.title}</b><p>{x.detail}</p></div>)}</>}
   {tab==='PRs'&& <><div className="sectionhead">Pull request queue <span>DEMO DATA</span></div>{prs.map(x=><div className="item" key={x.id}><span className="code">{x.id} · {x.tag}</span><b>{x.title}</b><p>{x.detail}</p></div>)}</>}
   {tab==='Task Flow'&&<TaskFlow/>}
   {tab==='Memory'&& <><div className="sectionhead">Shared memory <span>COMING IN PHASE 4</span></div><div className="item"><b>One knowledge base for Claude + Cursor</b><p>We will add persistent storage, access controls, source links and an MCP server. No live memory is connected yet.</p></div></>}
   {tab==='AI Setup'&&<div className="settings">
      <div className="sectionhead">AI provider onboarding <span>API SETTINGS</span></div>
      <label>Provider</label>
      <select value={settings.provider} onChange={e=>void loadProvider(e.target.value as Provider)}><option value="openai">OpenAI</option><option value="anthropic">Anthropic Claude</option><option value="custom">Custom OpenAI-compatible API</option></select>
      <label>Model</label>
      {settings.provider!=='custom'&&<select value={models[settings.provider].includes(settings.model)?settings.model:'__custom'} onChange={e=>setSettings(v=>({...v,model:e.target.value==='__custom'?'':e.target.value}))}>{models[settings.provider].map(m=><option key={m} value={m}>{m}</option>)}<option value="__custom">Other model ID…</option></select>}
      {(settings.provider==='custom'||!models[settings.provider].includes(settings.model))&&<input aria-label="Model ID" placeholder="Exact model ID" value={settings.model} onChange={e=>setSettings(v=>({...v,model:e.target.value}))}/>}
      {settings.provider==='custom'&&<><label>Chat completions endpoint (full URL)</label><input placeholder="https://your-host.example/v1/chat/completions" value={settings.endpoint} onChange={e=>setSettings(v=>({...v,endpoint:e.target.value}))}/></>}
      <label>API key {savedKey?'· credential saved':''}</label>
      <input type="password" autoComplete="off" placeholder={savedKey?'Leave blank to keep stored key':'Enter API key (custom local: NO_KEY)'} value={key} onChange={e=>setKey(e.target.value)}/>
      <p className="muted">Keys are stored in Windows Credential Manager, not in browser storage. Custom providers must expose the OpenAI chat-completions response format. For a local no-auth server, enter NO_KEY.</p>
      <div className="settingbuttons"><button disabled={busy} onClick={()=>void saveSettings()}>Save configuration</button><button className="secondary" disabled={busy||!savedKey} onClick={()=>void removeKey()}>Remove key</button></div>
      {feedback&&<p className="feedback" role="status">{feedback}</p>}
      <p className="muted">Use the chat below after saving. Each request is sent to the selected provider; API usage may incur charges.</p>
    </div>}
   {replies.length>0&&<div className="notes"><b>AI chat · {settings.provider}</b>{replies.map((m,i)=><p key={i}><strong>{m.role==='user'?'You':'AI'}:</strong> {m.text}</p>)}</div>}
   </div>
   <form className="composer" onSubmit={e=>{e.preventDefault();void sendAI();}}><input value={draft} onChange={e=>setDraft(e.target.value)} placeholder="Ask your selected AI model…" aria-label="Local note"/><button type="submit" title="Send to AI provider">↗</button></form>
   {error&&<div className="error" role="alert">{error}</div>}
   <footer>Jira, Slack and GitHub remain demo-only · AI uses your API key</footer>
  </div>}
 </div>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
