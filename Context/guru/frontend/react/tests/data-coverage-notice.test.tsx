import {afterEach, expect, it, vi} from 'vitest';
import {act} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {DataCoverageNotice} from '../src/components/DataCoverageNotice';
let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;});
it('explains missing bars and makes recovery an explicit action without replacing the stock', async()=>{
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  const patch=vi.fn();window.VaultRuntime={patch} as unknown as Window['VaultRuntime'];
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async()=>root!.render(<DataCoverageNotice issues={[{owner:'connors',kind:'empty_range',status:'needs_data',note:'No price bars are available for AAPL. Available history: 2000-01-03 to 2020-09-11.'}]} />));
  expect(host.querySelector('[data-summary-unavailable]')?.textContent).toContain('2020-09-11');
  expect(patch).not.toHaveBeenCalled();
  await act(async()=>host!.querySelector<HTMLButtonElement>('[data-summary-reset-dates]')!.click());
  expect(patch).toHaveBeenCalledWith({from:'',to:'',tab:'sharpe',library_page:'1'});
});
it('keeps a request failure visible without suggesting that changing dates repairs it',async()=>{
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async()=>root!.render(<DataCoverageNotice issues={[{owner:'chande',kind:'request_failed',status:'unavailable',note:'Some backtests could not be loaded for Nifty 50.'}]} />));
  expect(host.textContent).toContain('could not be loaded');
  expect(host.querySelector('[data-summary-reset-dates]')).toBeNull();
});
it('does not show a missing-data notice for successful requests',async()=>{
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async()=>root!.render(<DataCoverageNotice issues={[]} />));
  expect(host.innerHTML).toBe('');
});
