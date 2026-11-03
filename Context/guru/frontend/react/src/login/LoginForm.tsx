import {useEffect,useRef,useState} from 'react';
import {Icon} from './components/Icon.jsx';
import './loginForm.css';
export interface Credentials {email:string;password:string}
export default function LoginForm({autoFocus=false,onSignIn}:{autoFocus?:boolean;onSignIn:(credentials:Credentials)=>Promise<void>}) {
 const [email,setEmail]=useState(''),[password,setPassword]=useState(''),[visible,setVisible]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const alive=useRef(true),pending=useRef(false),passwordRef=useRef<HTMLInputElement>(null);
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;};},[]);
 useEffect(()=>{if(error&&!busy)passwordRef.current?.focus();},[error,busy]);
 async function submit(event:React.FormEvent<HTMLFormElement>) {
  event.preventDefault();if(pending.current || !event.currentTarget.reportValidity())return;
  pending.current=true;setBusy(true);setError('');
  try {await onSignIn({email:email.trim(),password});if(alive.current)setPassword('');}
  catch(e) {if(alive.current){setError(e instanceof Error?e.message:'Could not sign in. Please try again.');passwordRef.current?.focus();}}
  finally {pending.current=false;if(alive.current)setBusy(false);}
 }
 return <form id="login-form" className={`lf${error?' has-error':''}`} onSubmit={submit} aria-labelledby="lf-title" aria-busy={busy}>
  <h1 id="lf-title" className="lf-title">Sign in</h1>
  <p className="lf-sub">Your private strategy research workspace.</p>
  <div className="lf-field"><label htmlFor="email">Username</label><div className="lf-control"><span className="lf-icon"><Icon name="user"/></span>
   <input id="email" name="email" type="text" autoComplete="username" autoCapitalize="none" spellCheck={false} placeholder="Username or email" required autoFocus={autoFocus} disabled={busy} value={email} aria-describedby="login-error" onChange={e=>{setEmail(e.target.value);setError('');}}/>
  </div></div>
  <div className="lf-field"><label htmlFor="password">Password</label><div className="lf-control"><span className="lf-icon"><Icon name="lock"/></span>
   <input id="password" ref={passwordRef} name="password" type={visible?'text':'password'} autoComplete="current-password" placeholder="Enter your password" required disabled={busy} value={password} aria-describedby="login-error" onChange={e=>{setPassword(e.target.value);setError('');}}/>
   <button id="password-toggle" type="button" className="lf-eye" disabled={busy} aria-label={visible?'Hide password':'Show password'} aria-controls="password" aria-pressed={visible} onClick={()=>setVisible(v=>!v)}><Icon name="eye"/><span className={`lf-strike${visible?' is-visible':''}`}/></button>
  </div></div>
  <button id="submit" type="submit" className="lf-submit" disabled={busy}>{busy?'Signing in…':'Sign in'}</button>
  <p id="login-error" className={`lf-msg${error?' is-error':''}`} role={error?'alert':'status'} aria-live="polite">{error}</p>
  <p className="lf-access">Need access? Contact your workspace administrator.</p>
 </form>;
}
