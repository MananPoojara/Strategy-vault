import {afterEach,it,expect,vi} from 'vitest';
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import LoginForm from '../src/login/LoginForm';
let root:Root,host:HTMLDivElement;
afterEach(async()=>{await act(async()=>root?.unmount());host?.remove();});
async function mount(onSignIn:(x:{email:string,password:string})=>Promise<void>){
 Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});host=document.createElement('div');document.body.append(host);root=createRoot(host);await act(async()=>root.render(<LoginForm onSignIn={onSignIn}/>));
}
async function input(id:string,value:string){const el=host.querySelector<HTMLInputElement>('#'+id)!;await act(async()=>{Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(el,value);el.dispatchEvent(new Event('input',{bubbles:true}));});}
async function submit(){await act(async()=>host.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));}
it('uses email, current-password and a real submit form, without demo signup or stored-credential controls',async()=>{
 await mount(vi.fn());expect(host.querySelector('#email')?.getAttribute('type')).toBe('email');expect(host.querySelector('#password')?.getAttribute('autocomplete')).toBe('current-password');expect(host.textContent).toContain('workspace administrator');expect(host.textContent).not.toMatch(/preview|sign up|remember me/i);expect(host.querySelector('#password')?.hasAttribute('minlength')).toBe(false);
});
it('reveals and hides the password with an accessible non-submit button',async()=>{
 const signIn=vi.fn();await mount(signIn);await input('password','keep exact');const b=host.querySelector<HTMLButtonElement>('#password-toggle')!;await act(async()=>b.click());expect(host.querySelector('#password')?.getAttribute('type')).toBe('text');expect(b.getAttribute('aria-pressed')).toBe('true');expect(b.getAttribute('aria-label')).toBe('Hide password');await act(async()=>b.click());expect(host.querySelector('#password')?.getAttribute('type')).toBe('password');expect(signIn).not.toHaveBeenCalled();
});
it('blocks double submits while pending, exposes an error, and permits retry without saving the password',async()=>{
 let reject:(e:Error)=>void=()=>{};const first=new Promise<void>((_,r)=>{reject=r;});const signIn=vi.fn().mockReturnValueOnce(first).mockResolvedValueOnce(undefined);await mount(signIn);await input('email','research@example.invalid');await input('password','brief123');await submit();await submit();expect(signIn).toHaveBeenCalledOnce();expect(host.querySelector<HTMLButtonElement>('#submit')?.disabled).toBe(true);await act(async()=>{reject(new Error('Could not sign in.'));await first.catch(()=>{});});expect(host.querySelector('[role="alert"]')?.textContent).toBe('Could not sign in.');expect(document.activeElement).toBe(host.querySelector('#password'));await submit();expect(signIn).toHaveBeenCalledTimes(2);expect(host.querySelector<HTMLInputElement>('#password')?.value).toBe('');
});
