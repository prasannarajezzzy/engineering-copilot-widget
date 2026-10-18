import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import './style.css';

type Tab = 'Brief'|'Tickets'|'PRs'|'Meet'|'Task Flow'|'Memory'|'AI Setup';
type Provider = 'openai'|'anthropic'|'custom';
type Settings = {provider:Provider;model:string;endpoint:string};
type ProviderStatus = {settings:Settings;has_key:boolean};
const models:Record<Provider,string[]> = {
 openai:['gpt-5.1','gpt-5-mini','gpt-4.1-mini'],
 anthropic:['claude-sonnet-4-5','claude-haiku-4-5','claude-opus-4-1'],
 custom:[]
};

function IconResize(){return <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M1.5 8h13M4.5 5 1.5 8l3 3M11.5 5l3 3-3 3" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/></svg>;}
function IconFull(){return <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M6 2.5H2.5V6M10 2.5h3.5V6M13.5 10v3.5H10M2.5 10v3.5H6" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/></svg>;}
function IconWidget(){return <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><rect x="5" y="2.2" width="6" height="11.6" rx="1.8" fill="none" stroke="currentColor" strokeWidth="1.6"/></svg>;}
function Teddy({ small=false, phones=false }:{ small?:boolean; phones?:boolean }){
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
  {phones&&<g className="phones">
   <path d="M16 16 Q32 4 48 16" fill="none" stroke="#6b4a3a" strokeWidth="3.2" strokeLinecap="round"/>
   <rect x="6" y="14" width="12" height="16" rx="5" fill="#C4846A" stroke="#6b4a3a" strokeWidth="1.6"/>
   <rect x="46" y="14" width="12" height="16" rx="5" fill="#C4846A" stroke="#6b4a3a" strokeWidth="1.6"/>
  </g>}
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

type PullRequest = {repo:string;number:number;title:string;review:string;url:string;draft:boolean};
type PullPage = {login:string;source:string;pulls:PullRequest[]};
function PullRequests(){
 const [token,setToken]=useState('');
 const [page,setPage]=useState<PullPage|null>(null);
 const [saved,setSaved]=useState(false);
 const [working,setWorking]=useState(false);
 const [note,setNote]=useState('');
 const [problem,setProblem]=useState('');
 async function load(){setWorking(true);setProblem('');try{const next=await invoke<PullPage>('list_github_pulls');setPage(next);}catch(e){setPage(null);setProblem(String(e));}finally{setWorking(false);}}
 useEffect(()=>{void invoke<{has_token:boolean}>('github_status').then(s=>setSaved(s.has_token)).catch(()=>{});void load();},[]);
 async function save(){setWorking(true);setNote('');setProblem('');try{const login=await invoke<string>('save_github_token',{token});setToken('');setSaved(true);const next=await invoke<PullPage>('list_github_pulls');setPage(next);setNote('Saved for '+login);}catch(e){setProblem(String(e));}finally{setWorking(false);}}
 async function clearToken(){setWorking(true);setNote('');try{await invoke('clear_github_token');setSaved(false);setNote('Saved token removed.');}catch(e){setProblem(String(e));}finally{setWorking(false);}await load();}
 return <div className="settings">
  <div className="sectionhead">Open pull requests {page&&<span>{page.login}</span>}</div>
  <p className="muted">{page?`Signed in as ${page.login}${page.source==='gh'?' through the GitHub CLI.':' with a saved token.'}`:'Paste a GitHub personal access token that can read pull requests, or sign in with the GitHub CLI. Only open pull requests you authored are listed.'}</p>
  <label>GitHub token {saved?'· credential saved':''}</label>
  <input type="password" autoComplete="off" placeholder={saved?'Leave blank to keep the saved token':'github_pat_… or classic token'} value={token} onChange={e=>setToken(e.target.value)}/>
  <div className="settingbuttons">
   <button disabled={working||!token.trim()} onClick={()=>void save()}>Save token</button>
   <button className="secondary" disabled={working} onClick={()=>void load()}>Refresh</button>
   <button className="secondary" disabled={working||!saved} onClick={()=>void clearToken()}>Remove token</button>
  </div>
  {page&&page.pulls.length===0&&<p className="muted">No open pull requests for {page.login}.</p>}
  {page?.pulls.map(pr=><div className="item" key={pr.url}><span className="code">{pr.repo} #{pr.number} · {pr.review}</span><b>{pr.title}</b><p>{pr.url}</p></div>)}
  {note&&<p className="feedback" role="status">{note}</p>}
  {problem&&<p className="error" role="alert">{problem}</p>}
 </div>;
}

type MeetingNote = {id:string;created:number;decisions:string[];actions:string[];recap:string;heard:string;transcript:string;trimmed:boolean};
function clock(total:number){const minutes=Math.floor(total/60);const seconds=total%60;return `${minutes}:${String(seconds).padStart(2,'0')}`;}
function stamp(created:number){
 try{
  const date=new Date(created*1000);
  const opts:Intl.DateTimeFormatOptions={month:'short',day:'numeric'};
  if(date.getFullYear()!==new Date().getFullYear())opts.year='numeric';
  return {date:date.toLocaleDateString([],opts),time:date.toLocaleTimeString([],{hour:'numeric',minute:'2-digit'})};
 }catch{return {date:'Saved',time:''};}
}
function titleOf(note:MeetingNote){
 const source=note.recap.trim()||note.decisions.find(Boolean)||note.actions.find(Boolean)||note.heard||'Meeting notes';
 const words=source.match(/[A-Za-z0-9']+/g)??[];
 const four=words.slice(0,4);
 if(!four.length)return 'Meeting notes';
 const text=four.join(' ');
 return text.charAt(0).toUpperCase()+text.slice(1);
}
function Meetings({notes,live,heard,since,busy,problem,onListen}:{notes:MeetingNote[];live:boolean;heard:string;since:number|null;busy:boolean;problem:string;onListen:()=>void}){
 const [now,setNow]=useState(()=>Date.now());
 const [copied,setCopied]=useState('');
 const [copyError,setCopyError]=useState('');
 const [openId,setOpenId]=useState<string|null>(null);
 const newest=useRef<string|null>(null);
 useEffect(()=>{if(!live)return;const id=window.setInterval(()=>setNow(Date.now()),500);return ()=>window.clearInterval(id);},[live]);
 useEffect(()=>{const id=notes[0]?.id??null;if(id!==newest.current){newest.current=id;setOpenId(id);}},[notes]);
 async function copy(note:MeetingNote){
  const text=[
   note.decisions.length?`Decisions\n${note.decisions.map(item=>`• ${item}`).join('\n')}`:'',
   note.actions.length?`Action items\n${note.actions.map(item=>`• ${item}`).join('\n')}`:'',
   note.recap?`Recap\n${note.recap}`:''
  ].filter(Boolean).join('\n\n');
  try{await navigator.clipboard.writeText(text);setCopied(note.id);setCopyError('');}catch{setCopyError('I could not copy that.');}
 }
 const elapsed=since?Math.max(0,Math.floor((now-since)/1000)):0;
 const label=busy&&live?'Writing notes…':live?"I'm all ears":busy?'One moment…':"I'll take notes";
 return <div className="meet">
  <div className="sectionhead">Meeting notes <span>JUST FOR YOU</span></div>
  <p className="muted meet-lead">Hover the teddy and choose Start listening. I hear your microphone and the call on this PC, then keep the notes here.</p>
  <button className={`orb${live?' live':''}${busy?' busy':''}`} disabled={busy} aria-pressed={live} aria-label={live?'Stop listening':'Start listening'} onClick={onListen}>
   <Teddy phones={live}/>
  </button>
  <div className="meet-status">{label}</div>
  {live&&!busy&&since!=null&&<div className="meet-time">{clock(elapsed)}</div>}
  {live&&<button type="button" className="stop-listen" disabled={busy} onClick={onListen}>{busy?'Writing your notes…':'Stop listening'}</button>}
  <p className="muted meet-heard">{live&&heard?heard:!live&&!busy?'Or hover the little teddy outside and press Stop listening.':''}</p>
  {(problem||copyError)&&<p className="meet-problem" role="alert">{problem||copyError}</p>}
  {notes.length===0&&!live&&<p className="muted meet-empty">No meetings yet.</p>}
  {notes.length>0&&<ul className="meet-list">
   {notes.map(note=>{
    const open=note.id===openId;
    const {date,time}=stamp(note.created);
    const title=titleOf(note);
    return <li className={open?'meet-item open':'meet-item'} key={note.id}>
     <div className="meet-line">
      <button type="button" className="meet-rowbtn" aria-expanded={open} aria-label={open?`Close ${title}`:`Open ${title}`} onClick={()=>setOpenId(open?null:note.id)}>
       {!open&&<span className="meet-title">{title}</span>}
       <span className="meet-when">{date}{time?`, ${time}`:''}</span>
       <span className="meet-chevron" aria-hidden="true"/>
      </button>
      {open&&<button type="button" className="copybtn" onClick={()=>void copy(note)}>{copied===note.id?'Copied':'Copy'}</button>}
     </div>
     {open&&<div className="meet-body">
      <p className="heard">{note.heard}{note.trimmed?' · first 10 minutes':''}</p>
      {note.decisions.length>0&&<div><small>Decisions</small><ul>{note.decisions.map((item,index)=><li key={index}>{item}</li>)}</ul></div>}
      {note.actions.length>0&&<div><small>Action items</small><ul>{note.actions.map((item,index)=><li key={index}>{item}</li>)}</ul></div>}
      {note.recap&&<p className="recap">{note.recap}</p>}
      {note.transcript&&<details><summary>What I heard</summary><p>{note.transcript}</p></details>}
     </div>}
    </li>;
   })}
  </ul>}
 </div>;
}

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
 const [listening,setListening]=useState(false);
 const [listenBusy,setListenBusy]=useState(false);
 const [listenError,setListenError]=useState('');
 const [heard,setHeard]=useState('');
 const [since,setSince]=useState<number|null>(null);
 const [meetingNotes,setMeetingNotes]=useState<MeetingNote[]>(()=>location.search.includes('preview')?[
  {id:'a',created:Math.floor(Date.now()/1000),decisions:['None stated.'],actions:['Send the recap'],recap:'Hearing you and the call.',heard:'Hearing you and the call.',transcript:'We talked through the call.',trimmed:false},
  {id:'b',created:Math.floor(Date.now()/1000)-86400,decisions:['Ship on Friday'],actions:['Alex updates the board'],recap:'We agreed to ship the widget.',heard:'Microphone and the call.',transcript:'',trimmed:false},
  {id:'c',created:Math.floor(Date.now()/1000)-172800,decisions:[],actions:['Someone will write the notes'],recap:'Standup stayed short today.',heard:'Microphone only.',transcript:'',trimmed:true}
 ]:[]);
 const [frameMode,setFrameMode]=useState<'bubble'|'widget'|'desk'|'full'>('bubble');
 const [hoverMenu,setHoverMenu]=useState(false);
 const [menuCorner,setMenuCorner]=useState('br');
 const menuCornerRef=useRef('br');
 const hoverTimer=useRef<number|null>(null);
 const bubbleDrag=useRef<{pointerId:number;startX:number;startY:number;lastX:number;lastY:number;originX:number;originY:number;scale:number;moved:boolean;ready:boolean}|null>(null);
 const moveQueue=useRef<{x:number;y:number}|null>(null);
 const moveBusy=useRef(false);
 useEffect(()=>{void invoke<MeetingNote[]>('list_meetings').then(setMeetingNotes).catch(()=>{});void invoke<boolean>('meeting_listening').then(on=>{if(on){setListening(true);setSince(s=>s??Date.now());}}).catch(()=>{});},[]);
 async function toggle(){if(!('__TAURI_INTERNALS__' in window)){setExpanded(v=>!v);setFrameMode(expanded?'bubble':'widget');return;}try{const value=await invoke<boolean>('toggle_panel');setExpanded(value);setFrameMode(value?'widget':'bubble');setError('');}catch(e){setError(String(e));}}
 async function setFrame(mode:'bubble'|'widget'|'desk'|'full'){if(!('__TAURI_INTERNALS__' in window)){setFrameMode(mode);setExpanded(mode!=='bubble');if(mode==='full'){try{await document.documentElement.requestFullscreen();}catch{/* the browser can refuse fullscreen */}}else if(document.fullscreenElement){try{await document.exitFullscreen();}catch{}}return;}try{const applied=await invoke<'bubble'|'widget'|'desk'|'full'>('set_window_mode',{mode});setFrameMode(applied);setExpanded(applied!=='bubble');setError('');}catch(e){setError(String(e));}}
 const edgeDrag=useRef<{pointerId:number;edge:string;sx:number;sy:number;x:number;y:number;w:number;h:number}|null>(null);
 const edgeQueue=useRef<{x:number;y:number;width:number;height:number}|null>(null);
 const edgeBusy=useRef(false);
 async function pumpEdge(){if(edgeBusy.current)return;edgeBusy.current=true;try{while(edgeQueue.current){const next=edgeQueue.current;edgeQueue.current=null;await invoke('place_window',next);}}finally{edgeBusy.current=false;}}
 async function onEdgeDown(edge:string,e:React.PointerEvent<HTMLDivElement>){if(e.button!==0)return;e.preventDefault();e.stopPropagation();e.currentTarget.setPointerCapture(e.pointerId);if(!('__TAURI_INTERNALS__' in window))return;try{const origin=await invoke<{x:number;y:number;scale:number}>('widget_origin');edgeDrag.current={pointerId:e.pointerId,edge,sx:e.screenX,sy:e.screenY,x:origin.x/origin.scale,y:origin.y/origin.scale,w:window.innerWidth,h:window.innerHeight};}catch(err){setError(String(err));}}
 function onEdgeMove(e:React.PointerEvent<HTMLDivElement>){const drag=edgeDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;const dx=e.screenX-drag.sx;const dy=e.screenY-drag.sy;let x=drag.x,y=drag.y,w=drag.w,h=drag.h;if(drag.edge.includes('e'))w=drag.w+dx;if(drag.edge.includes('w')){w=drag.w-dx;x=drag.x+dx;}if(drag.edge.includes('s'))h=drag.h+dy;if(drag.edge.includes('n')){h=drag.h-dy;y=drag.y+dy;}edgeQueue.current={x,y,width:w,height:h};void pumpEdge();}
 function onEdgeUp(e:React.PointerEvent<HTMLDivElement>){const drag=edgeDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;edgeDrag.current=null;}
 async function placeMenu(open:boolean){if(!('__TAURI_INTERNALS__' in window))return;try{const place=await invoke<{corner:string}>('set_bubble_menu',{open,corner:open?null:menuCornerRef.current});menuCornerRef.current=place.corner;setMenuCorner(place.corner);}catch{/* browser preview has no window chrome */}}
 async function showMenu(){if(hoverTimer.current)window.clearTimeout(hoverTimer.current);setHoverMenu(true);await placeMenu(true);}
 function hideMenuSoon(){if(hoverTimer.current)window.clearTimeout(hoverTimer.current);hoverTimer.current=window.setTimeout(()=>{setHoverMenu(false);void placeMenu(false);},180);}
 async function openPanel(toMeet:boolean){if(toMeet)setTab('Meet');setHoverMenu(false);await placeMenu(false);if(!expanded)await toggle();}
 async function startListening(){setListenError('');if(!('__TAURI_INTERNALS__' in window)){setListenError('Listening lives in the desktop app.');return;}setListenBusy(true);try{const status=await invoke<{heard:string}>('start_meeting');setHeard(status.heard);setListening(true);setSince(Date.now());setHoverMenu(false);await placeMenu(false);}catch(e){const message=String(e);if(message.includes('already listening')){setListening(true);setSince(s=>s??Date.now());setHeard("I'm still listening.");setHoverMenu(false);await placeMenu(false);}else setListenError(message);}finally{setListenBusy(false);}}
 async function stopListening(){setListenError('');setSince(null);setListenBusy(true);setTab('Meet');setHoverMenu(false);await placeMenu(false);if(!expanded)await toggle();try{const note=await invoke<MeetingNote>('stop_meeting');setMeetingNotes(items=>[note,...items.filter(item=>item.id!==note.id)].slice(0,20));setListening(false);setHeard('');setSince(null);}catch(e){setListenError(String(e));setListening(false);setHeard('');setSince(null);}finally{setListenBusy(false);}}
 async function drag(){try{await invoke('begin_drag');}catch(e){setError(String(e));}}
 async function pumpMove(){if(moveBusy.current)return;moveBusy.current=true;try{while(moveQueue.current){const next=moveQueue.current;moveQueue.current=null;await invoke('move_widget',next);}}finally{moveBusy.current=false;}}
 function queueBubble(drag:{originX:number;originY:number;scale:number;startX:number;startY:number;lastX:number;lastY:number}){const dx=(drag.lastX-drag.startX)*drag.scale;const dy=(drag.lastY-drag.startY)*drag.scale;moveQueue.current={x:Math.round(drag.originX+dx),y:Math.round(drag.originY+dy)};void pumpMove();}
 async function onBubbleDown(e:React.PointerEvent<HTMLButtonElement>){if(e.button!==0)return;e.preventDefault();e.currentTarget.setPointerCapture(e.pointerId);const session={pointerId:e.pointerId,startX:e.screenX,startY:e.screenY,lastX:e.screenX,lastY:e.screenY,originX:0,originY:0,scale:1,moved:false,ready:false};bubbleDrag.current=session;try{const origin=await invoke<{x:number;y:number;scale:number}>('widget_origin');if(bubbleDrag.current!==session)return;session.originX=origin.x;session.originY=origin.y;session.scale=origin.scale;session.ready=true;if(Math.hypot(session.lastX-session.startX,session.lastY-session.startY)>=5){session.moved=true;queueBubble(session);}}catch(err){if(bubbleDrag.current!==session)return;const moved=session.moved;bubbleDrag.current=null;if(!moved)void toggle();else setError(String(err));}}
 function onBubbleMove(e:React.PointerEvent<HTMLButtonElement>){const drag=bubbleDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;drag.lastX=e.screenX;drag.lastY=e.screenY;if(Math.hypot(e.screenX-drag.startX,e.screenY-drag.startY)<5)return;drag.moved=true;if(!drag.ready)return;queueBubble(drag);}
 function onBubbleUp(e:React.PointerEvent<HTMLButtonElement>){const drag=bubbleDrag.current;if(!drag||e.pointerId!==drag.pointerId)return;bubbleDrag.current=null;if(!drag.moved)void openPanel(listening);}
 async function notify(){try{await invoke('test_notification',{body:'Your floating engineering assistant is ready. (Demo notification)'});}catch(e){setError(String(e));}}

 return <div className={`shell ${expanded?'expanded':'collapsed'}${frameMode==='desk'?' desk':''}${frameMode==='full'?' full':''}${night?' night':''}`}>
  {!expanded ? <div className={`dock${hoverMenu?` open ${menuCorner}`:''}`} onMouseEnter={()=>void showMenu()} onMouseLeave={hideMenuSoon}>
    {hoverMenu&&(menuCorner==='br'||menuCorner==='bl')&&<div className="listen-menu" onMouseEnter={()=>void showMenu()}>
     <button type="button" className="listen" disabled={listenBusy} onClick={()=>void (listening?stopListening():startListening())}>{listening?'Stop listening':'Start listening'}</button>
     {listenError&&<p className="listen-error">{listenError}</p>}
    </div>}
    <button className={`bubble${listening?' listening':''}`} onPointerDown={onBubbleDown} onPointerMove={onBubbleMove} onPointerUp={onBubbleUp} onContextMenu={e=>e.preventDefault()} title="Drag to a corner. Hover for listening." aria-label="Copilot"><Teddy phones={listening}/><span className="signal"/></button>
    {hoverMenu&&(menuCorner==='tr'||menuCorner==='tl')&&<div className="listen-menu" onMouseEnter={()=>void showMenu()}>
     <button type="button" className="listen" disabled={listenBusy} onClick={()=>void (listening?stopListening():startListening())}>{listening?'Stop listening':'Start listening'}</button>
     {listenError&&<p className="listen-error">{listenError}</p>}
    </div>}
   </div> :
  <div className="panel">
   <div className="topbar" onMouseDown={e=>{if(e.button===0 && e.target===e.currentTarget)void drag();}}>
    <span className="logo"><Teddy small phones={listening}/></span><div className="heading"><strong>Copilot</strong><small>{frameMode==='full'?'Full screen':frameMode==='desk'?'Drag any edge to resize':'Your desk companion'}</small></div>
    <button className={night?'iconbtn on':'iconbtn'} aria-pressed={night} title={night?'Switch to day':'Switch to night'} onClick={()=>setNight(v=>!v)}>{night?'☀':'☾'}</button>
    {frameMode!=='desk'&&<button className="iconbtn" title={frameMode==='full'?'Leave full screen and resize':'Open a window you can resize'} onClick={()=>void setFrame('desk')}><IconResize/></button>}
    {frameMode!=='full'&&<button className="iconbtn" title="Full screen" onClick={()=>void setFrame('full')}><IconFull/></button>}
    {frameMode!=='widget'&&<button className="iconbtn" title="Back to the small widget" onClick={()=>void setFrame('widget')}><IconWidget/></button>}
    <button className="iconbtn" title="Drag window" onMouseDown={e=>{e.preventDefault();void drag();}}>⠿</button>
    <button className="iconbtn" title="Minimize to floating bubble" onClick={()=>void (frameMode==='desk'||frameMode==='full'?setFrame('bubble'):toggle())}>−</button>
   </div>
   <div className="welcome"><div><span className="status"/> Ready when you are</div><button className="notify" onClick={notify}>Say hello</button></div>
   {tab!=='Brief'&&tab!=='Tickets'&&tab!=='PRs'&&tab!=='Meet'&&<div className="greeting"><small>TODAY</small><h1>Hello there</h1><p>Jira, Slack, and GitHub will show up here when you connect them.</p></div>}
   {tab==='Brief'&&<div className="stats"><div><b>{demoCount}</b><small>Sample updates</small></div><div><b>2</b><small>Sample tickets</small></div><div><b>2</b><small>Sample PRs</small></div></div>}
   <div className="tabs">{(['Brief','Tickets','PRs','Meet','Task Flow','Memory','AI Setup'] as Tab[]).map(t=><button className={tab===t?'selected':''} onClick={()=>setTab(t)} key={t}>{t}</button>)}</div>
   <div className="content">
   {tab==='Brief'&& <><div className="sectionhead">Today's briefing <span>DEMO</span></div><div className="item"><b>● Jira: comment requires attention</b><p>ENG-142 · A teammate requested clarification.</p></div><div className="item"><b>◆ GitHub: PR awaiting review</b><p>#284 · Authentication middleware has new changes.</p></div><div className="item"><b>◌ Slack: conversation summary</b><p>No account connected yet. Phase 2 will add live events.</p></div></>}
   {tab==='Tickets'&& <><div className="sectionhead">Jira tickets <span>DEMO DATA</span></div>{tickets.map(x=><div className="item" key={x.id}><span className="code">{x.id} · {x.tag}</span><b>{x.title}</b><p>{x.detail}</p></div>)}</>}
   {tab==='PRs'&&<PullRequests/>}
   {tab==='Meet'&&<Meetings notes={meetingNotes} live={listening} heard={heard} since={since} busy={listenBusy} problem={listenError} onListen={()=>void (listening?stopListening():startListening())}/>}
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
  {frameMode==='desk'&&<>
   <div className="grip n" onPointerDown={e=>void onEdgeDown('n',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip s" onPointerDown={e=>void onEdgeDown('s',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip e" onPointerDown={e=>void onEdgeDown('e',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip w" onPointerDown={e=>void onEdgeDown('w',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip ne" onPointerDown={e=>void onEdgeDown('ne',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip nw" onPointerDown={e=>void onEdgeDown('nw',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip se" onPointerDown={e=>void onEdgeDown('se',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
   <div className="grip sw" onPointerDown={e=>void onEdgeDown('sw',e)} onPointerMove={onEdgeMove} onPointerUp={onEdgeUp}/>
  </>}
 </div>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
