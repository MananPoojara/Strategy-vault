import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const dir=fileURLToPath(new URL('..',import.meta.url));
const envFile=process.env.VAULT_ENV_FILE||`${homedir()}/.cache/strategy-vault-webapp/.env`;
const env=Object.fromEntries((await readFile(envFile,'utf8')).split('\n').filter(s=>s&&!s.startsWith('#')).map(s=>{const i=s.indexOf('=');return[s.slice(0,i),s.slice(i+1)];}));
const base=env.VAULT_PUBLIC_URL;
const dc=['compose','--project-directory',dir,'--env-file',envFile];
const key='restart.'+randomBytes(5).toString('hex');
await test('conclusions, account settings and sessions survive app/database restart',async()=>{
 const login=await fetch(base+'/api/auth/sign-in/email',{method:'POST',headers:{Origin:base,'Content-Type':'application/json'},body:JSON.stringify({email:env.VAULT_ADMIN_EMAIL,password:env.VAULT_ADMIN_PASSWORD})});
 assert.equal(login.status,200,await login.clone().text());
 const cookie=login.headers.getSetCookie().map(v=>v.split(';')[0]).join('; '),headers={Cookie:cookie,Origin:base};
 const original=(await (await fetch(base+'/api/app/me',{headers})).json()).settings;
 try {
  assert.equal((await fetch(base+'/api/app/notes?key='+key,{method:'POST',headers:{...headers,'Content-Type':'text/plain'},body:'Persisted through service restart'})).status,200);
  const saved={theme:'lagoon',lastView:'#/o/f1a?region=in&tk=NIFTY%2050&from=2020-01-01&to=2025-12-31&cost=3&p=hold%3A6'};
  assert.equal((await fetch(base+'/api/app/settings',{method:'PUT',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify(saved)})).status,200);
  execFileSync('docker',[...dc,'restart','postgres','auth'],{stdio:'pipe'});
  let ready=false;
  for(let attempt=0;attempt<30;attempt++) {try{const response=await fetch(base+'/healthz');if(response.ok){ready=true;break;}}catch{}await new Promise(resolve=>setTimeout(resolve,1000));}
  assert.ok(ready,'App should recover after restarting its own database and auth service');
  const me=await fetch(base+'/api/app/me',{headers});assert.equal(me.status,200,'The original session must remain valid');
  assert.deepEqual((await me.json()).settings,saved);
  assert.equal(await (await fetch(base+'/api/app/notes?key='+key,{headers})).text(),'Persisted through service restart');
 } finally {
  await fetch(base+'/api/app/settings',{method:'PUT',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify(original)});
  execFileSync('docker',[...dc,'exec','-T','postgres','psql','-U','vault','-d','vault','-v','ON_ERROR_STOP=1','-c',`DELETE FROM vault_note_history WHERE note_key='${key}'; DELETE FROM vault_notes WHERE note_key='${key}'; DELETE FROM vault_audit WHERE entity='${key}';`],{stdio:'pipe'});
  await fetch(base+'/api/auth/sign-out',{method:'POST',headers:{...headers,'Content-Type':'application/json'},body:'{}'});
 }
});
