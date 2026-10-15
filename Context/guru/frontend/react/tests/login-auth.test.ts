import {describe,it,expect,vi} from 'vitest';
import {safeLoginNext,signInEmail} from '../src/login/auth';
describe('login return routing',()=>{
 const origin='http://vault.test';
 it.each(['https://evil.invalid/','javascript:alert(1)','//evil.invalid/','/\\evil.invalid','/path\\x','/path\nx','/login.html','/nested/../login.html?next=/','/%6cogin.html','/%2f%2fevil.invalid','/%5cevil.invalid','/%0alogin','/LOGIN.html'])('rejects unsafe or looping return %s',value=>expect(safeLoginNext(value,'',origin)).toBe('/'));
 it('retains real hash routes, query and safe root-relative paths',()=>{
  expect(safeLoginNext('/?q=book','#/s/dmi_1',origin)).toBe('/?q=book#/s/dmi_1');
  expect(safeLoginNext('/#/s/dmi_1?tk=NIFTY%2050','#ignored',origin)).toBe('/#/s/dmi_1?tk=NIFTY%2050');
  expect(safeLoginNext('/#/strategies?q=50%25','',origin)).toBe('/#/strategies?q=50%25');
  expect(safeLoginNext('/?q=50%25','',origin)).toBe('/?q=50%25');
  expect(safeLoginNext('/malformed%','',origin)).toBe('/');
  expect(safeLoginNext('/%25250a','',origin)).toBe('/');
  expect(safeLoginNext('/admin.html','',origin)).toBe('/admin.html');
 });
});
describe('real email authentication contract',()=>{
 it('posts trimmed email and unmodified password with same-origin cookies and no credential persistence',async()=>{
  const fetcher=vi.fn().mockResolvedValue(new Response('{}',{status:200}));
  await signInEmail('  test@example.invalid  ','  correct password  ',fetcher);
  expect(fetcher).toHaveBeenCalledOnce();
  const [url,init]=fetcher.mock.calls[0];expect(url).toBe('/api/auth/sign-in/email');expect(init.credentials).toBe('same-origin');expect(init.method).toBe('POST');expect(JSON.parse(init.body)).toEqual({email:'test@example.invalid',password:'  correct password  '});
 });
 it.each([401,403,429,500])('does not accept a failed auth response (%s) or expose its server body',async status=>{
  const fetcher=vi.fn().mockResolvedValue(new Response('private server stack',{status}));
  await expect(signInEmail('test@example.invalid','wrong',fetcher)).rejects.toThrow(status===429?'Too many attempts':'Could not sign in');
 });
 it('reports a network interruption with a safe retry message',async()=>{
  const fetcher=vi.fn().mockRejectedValue(new Error('private host info'));
  await expect(signInEmail('test@example.invalid','wrong',fetcher)).rejects.toThrow('Connection interrupted');
 });
});
