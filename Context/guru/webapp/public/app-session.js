window.VAULT_ACCOUNT_PROMISE=(async()=>{
  if(document.readyState === "loading") await new Promise(resolve=>document.addEventListener("DOMContentLoaded",resolve,{once:true}));
  const response=await fetch('/api/app/me');
  if(response.status===401){location.replace('/login.html?next='+encodeURIComponent(location.pathname+location.search+location.hash));throw new Error('Please sign in.');}
  if(!response.ok)throw new Error('Could not load your account. Please reload.');
  const data=await response.json();window.VAULT_USER=data.user;
  const style=document.createElement('style');style.textContent='.account-tools{display:flex;gap:10px;align-items:center;font-size:12px;white-space:nowrap}.account-tools button{background:transparent;border:1px solid var(--steel);padding:4px 9px;cursor:pointer}.account-tools a{color:var(--blue)}.account-msg{position:fixed;bottom:18px;right:18px;z-index:100;background:var(--card);color:var(--ink);border:1px solid var(--steel);padding:12px 18px;max-width:360px;font-size:13px}@media(max-width:850px){.account-tools .account-name{display:none}.topbar{gap:12px!important;padding:0 16px!important;flex-wrap:wrap;height:auto!important;min-height:58px}.account-tools{margin-left:auto}}';document.head.append(style);
  const tools=document.createElement('div');tools.className='account-tools';
  const name=document.createElement('span');name.className='account-name';name.textContent=data.user.name+' · '+data.user.role;tools.append(name);
  if(data.user.role==='admin'){const link=document.createElement('a');link.href='/admin.html';link.textContent='Accounts';tools.append(link);}
  const button=document.createElement('button');button.type='button';button.textContent='Sign out';tools.append(button);document.querySelector('.topbar').append(tools);
  const notify=text=>{document.querySelector('.account-msg')?.remove();const el=document.createElement('div');el.className='account-msg';el.setAttribute('role','status');el.textContent=text;document.body.append(el);setTimeout(()=>el.remove(),6000);};
  button.onclick=async()=>{button.disabled=true;try{const r=await fetch('/api/auth/sign-out',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'});if(!r.ok)throw new Error();location.replace('/login.html');}catch{button.disabled=false;notify('Could not sign out. Please try again.');}};
  window.VAULT_SETTINGS=data.settings;
  if(!location.hash && data.settings.lastView) history.replaceState(null,'',location.pathname+location.search+data.settings.lastView);
  let viewTimer;addEventListener('hashchange',()=>{clearTimeout(viewTimer);viewTimer=setTimeout(async()=>{try{const r=await fetch('/api/app/settings',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({lastView:location.hash||'#/'})});if(!r.ok)throw new Error();}catch{notify('Could not save your current view.');}},500);});
  document.addEventListener('click',async event=>{const theme=event.target.closest('#themes button[data-theme]')?.dataset.theme;if(!theme)return;try{const r=await fetch('/api/app/settings',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({theme})});if(!r.ok)throw new Error();}catch{notify('Theme changed on this device; saving to your account failed.');}});
  return data;
})();
