const requestedNext = new URLSearchParams(location.search).get('next') || '/';
const nextValue = requestedNext + (!requestedNext.includes('#') ? (location.hash || '') : '');
const next = nextValue.startsWith('/') && !nextValue.startsWith('//') && !/[\\\x00-\x1f\x7f]/.test(nextValue) && !nextValue.startsWith('/login') ? nextValue : '/';
fetch('/api/app/me').then(r => { if (r.ok) location.replace(next); }).catch(() => {});
document.querySelector('#login-form').addEventListener('submit', async event => {
  event.preventDefault(); const button=document.querySelector('#submit'), message=document.querySelector('#message');
  const buttonContent=button.innerHTML;
  button.disabled=true; button.setAttribute('aria-busy','true'); button.textContent='Signing in…'; message.textContent='';
  try {
    const response=await fetch('/api/auth/sign-in/email',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:document.querySelector('#email').value.trim(),password:document.querySelector('#password').value})});
    if(!response.ok) throw new Error(response.status===429?'Too many attempts. Please wait a minute and try again.':'Could not sign in. Check your email and password.');
    document.querySelector('#password').value=''; location.replace(next);
  } catch(error) { message.textContent=error.message; button.disabled=false; button.removeAttribute('aria-busy'); button.innerHTML=buttonContent; }
});

const passwordToggle=document.querySelector('#password-toggle');
passwordToggle?.addEventListener('click', () => {
  const password=document.querySelector('#password');
  const visible=password.type==='password';
  password.type=visible?'text':'password';
  passwordToggle.setAttribute('aria-pressed',String(visible));
  passwordToggle.setAttribute('aria-label',visible?'Hide password':'Show password');
});

const workflowInsights={
  library:'Start with the original idea. Keep the reasoning behind every rule.',
  rules:'Make the idea precise: entry, exit and filters you can test consistently.',
  evidence:'Look beyond a return. Study the trades, drawdowns and market context.'
};
document.querySelectorAll('[data-step]').forEach(step => {
  step.addEventListener('click', () => {
    const stage=step.dataset.step;
    if(!Object.hasOwn(workflowInsights,stage)) return;
    document.querySelectorAll('[data-step]').forEach(item => {
      const selected=item===step;
      item.classList.toggle('is-active',selected);
      item.setAttribute('aria-pressed',String(selected));
    });
    document.querySelector('.research-scene').dataset.stage=stage;
    document.querySelector('#workflow-insight').textContent=workflowInsights[stage];
  });
});
