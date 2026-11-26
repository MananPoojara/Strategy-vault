import {afterEach,expect,it,vi} from "vitest";
import {act} from "react";
import {PortfolioResults,portfolioTradeRows} from "../src/features/lab/PortfolioResults";
import {mount,unmount,runFixture} from "./fixtures/lab";
import type {PortfolioRunResult} from "../src/features/lab/types";
const runtime=window.VaultRuntime;afterEach(async()=>{await unmount();window.VaultRuntime=runtime;});
import {portfolioResultFixture as result} from "./fixtures/portfolio";
it("keeps ticker-qualified raw fills and cash units distinct from normalized equity",async()=>{
 const render=vi.fn();window.VaultRuntime={renderAreaChart:render} as never;
 const host=await mount(<PortfolioResults result={result}/>);
 expect(host.textContent).toContain("Today's membership has survivorship bias.");expect(host.textContent).toContain("MISSING");expect(host.textContent).toContain("ACTUAL");expect(host.textContent).toContain("Cash");
 expect(portfolioTradeRows(result)[1][0]).toBe("ACTUAL");
 expect(render.mock.calls.some(c=>c[2][1]===-.010000000000000009&&c[3].markMin===true)).toBe(true);
 expect(host.textContent).toContain("Contribution / signal equity");
});
it("shows AmiBroker's detailed log, finds a stock or a day-first date, and keeps every session for the download",async()=>{
 window.VaultRuntime={renderAreaChart:vi.fn()} as never;
 const log=[{date:"2008-04-30",lines:["Entry signals(score): INFY=Buy(1), TCS=Buy(1)","Enter Long, INFY, Price: 219.14, Shares: 375","Skipped TCS: no free position (1 of 1 open), rank 2","1 Open Positions: INFY (+375)"]},{date:"2008-05-30",lines:["Exit Long, INFY, Price: 245.35","0 Open Positions: "]}];
 const host=await mount(<PortfolioResults result={{...result,log} as PortfolioRunResult}/>);
 const pre=()=>host.querySelector("[data-lab-portfolio-log] pre")?.textContent??"";
 expect(host.querySelector("[data-lab-portfolio-log] summary")?.textContent).toBe("Detailed log · 2 sessions with signals or trades");
 expect(pre()).toContain("30-04-2008\n   Entry signals(score): INFY=Buy(1), TCS=Buy(1)");
 const find=host.querySelector<HTMLInputElement>("[aria-label='Find in the detailed log']")!;
 const type=async(v:string)=>{const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!;await act(async()=>{set.call(find,v);find.dispatchEvent(new Event("input",{bubbles:true}));});};
 await type("30-05");expect(pre()).toContain("Exit Long, INFY");expect(pre()).not.toContain("TCS");
 await type("tcs");expect(pre()).toContain("Skipped TCS");expect(pre()).not.toContain("30-05-2008");
 await type("NOPE");expect(host.textContent).toContain("No session matches");
});
it("has no detailed log section when the run returns none",async()=>{
 window.VaultRuntime={renderAreaChart:vi.fn()} as never;
 const host=await mount(<PortfolioResults result={result}/>);expect(host.querySelector("[data-lab-portfolio-log]")).toBeNull();
});
