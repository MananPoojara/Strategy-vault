import {useMemo,useState,type CSSProperties} from 'react';
import {pristineNumber,type PristinePresentation} from '../services/pristine-presenter';

const MONTHS=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
type Palette='classic'|'ocean'|'violet'|'mono';
type VisualValue={value:number|null;reason?:string};
const absent:VisualValue={value:null};
/** Averages are visual summaries only, never engine statistics. Unknown
 * observed periods propagate their reason; outside-window periods are absent. */
function average(values:readonly VisualValue[]):VisualValue{
  const unavailable=values.find(value=>value.reason);
  if(unavailable)return {value:null,reason:'An observed period is unavailable; no average is asserted.'};
  const known=values.flatMap(value=>value.value===null?[]:[value.value]);
  if(!known.length)return absent;
  const sum=known.reduce((total,value)=>total+value,0),value=sum/known.length;
  if(sum!==0&&value===0)return {value:null,reason:'Visual average underflow; recorded periods and engine statistics are unchanged.'};
  return Number.isFinite(sum)&&Number.isFinite(value)?{value}:{value:null,reason:'Visual average overflow; recorded periods and engine statistics are unchanged.'};
}
/** Consumes already typed visual periods. It has no strategy/admission path. */
export function PristineMonthlyHeatmap({model}: {model:Pick<PristinePresentation,'months'|'years'|'visualPeriodReasons'>}){
  const [palette,setPalette]=useState<Palette>('classic'),[descending,setDescending]=useState(false);
  const calendar=useMemo(()=>{
    const years=[...new Set([...Object.keys(model.months),...Object.keys(model.years)].map(key=>key.slice(0,4)))].sort();
    const month=(year:string,index:number):VisualValue=>{const key=`${year}-${String(index+1).padStart(2,'0')}`;return Object.hasOwn(model.months,key)?{value:model.months[key],reason:model.visualPeriodReasons.months[key]}:absent;};
    const annual=(year:string):VisualValue=>Object.hasOwn(model.years,year)?{value:model.years[year],reason:model.visualPeriodReasons.years[year]}:absent;
    const averages=MONTHS.map((_,index)=>average(years.map(year=>month(year,index))));
    const annualAverage=average(years.map(annual));
    const cap=(values:readonly (number|null)[])=>values.reduce<number>((maximum,value)=>value===null?maximum:Math.max(maximum,Math.abs(value)),.01);
    return {years,month,annual,averages,annualAverage,monthCap:cap(Object.values(model.months)),annualCap:cap(Object.values(model.years))};
  },[model]);
  const shades=palette==='ocean'?['#1689bb','#d36d28']:palette==='violet'?['#078f84','#b746a2']:palette==='mono'?['var(--blue)','var(--blue)']:['#19855d','#cf5550'];
  const style=(entry:VisualValue,cap:number):CSSProperties=>{
    if(entry.value===null)return {background:'color-mix(in srgb,var(--ink3) 6%,var(--card))',color:'var(--ink)'};
    if(entry.value===0)return {background:'color-mix(in srgb,var(--ink3) 12%,var(--card))',color:'var(--ink)'};
    const intensity=Math.round((Math.min(1,Math.abs(entry.value)/cap)*.56+.16)*100);
    return {'--pristine-heat-shade':shades[entry.value>0?0:1],background:`color-mix(in srgb,var(--pristine-heat-shade) ${intensity}%,var(--card))`,color:'var(--ink)'}as CSSProperties;
  };
  const cell=(entry:VisualValue,key:string,cap=calendar.monthCap,total=false)=><td key={key} className={total?'t':undefined} data-period={key} data-annual-average={key==='avg-total'?'':undefined} aria-label={key==='avg-total'?'Avg/Total':undefined} data-sign={entry.value===null?(entry.reason?'unavailable':'absent'):entry.value===0?'zero':entry.value>0?'positive':'negative'} style={style(entry,cap)} title={entry.reason||(entry.value===null?'Outside available period':pristineNumber(entry.value,true))}>{entry.value===null?(entry.reason?'—':''):pristineNumber(entry.value,true).replace(/%$/,'')}</td>;
  const years=descending?[...calendar.years].reverse():calendar.years;
  return <>
    <div className="chart-tools"><label className="dfield">Colors <select aria-label="Monthly heatmap colors" value={palette} onChange={event=>setPalette(event.target.value as Palette)}><option value="classic">Green / red</option><option value="ocean">Blue / orange</option><option value="violet">Teal / magenta</option><option value="mono">Single hue</option></select></label><span className="join sm"><button type="button" className={`join-item ${descending?'':'active'}`} onClick={()=>setDescending(false)}>Oldest first</button><button type="button" className={`join-item ${descending?'active':''}`} onClick={()=>setDescending(true)}>Newest first</button></span></div>
    <div className="heat-scroll"><table className="heat2 heatmap-refined" aria-label="Monthly returns heatmap"><thead><tr><th className="y" scope="col">Year</th>{MONTHS.map(month=><th key={month} scope="col">{month}</th>)}<th className="t" scope="col">Total</th></tr></thead><tbody>{years.map(year=><tr key={year}><th className="y" scope="row">{year}</th>{MONTHS.map((_,index)=>cell(calendar.month(year,index),`${year}-${String(index+1).padStart(2,'0')}`))}{cell(calendar.annual(year),`${year}-total`,calendar.annualCap,true)}</tr>)}</tbody><tfoot><tr><th className="y" scope="row">Avg</th>{calendar.averages.map((value,index)=>cell(value,`avg-${index+1}`))}{cell(calendar.annualAverage,'avg-total',calendar.annualCap,true)}</tr></tfoot></table></div>
    <p className="rnote">Values are percentages; darker shading means a larger return magnitude. Total is the selected calendar-year visual return. Avg/Total is the arithmetic mean of the displayed calendar-year returns, including partial years; it is not CAGR. Zero returns display 0.00; outside-window months stay blank. Unavailable observed returns and their averages retain an explicit reason.</p>
    {calendar.annualAverage.reason&&<p className="rnote" role="status">Avg/Total: {calendar.annualAverage.reason}</p>}
  </>;
}
