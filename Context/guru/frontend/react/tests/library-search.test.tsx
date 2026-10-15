import {afterEach, expect, it, vi} from 'vitest';
import {act} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {LibrarySearch} from '../src/components/LibraryControls';
import {matchingSearchAuthors} from '../src/services/library-search';

const authors = [
  {id:'williams', name:'Larry Williams', photo:'img/gurus/williams.jpg', initials:'LW', href:'#/guru/williams'},
  {id:'connors', name:'Larry Connors', photo:'img/gurus/connors.jpg', initials:'LC', href:'#/guru/connors'},
  {id:'ehlers', name:'John Ehlers', photo:'', initials:'JE', href:'#/guru/ehlers'},
];
let root: Root | undefined, host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root=undefined; host=undefined; vi.useRealTimers();
});
async function mount(query='') {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT:true});
  const patch=vi.fn();
  window.VaultRuntime={patch} as unknown as Window['VaultRuntime'];
  host=document.createElement('div'); document.body.append(host); root=createRoot(host);
  await act(async () => root!.render(<LibrarySearch query={query} authors={authors}/>));
  return {patch, input:host.querySelector<HTMLInputElement>('#library-query')!};
}
async function type(input:HTMLInputElement, value:string) {
  await act(async () => {
    input.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!.call(input,value);
    input.dispatchEvent(new Event('input',{bubbles:true}));
  });
}
async function key(input:HTMLInputElement, key:string) {
  await act(async () => input.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true})));
}
it('matches author names immediately, with prefix ranking, accent handling and a bounded list', () => {
  expect(matchingSearchAuthors(authors,' larr ').map(a=>a.id)).toEqual(['connors','williams']);
  expect(matchingSearchAuthors(authors,'CONN').map(a=>a.id)).toEqual(['connors']);
  expect(matchingSearchAuthors(authors,'')).toEqual([]);
  expect(matchingSearchAuthors(authors,'missing')).toEqual([]);
  expect(matchingSearchAuthors([{...authors[0],name:'José García'}],'jose')).toHaveLength(1);
  expect(matchingSearchAuthors(Array.from({length:20},(_,i)=>({...authors[0],id:String(i)})),'Larry')).toHaveLength(6);
});
it('shows local circular portrait options without starting research requests while typing',async()=>{
  vi.useFakeTimers();
  const {input,patch}=await mount(); await type(input,'larr');
  expect(input.getAttribute('role')).toBe('combobox');
  expect(input.getAttribute('aria-expanded')).toBe('true');
  const options=[...host!.querySelectorAll('[role=option]')];
  expect(options.map(o=>o.textContent)).toEqual(expect.arrayContaining([expect.stringContaining('Larry Connors'),expect.stringContaining('Larry Williams')]));
  expect(options[0].querySelector('img')?.getAttribute('src')).toBe('img/gurus/connors.jpg');
  expect(options[0].getAttribute('href')).toBe('#/guru/connors');
  await act(async()=>vi.advanceTimersByTime(1000)); expect(patch).not.toHaveBeenCalled();
});
it('supports keyboard selection, Escape, reopen, and Tab without trapping focus',async()=>{
  const {input}=await mount(); await type(input,'larr'); await key(input,'ArrowDown');
  const selected=document.getElementById(input.getAttribute('aria-activedescendant')!);
  expect(selected?.getAttribute('aria-selected')).toBe('true'); expect(selected?.textContent).toContain('Larry Connors');
  await key(input,'ArrowUp'); expect(host!.querySelector('[aria-selected=true]')?.textContent).toContain('Larry Williams');
  await key(input,'Escape'); expect(input.getAttribute('aria-expanded')).toBe('false');
  await key(input,'ArrowDown'); expect(input.getAttribute('aria-expanded')).toBe('true');
  await key(input,'Tab'); expect(input.getAttribute('aria-expanded')).toBe('false');
});
it('submits free text for books and authors and does not fabricate author matches',async()=>{
  const {input,patch}=await mount(); await type(input,'  Swing Trading  ');
  expect(host!.querySelectorAll('[role=option]')).toHaveLength(0);
  expect(host!.textContent).toContain('No matching authors');
  await act(async()=>host!.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));
  expect(patch).toHaveBeenCalledExactlyOnceWith({library_q:'Swing Trading',library_limit:'24',library_page:'1',source_page:'1'});
  expect(input.getAttribute('aria-expanded')).toBe('false');
});
it('falls back to initials if a portrait cannot load and dismisses suggestions on outside blur',async()=>{
  const {input}=await mount(); await type(input,'larr');
  const img=host!.querySelector('img')!;
  await act(async()=>img.dispatchEvent(new Event('error')));
  expect(host!.querySelector('[role=option]')!.querySelector('img')).toBeNull();
  expect(host!.querySelector('[role=option] .search-avatar')?.textContent).toBe('LC');
  await act(async()=>input.blur()); expect(input.getAttribute('aria-expanded')).toBe('false');
});

it('Enter opens the highlighted author while ordinary Enter remains a full search',async()=>{
  const {input,patch}=await mount(); await type(input,'larr'); await key(input,'ArrowDown');
  const before=window.location.hash;
  await key(input,'Enter');
  expect(window.location.hash).toBe('#/guru/connors');
  expect(patch).not.toHaveBeenCalled();
  window.location.hash=before;
});

it('keeps curated author matches ahead of similarly named unreviewed source credits',()=>{
  const credits=Array.from({length:10},(_,i)=>({...authors[0],id:'credit'+i,name:'Larry A '+i,curated:false}));
  const curated=authors.slice(0,2).map(a=>({...a,curated:true}));
  expect(matchingSearchAuthors([...credits,...curated],'larr').slice(0,2).map(a=>a.id)).toEqual(['connors','williams']);
});

it('Escape suppresses the native search-input clear action when dismissing suggestions',async()=>{
  const {input}=await mount(); await type(input,'larr');
  const escape=new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true});
  await act(async()=>input.dispatchEvent(escape));
  expect(escape.defaultPrevented).toBe(true);expect(input.value).toBe('larr');
});

it('does not reopen suggestions when the runtime restores focus after submitting that query',async()=>{
  const {input}=await mount('Larry');
  await act(async()=>input.focus());
  expect(input.getAttribute('aria-expanded')).toBe('false');
  await key(input,'ArrowDown');expect(input.getAttribute('aria-expanded')).toBe('true');
});

it('keeps a curated surname match visible ahead of unreviewed prefix matches',()=>{
  const credits=Array.from({length:8},(_,i)=>({...authors[0],id:'credit'+i,name:'Ehlers credit '+i,curated:false}));
  const john={...authors[2],curated:true};
  expect(matchingSearchAuthors([...credits,john],'Ehlers')[0].id).toBe('ehlers');
});
