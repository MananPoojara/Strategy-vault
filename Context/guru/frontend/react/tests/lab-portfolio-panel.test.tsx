import {afterEach,expect,it,vi} from "vitest";
import {PortfolioPanel,dbName,scratchList} from "../src/features/lab/PortfolioPanel";
import {defaultSpec} from "../src/features/lab/spec-model";
import {catalogue7Fixture,mount,unmount} from "./fixtures/lab";
import {act} from "react";
afterEach(unmount);
it("offers advertised modes and real stocks, keeps settings until explicitly edited",async()=>{
 const onChange=vi.fn();const cat={...catalogue7Fixture,portfolio:{modes:["signals","rotation"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar","W","M","Q","Y"],baskets:[]}};
 const host=await mount(<PortfolioPanel spec={defaultSpec()} catalogue={cat} market="in" options={[]} onChange={onChange}/>);
 const signals=[...host.querySelectorAll<HTMLButtonElement>('[aria-label="Strategy mode"] button')].find(b=>b.textContent==="Portfolio · entry and exit signals")!;expect(signals).toBeDefined();
 expect(host.querySelector('[aria-label="Strategy mode"] [aria-pressed="true"]')!.textContent).toBe("Single instrument"); // the default
 await act(async()=>signals.click());expect(onChange.mock.calls[0][0].portfolio.mode).toBe("signals");
 expect(onChange.mock.calls[0][0].execution?.capital).toBeUndefined();
});
it("lists dated C&P universes by name with no member cap, and explains them when picked",async()=>{
 const cap={modes:["signals","rotation"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar"],
  baskets:[{id:"nse500",members:500,membership_kind:"current_snapshot",supported:false},{id:"nse500_cp",label:"NSE 500 (members by date)",members:1494,membership_kind:"dated_aux",supported:true}]};
 const spec={...defaultSpec(),portfolio:{mode:"signals",universe:{basket:"nse500_cp"},max_open:5}} as never;
 const host=await mount(<PortfolioPanel spec={spec} catalogue={{...catalogue7Fixture,portfolio:cap}} market="in" options={[]} onChange={vi.fn()}/>);
 const opt=host.querySelector<HTMLInputElement>('[aria-label="Portfolio universe"] input[value="nse500_cp"]')!;
 expect(opt.closest("label")!.textContent).toBe("NSE 500 (members by date)by date1494 names");expect(opt.disabled).toBe(false);expect(opt.checked).toBe(true);
 expect(host.textContent).toContain("bought only on days it was in it");
});
it("opens an AmiBroker database's watchlist with its Aux filters, as AmiBroker does",async()=>{
 const cap={modes:["signals"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar"],
  baskets:[{id:"nse500_cp",label:"NSE 500 (members by date)",members:1494,membership_kind:"dated_aux",supported:true},
   {id:"ami-cp-currentfo",label:"CURRENT FO",database:"C&P DATABASE",members:94,membership_kind:"ami_watchlist",supported:true},
   {id:"ami-cp-nifty50cp141",label:"Nifty 50 C&P 141",database:"C&P DATABASE",members:138,membership_kind:"ami_watchlist",supported:true}]};
 const onChange=vi.fn();const spec={...defaultSpec(),portfolio:{mode:"signals",universe:{basket:"nse500_cp"},max_open:5}} as never;
 const host=await mount(<PortfolioPanel spec={spec} catalogue={{...catalogue7Fixture,portfolio:cap}} market="in" options={[]} onChange={onChange}/>);
 const db=host.querySelector<HTMLSelectElement>('select[aria-label="Database"]')!;expect([...db.options].map(o=>o.value)).toEqual(["C&P DATABASE",expect.stringContaining("Vault lists")]);
 await act(async()=>{db.value="C&P DATABASE";db.dispatchEvent(new Event("change",{bubbles:true}));});
 expect(onChange.mock.calls[0][0].portfolio.universe).toEqual({basket:"ami-cp-nifty50cp141"}); // the team's watchlist when present
 await unmount();const opened={...defaultSpec(),portfolio:{mode:"signals",universe:{basket:"ami-cp-currentfo"},max_open:5}} as never;onChange.mockClear();
 const h2=await mount(<PortfolioPanel spec={opened} catalogue={{...catalogue7Fixture,portfolio:cap}} market="in" options={[]} onChange={onChange}/>);
 expect(h2.querySelector<HTMLSelectElement>('[aria-label="Watchlist"]')!.value).toBe("ami-cp-currentfo");
 await act(async()=>h2.querySelector<HTMLInputElement>('input[aria-label="Aux1"]')!.click());expect(onChange.mock.calls[0][0].execution.aux1).toBe(true);
});
it("names databases without their folder suffix, and never lets Aux2 block every buy where it is not filled",async()=>{
 expect(dbName("C&P DATABASE DATABASE UPDATED TILL 16-10-2026")).toBe("C&P DATABASE · till 16-10-2026");
 expect(dbName("NIFTY 50 C&P (With Aux On - Off)_Updated Till 16-10-2026")).toBe("NIFTY 50 C&P (With Aux On - Off) · till 16-10-2026");
 const cap={modes:["signals"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar"],
  baskets:[{id:"ami-n50-currentnifty",label:"Current Nifty",database:"NIFTY 50 C&P (With Aux On - Off)_Updated Till 16-10-2026",members:50,membership_kind:"ami_watchlist",supported:true}]};
 const spec={...defaultSpec(),portfolio:{mode:"signals",universe:{basket:"ami-n50-currentnifty"},max_open:5}} as never;
 const host=await mount(<PortfolioPanel spec={spec} catalogue={{...catalogue7Fixture,portfolio:cap}} market="in" options={[]} onChange={vi.fn()}/>);
 expect(host.querySelector('input[aria-label="Aux1"]')!.closest("label")!.textContent).toContain("in NIFTY 50 that day");
 const aux2=host.querySelector<HTMLInputElement>('input[aria-label="Aux2"]')!;expect(aux2.disabled).toBe(true);expect(aux2.closest("label")!.textContent).toContain("not filled in this database");
});
it("hides the team's scratch watchlists but keeps the research lists and one already chosen",async()=>{
 for(const n of ["123","Check","Temp_123","Temp1","temp_50","NEW STOCK CHECK(IGNORE)","WL For AFL Check","List 62"])expect(scratchList(n),n).toBe(true);
 for(const n of ["Nifty 50 C&P 141","Current Nifty","NSE 500 C&P","FO SCRIPTS C&P","MOM Symbol","new Added","C&P Stocks"])expect(scratchList(n),n).toBe(false);
 const w=(id:string,label:string)=>({id,label,database:"NIFTY 50 C&P",members:5,membership_kind:"ami_watchlist",supported:true});
 const cap={modes:["signals"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar"],baskets:[w("ami-n50-check","Check"),w("ami-n50-temp123","Temp_123"),w("ami-n50-currentnifty","Current Nifty")]};
 const at=async(basket:string)=>{await unmount();const h=await mount(<PortfolioPanel spec={{...defaultSpec(),portfolio:{mode:"signals",universe:{basket},max_open:5}} as never} catalogue={{...catalogue7Fixture,portfolio:cap}} market="in" options={[]} onChange={vi.fn()}/>);
  return [...h.querySelectorAll<HTMLOptionElement>('select[aria-label="Watchlist"] option')].map(o=>o.value);};
 expect(await at("ami-n50-currentnifty")).toEqual(["ami-n50-currentnifty"]);
 expect(await at("ami-n50-check")).toEqual(["ami-n50-check","ami-n50-currentnifty"]); // a saved strategy keeps its list
});
