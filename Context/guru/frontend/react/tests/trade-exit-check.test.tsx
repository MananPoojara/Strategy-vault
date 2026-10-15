import {expect,it} from "vitest";
import {renderToStaticMarkup} from "react-dom/server";
import {ResearchMarkup} from "../src/components/ResearchMarkup";
import {tradeExitCheckMarkup} from "../src/services/trade-exit-check";

it("renders unavailable signal context as neutral text without asserting a period match",()=>{
 const markup=tradeExitCheckMarkup({open:false,reason:"Close < 50-day SMA",signal:"2020-01-06",at_exit:{date:"2020-01-06",ma_kind:"SMA",ma_period:5,vs_ma_p:-1}});
 const html=renderToStaticMarkup(<ResearchMarkup html={`<table><tbody><tr><td>Close below SMA${markup}</td></tr></tbody></table>`}/>);
 expect(html).toContain('data-exit-check="unavailable"');expect(html).toContain("Snapshot unavailable");expect(html).not.toContain("-1.00%");
});
it("renders the correct labelled signal-bar value without executable markup",()=>{
 const markup=tradeExitCheckMarkup({open:false,reason:"RSI(14) > 70",signal:"2020-01-06",at_exit:{date:"2020-01-06",rsi_kind:"Wilder",rsi_period:14,rsi_p:75}});
 const html=renderToStaticMarkup(<ResearchMarkup html={markup}/>);
 expect(html).toContain('data-exit-check="available"');expect(html).toContain("RSI(14) 75.0");expect(html).toContain("2020-01-06");expect(html).not.toContain("source-rule verified");
 expect(tradeExitCheckMarkup({reason:'<script>alert(1)</script>',at_exit:{}})).toBe("");
});
