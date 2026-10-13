import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
const envFile=process.env.VAULT_ENV_FILE||`${homedir()}/.cache/strategy-vault-webapp/.env`;
const env=Object.fromEntries((await readFile(envFile,'utf8')).split('\n').filter(s=>s&&!s.startsWith('#')).map(s=>{const i=s.indexOf('=');return[s.slice(0,i),s.slice(i+1)];}));
await test('login throttling cannot be bypassed by forging the trusted client-IP header',async()=>{
 const statuses=[];
 for(let i=0;i<12;i++){
  const response=await fetch(env.VAULT_PUBLIC_URL+'/api/auth/sign-in/email',{method:'POST',headers:{Origin:env.VAULT_PUBLIC_URL,'Content-Type':'application/json','x-vault-client-ip':`198.51.100.${i+1}`},body:JSON.stringify({email:env.VAULT_ADMIN_EMAIL,password:'wrong-password-123'})});
  await response.text();statuses.push(response.status);
 }
 assert.ok(statuses.includes(429),'Repeated attempts should receive HTTP 429 even when the caller changes the spoofed IP header');
 assert.equal(statuses.at(-1),429);
});
