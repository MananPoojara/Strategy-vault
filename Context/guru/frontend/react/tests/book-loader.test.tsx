import {afterEach,expect,it,vi} from "vitest";
import {act} from "react";
import {createRoot,type Root} from "react-dom/client";
import {renderToStaticMarkup} from "react-dom/server";
import {BookLoader} from "../src/components/BookLoader";

vi.mock("../src/compat/runtime.js",()=>({}));
vi.mock("../src/services/api",()=>({getAccount:()=>new Promise(()=>{}),saveSettings:vi.fn(),ApiError:class extends Error{status=500;}}));
let root:Root|undefined,host:HTMLDivElement|undefined;
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;});
it("preserves the book covers, three animated pages and accessible loading status",()=>{
 const html=renderToStaticMarkup(<BookLoader message="Loading this book…"/>);
 expect(html).toContain('role="status"');expect(html).toContain('aria-live="polite"');
 expect(html).toContain('class="bflip"');expect(html).toContain('class="bcv l"');expect(html).toContain('class="bcv r"');
 expect(html.match(/class="bpg"/g)).toHaveLength(3);expect(html).toContain("Loading this book…");expect(html).toContain('aria-hidden="true"');
});
it("shows the book during initial and SPA loads, retains the old page and clears it on completion or error",async()=>{
 const {App}=await import("../src/app/App"),{vaultStore}=await import("../src/state/store");
 Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
 window.VaultRuntime={actions:()=>"",saved:()=>[],commit:vi.fn(),cleanup:vi.fn()} as unknown as Window["VaultRuntime"];
 host=document.createElement("div");document.body.append(host);root=createRoot(host);
 await act(async()=>root!.render(<App/>));
 expect(host.querySelectorAll("#main [data-book-loader]")).toHaveLength(1);
 expect(host.querySelectorAll("#flipov")).toHaveLength(0);expect(host.querySelector("#main")?.getAttribute("aria-busy")).toBe("true");
 const route={name:"author" as const,path:"/guru/connors",id:"connors",query:new URLSearchParams()};
 await act(async()=>{const ticket=vaultStore.begin(route.path);vaultStore.commit(ticket,{route,html:"<p>Existing book page</p>"});});
 expect(host.textContent).toContain("Existing book page");expect(host.querySelectorAll("[data-book-loader]")).toHaveLength(0);
 let ticket=0;await act(async()=>{ticket=vaultStore.begin("/guru/bandy");});
 expect(host.textContent).toContain("Existing book page");expect(host.querySelectorAll("#flipov [data-book-loader]")).toHaveLength(1);
 await act(async()=>{vaultStore.commit(ticket,{html:"<p>Next book page</p>",route});});
 expect(host.textContent).toContain("Next book page");expect(host.querySelectorAll("[data-book-loader]")).toHaveLength(0);
 await act(async()=>{const next=vaultStore.begin("/guru/bandy");vaultStore.fail(next,"Response unavailable");});
 expect(host.querySelector("[role=alert]")?.textContent).toContain("Response unavailable");expect(host.querySelectorAll("[data-book-loader]")).toHaveLength(0);
});
