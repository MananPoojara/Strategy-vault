import './base.css';
import {useEffect,useRef} from 'react';
import WelcomeScene from './WelcomeScene.jsx';
import {safeLoginNext,signInEmail} from './auth';
import type {Credentials} from './LoginForm';
export default function LoginPage({navigate=(url:string)=>window.location.replace(url),fetcher=fetch}:{navigate?:(url:string)=>void;fetcher?:typeof fetch}) {
 const next=useRef(safeLoginNext(new URLSearchParams(window.location.search).get('next'),window.location.hash));
 const controller=useRef<AbortController|null>(null);
 useEffect(()=>{
  const abort=new AbortController();controller.current=abort;
  fetcher('/api/app/me',{credentials:'same-origin',signal:abort.signal}).then(r=>{if(r.ok&&!abort.signal.aborted)navigate(next.current);}).catch(()=>{});
  return()=>{abort.abort();controller.current=null;};
 },[fetcher,navigate]);
 async function login({email,password}:Credentials){
  const signal=controller.current?.signal;
  await signInEmail(email,password,fetcher,signal);
  if(!signal?.aborted)navigate(next.current);
 }
 return <WelcomeScene onSignIn={login} audioEnabled={import.meta.env.VAULT_LOGIN_AUDIO_ENABLED==='true'}/>;
}
