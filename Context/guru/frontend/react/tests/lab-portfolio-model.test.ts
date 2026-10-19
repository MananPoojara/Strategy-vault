import { expect,it } from "vitest";
import { defaultSpec } from "../src/features/lab/spec-model";
import { portfolioProblems,portfolioIdentity,portfolioMembers,withPortfolioMode,portfolioGridProblems } from "../src/features/lab/portfolio-model";
import { catalogue7Fixture } from "./fixtures/lab";
const cat={...catalogue7Fixture,portfolio:{modes:["signals","rotation"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar","W","M","Q","Y"],baskets:[{id:"nifty50",members:50,membership_kind:"current_snapshot"}]}};
it("enables portfolio explicitly without changing incompatible user settings",()=>{
 const spec={...defaultSpec(),direction:"short" as const,origin:{book:"b",method:"m",template_version:1}};
 const next=withPortfolioMode(spec,"signals");expect(next.direction).toBe("short");expect(next.origin).toEqual(spec.origin);expect(next.execution?.capital).toBeUndefined();
 expect(portfolioProblems(next,cat,"in").join(" ")).toContain("capital");expect(portfolioProblems(next,cat,"in").join(" ")).toContain("origin");
 expect(withPortfolioMode(next,"off")).toEqual(spec);
});
it("validates actual-member lists, rotation invariants, and advertised limits",()=>{
 const spec={...withPortfolioMode(defaultSpec(),"rotation"),entry:{all:[]},exit:{any:[]},execution:{capital:1000}};
 spec.portfolio={...spec.portfolio!,universe:{basket:"nifty50"},score:{abs:{s:"rsi",p:{length:2}}}};
 expect(portfolioProblems(spec,cat,"in")).toEqual([]);expect(portfolioIdentity(spec)).toBe("portfolio:nifty50");expect(portfolioMembers(spec,cat)).toBe(50);
 expect(portfolioGridProblems({...spec,optimize:[{path:"portfolio.max_open",from:1,to:201,step:1}]},cat).join(" ")).toContain("200");
 expect(portfolioProblems(spec,catalogue7Fixture,"in").join(" ")).toContain("advertised");
 expect(portfolioProblems({...spec,portfolio:{...spec.portfolio!,worst_rank_held:1,max_open:2}},cat,"in").join(" ")).toContain("worst");
});
it("keeps score traversal and Cartesian rank/work limits consistent",()=>{
 const s={...withPortfolioMode(defaultSpec(),"rotation"),entry:{all:[]},exit:{any:[]},execution:{capital:1000}};s.portfolio={...s.portfolio!,universe:{basket:"nifty50"},score:{count:{lhs:{s:"close"},op:"rising",bars:2},bars:3},max_open:2};
 const bad={...s,optimize:[{path:"portfolio.max_open",from:2,to:6,step:2},{path:"portfolio.worst_rank_held",from:4,to:8,step:2}]};expect(portfolioGridProblems(bad,cat).join(" ")).toContain("Every optimized");
 expect(portfolioProblems({...s,portfolio:null} as never,cat,"in").join(" ")).toContain("kept as is");
});

it("refuses the single-instrument AmiBroker fields in a portfolio, like the engine, and strips them on switching", () => {
 const port={...defaultSpec(),execution:{capital:1e6},portfolio:{universe:{basket:"nifty50"},mode:"signals" as const}};
 const cat={...catalogue7Fixture,portfolio:{modes:["signals","rotation"],max_open:50,max_universe:120,max_universe_list:50,max_combos:200,max_work:12000,baskets:[{id:"nifty50",members:50}]}} as never;
 for(const[key,value]of[["reentry",{stop_loss:3}],["prices",{buy:"close"}],["delays",{buy:0}],["limits",{round_lot:10}],["tick_size",0.05],["pad_align",true]] as const){
  const msg=portfolioProblems({...port,execution:{...port.execution,[key]:value}},cat,"in");
  expect(msg).toContain(`execution.${key} is not supported in a portfolio; settings are kept until you edit them.`);
 }
 const single={...defaultSpec(),execution:{capital:1e6,reentry:{stop_loss:3},prices:{buy:"close" as const},delays:{buy:0},limits:{round_lot:10},tick_size:0.05,pad_align:true}};
 expect(withPortfolioMode(single,"signals").execution).toEqual({capital:1e6});
});
