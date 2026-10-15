import {afterEach,expect,it,vi} from "vitest";
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
