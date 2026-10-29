import {build} from 'vite';
import react from '@vitejs/plugin-react';
import {readFile,writeFile,mkdir,copyFile,stat,rm} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),out=resolve(root,'dist-login');
// Optional user-supplied short UI audio stays outside Git. No download or fallback recording.
const input=process.env.VAULT_LOGIN_AUDIO;
let audio=false;
if(input){try{const s=await stat(input);if(!s.isFile()||s.size>1024*1024)throw Error('Optional login audio must be a regular file no larger than1MiB.');const bytes=await readFile(input);if(bytes.subarray(0,4).toString()!=='OggS'||!bytes.subarray(0,128).includes(Buffer.from('OpusHead')))throw Error('Optional login audio must be an actual Ogg Opus file.');audio=true;}catch(error){if(error.code!=='ENOENT')throw error;}}
await build({configFile:false,root,plugins:[react()],publicDir:false,define:{'process.env.NODE_ENV':JSON.stringify('production'),'import.meta.env.VAULT_LOGIN_AUDIO_ENABLED':JSON.stringify(audio?'true':'false')},build:{outDir:out,emptyOutDir:true,lib:{entry:resolve(root,'src/login/main.tsx'),name:'VaultLogin',formats:['iife'],fileName:()=> 'login.js',cssFileName:'login'},minify:true}});
const generated=await readFile(resolve(out,'login.css'),'utf8'),shared=await readFile(resolve(root,'../../webapp/public/login.css'),'utf8');
// admin.html shares this stylesheet. Keep its existing base rules byte-for-byte.
await writeFile(resolve(out,'login.css'),shared+'\n/* React welcome/login styles, scoped to their own body and components. */\n'+generated);
await writeFile(resolve(out,'login.html'),'<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="theme-color" content="#FBF4EA"><title>Sign in · Strategy Vault</title><link rel="icon" href="data:,"><link rel="stylesheet" href="/login.css"><script src="/maintenance-watch.js" defer></script><script src="/login.js" defer></script></head><body class="vault-welcome"><div id="login-root"></div><noscript>JavaScript is required to sign in to Strategy Vault.</noscript></body></html>');
const dest=resolve(root,'dist/vendor/login/landing/aarti.ogg');
if(audio){await mkdir(dirname(dest),{recursive:true});await copyFile(input,dest);}else await rm(dest,{force:true});
