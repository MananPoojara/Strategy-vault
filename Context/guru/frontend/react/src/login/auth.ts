/** Preserve same-origin research links without accepting an external redirect or login loop. */
export function safeLoginNext(raw: string | null, hash = '', origin = window.location.origin): string {
  const value=(raw || '/')+(!(raw || '/').includes('#') ? hash : '');
  let decoded=value;
  try {
    decodeURIComponent(value); // Reject malformed original escapes, but preserve valid literal percent signs.
    for(let i=0;i<4;i++) {
      if(!decoded.startsWith('/') || decoded.startsWith('//') || /[\\\u0000-\u001f\u007f]/.test(decoded)) return '/';
      const next=decodeURIComponent(decoded.replace(/%(?![0-9a-f]{2})/gi,'%25'));if(next===decoded)break;decoded=next;
    }
    if(!decoded.startsWith('/') || decoded.startsWith('//') || /[\\\u0000-\u001f\u007f]/.test(decoded))return '/';
    const url=new URL(value,origin),decodedURL=new URL(decoded,origin);
    if(url.origin!==origin || decodedURL.origin!==origin || /^\/(?:login|welcome)(?:[/.?#]|$)/i.test(decodedURL.pathname)) return '/';
    return url.pathname+url.search+url.hash;
  } catch {return '/';}
}
export async function signInEmail(email: string,password: string,fetcher: typeof fetch=fetch,signal?: AbortSignal): Promise<void> {
  let response: Response;
  try {
    response=await fetcher('/api/auth/sign-in/email',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:email.trim(),password}),signal});
  } catch(error) {
    if(signal?.aborted)throw error;
    throw new Error('Connection interrupted. Please try again.');
  }
  if(!response.ok)throw new Error(response.status===429?'Too many attempts. Please wait a minute and try again.':'Could not sign in. Check your email and password.');
}
