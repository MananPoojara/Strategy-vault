import {useEffect,useState} from 'react';
export function useSceneEnvironment(){
 const [visible,setVisible]=useState(()=>!document.hidden);
 const [reduced,setReduced]=useState(()=>window.matchMedia?.('(prefers-reduced-motion: reduce)').matches??false);
 useEffect(()=>{
  const media=window.matchMedia?.('(prefers-reduced-motion: reduce)'),motion=()=>setReduced(media?.matches??false),visibility=()=>setVisible(!document.hidden);
  media?.addEventListener('change',motion);document.addEventListener('visibilitychange',visibility);
  return()=>{media?.removeEventListener('change',motion);document.removeEventListener('visibilitychange',visibility);};
 },[]);
 return {visible,reduced};
}
