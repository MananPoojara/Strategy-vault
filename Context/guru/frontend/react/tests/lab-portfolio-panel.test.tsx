import {afterEach,expect,it,vi} from "vitest";
import {PortfolioPanel} from "../src/features/lab/PortfolioPanel";
import {defaultSpec} from "../src/features/lab/spec-model";
import {catalogue7Fixture,mount,unmount} from "./fixtures/lab";
import {act} from "react";
afterEach(unmount);
it("offers advertised modes and real stocks, keeps settings until explicitly edited",async()=>{
 const onChange=vi.fn();const cat={...catalogue7Fixture,portfolio:{modes:["signals","rotation"],max_universe:120,max_universe_list:50,max_open:50,max_combos:200,max_work:12000,rebalance:["bar","W","M","Q","Y"],baskets:[]}};
 const host=await mount(<PortfolioPanel spec={defaultSpec()} catalogue={cat} market="in" options={[]} onChange={onChange}/>);
 const select=host.querySelector<HTMLSelectElement>('[aria-label="Strategy mode"]')!;expect(select).not.toBeNull();
 await act(async()=>{select.value="signals";select.dispatchEvent(new Event("change",{bubbles:true}));});expect(onChange.mock.calls[0][0].portfolio.mode).toBe("signals");
 expect(onChange.mock.calls[0][0].execution?.capital).toBeUndefined();
});
