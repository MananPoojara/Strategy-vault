import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {it,expect,vi,afterEach} from 'vitest';
import {useSceneEnvironment} from '../src/login/useSceneEnvironment';
import IncenseStand from '../src/login/components/landing/IncenseStand.jsx';
let root:Root|undefined,host:HTMLDivElement|undefined;
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;vi.restoreAllMocks();vi.unstubAllGlobals();});
async function mount(node:React.ReactNode){Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});host=document.createElement('div');document.body.append(host);root=createRoot(host);await act(async()=>root!.render(node));}
it('responds to tab visibility and dynamic reduced motion, then removes both listeners',async()=>{
 const listeners=new Set<()=>void>(),media={matches:false,addEventListener:vi.fn((_:string,fn:()=>void)=>listeners.add(fn)),removeEventListener:vi.fn((_:string,fn:()=>void)=>listeners.delete(fn))};vi.stubGlobal('matchMedia',()=>media);
 const remove=vi.spyOn(document,'removeEventListener');const hidden=vi.spyOn(document,'hidden','get').mockReturnValue(false);
 function Probe(){const value=useSceneEnvironment();return <output>{JSON.stringify(value)}</output>}
 await mount(<Probe/>);expect(host!.textContent).toContain('"visible":true');
 await act(async()=>{hidden.mockReturnValue(true);document.dispatchEvent(new Event('visibilitychange'));media.matches=true;listeners.forEach(fn=>fn());});
 expect(host!.textContent).toContain('"visible":false');expect(host!.textContent).toContain('"reduced":true');
 await act(async()=>root!.unmount());root=undefined;expect(listeners.size).toBe(0);expect(remove).toHaveBeenCalledWith('visibilitychange',expect.any(Function));
});
it('cancels active smoke on pause/reduced-motion/unmount and does not spin while static',async()=>{
 const raf=vi.fn().mockReturnValue(41),cancel=vi.fn();vi.stubGlobal('requestAnimationFrame',raf);vi.stubGlobal('cancelAnimationFrame',cancel);
 const ctx={setTransform(){},clearRect(){},beginPath(){},moveTo(){},lineTo(){},stroke(){}};vi.spyOn(HTMLCanvasElement.prototype,'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
 await mount(<IncenseStand active={true} reduced={false}/>);expect(raf).toHaveBeenCalledOnce();
 await act(async()=>root!.render(<IncenseStand active={false} reduced={false}/>));expect(cancel).toHaveBeenCalledWith(41);expect(raf).toHaveBeenCalledOnce();
 await act(async()=>root!.render(<IncenseStand active={true} reduced={true}/>));expect(raf).toHaveBeenCalledOnce();
});
it('keeps the login decoration harmless when canvas drawing is unavailable',async()=>{
 const raf=vi.fn();vi.stubGlobal('requestAnimationFrame',raf);vi.spyOn(HTMLCanvasElement.prototype,'getContext').mockReturnValue(null);
 await mount(<IncenseStand active={true} reduced={false}/>);expect(host!.querySelector('svg')).not.toBeNull();expect(raf).not.toHaveBeenCalled();
});
