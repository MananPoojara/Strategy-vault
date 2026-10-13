(function(root,factory){
  if(typeof module==='object'&&module.exports)module.exports=factory();
  else root.VaultAnalyst=factory();
})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  const metricKeys=['cagr','net_profit','max_dd','win_rate','sharpe','monthly_thickness','trades','expectancy_ratio','car_mdd'];
  const rankKeys=['cagr','sharpe','monthly_thickness','expectancy_ratio','car_mdd'];
  const labels={cagr:'CAGR',net_profit:'Net profit',max_dd:'Max DD',win_rate:'Win %',sharpe:'Sharpe',monthly_thickness:'Monthly thickness',trades:'Trades',expectancy_ratio:'Expectancy ratio',car_mdd:'CAR/MDD'};
  const percentages=new Set(['cagr','net_profit','max_dd','win_rate','monthly_thickness']);
  const finite=v=>typeof v==='number'&&Number.isFinite(v);
  const number=v=>finite(v)?v:null;
  const canonical=k=>k==='mar'?'car_mdd':k==='total_return'?'compounded_return':k;
  const fmt=(k,v)=>!finite(v)?'unavailable':percentages.has(k)?(v*100).toFixed(2)+'%':k==='trades'?String(v):v.toFixed(2);
  const fmtDelta=(k,v)=>!finite(v)?'unavailable':(v>0?'+':'')+(percentages.has(k)?(v*100).toFixed(2)+' pp':v.toFixed(2));
  function stats(input){
    const s=input||{},out={};
    metricKeys.forEach(k=>out[k]=number(s[k]));
    if(!finite(out.net_profit))out.net_profit=number(s.closed_return_sum);
    if(!finite(out.car_mdd))out.car_mdd=number(s.mar);
    return out;
  }
  function row(res,index){
    const source=res?.rows?.length?res.rows[index]:res?.picked?.[index];
    if(!Array.isArray(source)||!Array.isArray(res.cols)||source.length!==res.cols.length)return null;
    const p={...(res.fixed||{})},m={};
    (res.dims||[]).forEach((d,i)=>p[d.key]=number(source[i]));
    res.cols.forEach((key,i)=>{if(i>=(res.dims||[]).length)m[key]=source[i];});
    return {p,m:stats(m)};
  }
  function compare(candidate,reference){
    return metricKeys.map(key=>{
      const a=candidate[key],b=reference[key],delta=finite(a)&&finite(b)?a-b:null;
      const advantage=delta===null?null:key==='max_dd'?Math.abs(b)-Math.abs(a):delta;
      return {key,label:labels[key],value:a,reference:b,delta,
        assessment:advantage===null?'unavailable':key==='trades'||key==='win_rate'?'descriptive':advantage>1e-12?'improved':advantage< -1e-12?'weakened':'unchanged'};
    });
  }
  function beforeAfter(run){
    const before=run?.before?.m?stats(run.before.m):null,after=run?.after?.m?stats(run.after.m):null;
    if(!before||!after)return {available:false,before,after,deltas:[],summary:['Before/after comparison unavailable: '+(!before?(run?.before?.error||'no before-period result'):(run?.after?.error||'no after-period result'))+'.']};
    const deltas=compare(after,before),summary=[
      'Before → after: '+fmt('trades',before.trades)+' → '+fmt('trades',after.trades)+' closed trades; CAGR '+fmt('cagr',before.cagr)+' → '+fmt('cagr',after.cagr)+'; Sharpe '+fmt('sharpe',before.sharpe)+' → '+fmt('sharpe',after.sharpe)+'.',
      'Max DD '+fmt('max_dd',before.max_dd)+' → '+fmt('max_dd',after.max_dd)+'; expectancy '+fmt('expectancy_ratio',before.expectancy_ratio)+' → '+fmt('expectancy_ratio',after.expectancy_ratio)+'. Fixed settings were evaluated in both periods.'
    ];
    if([before.trades,after.trades].some(n=>finite(n)&&n<30))summary.push('A split has fewer than 30 closed trades; these comparisons are descriptive. Thirty is a screening heuristic, not a confidence threshold.');
    return {available:true,before,after,deltas,summary};
  }
  function ranking(res,c){
    const pct=rankKeys.map((_,i)=>number(c?.pct?.[i])),valid=pct.every(v=>finite(v)&&v>=0&&v<=100);
    const recomputedScore=valid?pct.reduce((a,b)=>a+b,0)/rankKeys.length:null,recomputedWeakest=valid?Math.min(...pct):null;
    const expectedGroups=(res.rank?.groups||[]).filter(g=>(g.rows||[]).includes(c.row)).map(g=>canonical(g.stat));
    const suppliedGroups=(c.groups||[]).map(canonical),sameGroups=expectedGroups.length===suppliedGroups.length&&expectedGroups.every(k=>suppliedGroups.includes(k));
    const leaderComparisons=rankKeys.map((key,i)=>{
      const group=(res.rank?.groups||[]).find(g=>canonical(g.stat)===key),leader=row(res,group?.rows?.[0]),candidate=row(res,c.row);
      const value=candidate?.m[key]??null,leaderValue=leader?.m[key]??null,positive=finite(leaderValue)&&leaderValue>0;
      return {key,label:labels[key],value,leader:leaderValue,delta:finite(value)&&finite(leaderValue)?value-leaderValue:null,percentile:pct[i],
        ratio:finite(value)&&positive?value/leaderValue:null,note:positive?'Ratio is descriptive; percentile determines the ranking.':'Ratio withheld because the leader is unavailable or non-positive.'};
    });
    return {score:number(c.score),weakest:number(c.weakest),percentiles:pct,recomputedScore,recomputedWeakest,
      scoreVerified:finite(c.score)&&valid&&Math.abs(c.score-recomputedScore)<1e-4,
      weakestVerified:finite(c.weakest)&&valid&&Math.abs(c.weakest-recomputedWeakest)<1e-4,
      groupsVerified:sameGroups,groups:expectedGroups,leaderComparisons};
  }
  function neighborhood(res,c){
    const unavailable=note=>({available:false,inspected:0,rows:[],metrics:[],note});
    if(!Array.isArray(res.rows)||!finite(res.combos)||res.rows.length!==res.combos||!res.rows.length)return unavailable('The complete grid is unavailable; slim or partial results cannot establish local sensitivity.');
    if(!res.rows.every(r=>Array.isArray(r)&&r.length===res.cols?.length))return unavailable('Grid rows are incomplete; local sensitivity is unavailable.');
    const base=row(res,c.row),dims=res.dims||[];
    if(!base||!dims.length)return unavailable('No varied parameter neighborhood is available.');
    const neighborRows=[],missing=[];
    for(const d of dims){
      const values=[...new Set((d.values||[]).filter(finite))].sort((a,b)=>a-b),i=values.indexOf(base.p[d.key]);
      if(i<0)return unavailable('The candidate is not a member of the supplied parameter grid.');
      for(const target of [values[i-1],values[i+1]].filter(finite)){
        const matches=[];
        res.rows.forEach((r,index)=>{if(dims.every((dim,j)=>r[j]===(dim.key===d.key?target:base.p[dim.key])))matches.push(index);});
        if(matches.length===1)neighborRows.push(matches[0]);else missing.push(d.key+'='+target);
      }
    }
    if(missing.length)return unavailable('Expected adjacent grid settings are absent or ambiguous: '+missing.join(', ')+'.');
    const indices=[...new Set(neighborRows)],neighbors=indices.map(i=>row(res,i));
    if(!indices.length)return unavailable('The candidate has no immediate one-step neighbors in this grid.');
    const metrics=rankKeys.map(key=>{
      const vals=neighbors.map(n=>n.m[key]).filter(finite),min=vals.length?Math.min(...vals):null,max=vals.length?Math.max(...vals):null,value=base.m[key];
      return {key,label:labels[key],value,min,max,finiteCount:vals.length,minDelta:finite(value)&&finite(min)?min-value:null,maxDelta:finite(value)&&finite(max)?max-value:null};
    });
    return {available:true,inspected:indices.length,rows:indices,metrics,note:'Immediate one-step neighbors, one parameter changed at a time and other settings held fixed, on the selection dates. This is local sensitivity within the tried grid, not proof of general robustness.'};
  }
  function analyze(D={}){
    const res=D.res||{},allRuns=Array.isArray(D.runs)?D.runs:[],bookRun=allRuns.find(r=>r.book)||allRuns[0];
    const dates=bookRun?.d?.dates||[],metadata={strategy:D.v?.name||D.v?.id||'',instrument:D.tk||'',market:D.region||D.context?.region||null,
      selection:{label:D.selectionLabel||D.selection?.label||'Selection window',from:res.from||null,to:res.to||null},
      evaluation:{label:D.kind||'Selected dates',from:dates[0]||D.win?.from||null,to:dates[dates.length-1]||D.win?.to||null},
      publicationYear:Number.isInteger(D.pub)?D.pub:null,cost:number(D.cost??D.context?.cost),combinations:number(res.combos),eligible:number(res.rank?.eligible),
      minTrades:number(res.rank?.min_trades),perYear:number(res.rank?.per_year)};
    const limits=[
      'Selection and evaluation dates are reported separately. Settings selected on all history and then split before/after publication are retrospective; that split is not an untouched holdout.',
      'The five ranking metrics are not five independent confirmations: CAGR and CAR/MDD share a numerator, and other return measures may be correlated.',
      'Trade-count eligibility is a ranking filter, not a confidence certificate. A high percentile is relative to the tried grid and does not establish positive absolute returns.',
      'Matching rounded aggregate statistics does not prove identical trades. The top-three pool is limited to the union of the five top-five groups.',
      'This assessment interprets computed results; it does not certify source fidelity, independent trade-by-trade verification or future performance.'
    ];
    if(!bookRun?.m||!res.rank||dates.length<2)return {status:'withheld',metadata,book:null,candidates:[],flags:[],summary:['No supported optimization and evaluated backtest are available. Source-only methods receive no invented performance statistics or selections.'],limits};
    const bookStats=stats(bookRun.m),book={name:'Book',parameters:{...(D.def||{}),...(bookRun.p||{})},stats:bookStats,beforeAfter:beforeAfter(bookRun)};
    const flags=[];
    if(res.rank.eligible===1)flags.push('Only one outcome was eligible; a 100 percentile score cannot establish superiority to alternatives.');
    const candidates=(res.rank.top3||[]).slice(0,3).map((c,i)=>{
      const run=allRuns.find(r=>!r.book&&r.c?.row===c.row)||allRuns.filter(r=>!r.book)[i];
      const selected=row(res,c.row),s=stats(run?.m),r=ranking(res,c),comparisons=compare(s,bookStats),ba=beforeAfter(run),n=neighborhood(res,c),candidateFlags=[];
      if(metricKeys.some(k=>!finite(s[k])))candidateFlags.push('Some evaluation statistics are unavailable.');
      if(!r.scoreVerified||!r.weakestVerified||!r.groupsVerified)candidateFlags.push('Supplied ranking metadata could not be fully verified; candidate order is preserved rather than silently changed.');
      if(finite(s.trades)&&s.trades<30)candidateFlags.push('Fewer than 30 closed trades in the evaluation window; sample adequacy is unresolved. Thirty is a screening heuristic, not a confidence threshold.');
      if(finite(s.net_profit)&&s.net_profit<0)candidateFlags.push('Net profit is negative despite this candidate’s relative grid rank.');
      const summary=[
        'CAGR '+fmt('cagr',s.cagr)+' vs book '+fmt('cagr',bookStats.cagr)+' ('+fmtDelta('cagr',comparisons[0].delta)+'); Sharpe '+fmt('sharpe',s.sharpe)+' vs '+fmt('sharpe',bookStats.sharpe)+'; '+fmt('trades',s.trades)+' closed trades.',
        'Monthly thickness '+fmt('monthly_thickness',s.monthly_thickness)+'; expectancy '+fmt('expectancy_ratio',s.expectancy_ratio)+'; CAR/MDD '+fmt('car_mdd',s.car_mdd)+'; Max DD '+fmt('max_dd',s.max_dd)+' vs book '+fmt('max_dd',bookStats.max_dd)+'.',
        'Selection score '+fmt('score',r.score)+'; weakest percentile '+fmt('score',r.weakest)+'; member of '+r.groups.length+' of the five top-five groups. '+(r.scoreVerified&&r.weakestVerified&&r.groupsVerified?'Arithmetic and membership agree with the supplied ranking.':'Ranking verification is incomplete.')
      ];
      const improved=comparisons.filter(m=>rankKeys.includes(m.key)&&m.assessment==='improved').map(m=>m.label),weakened=comparisons.filter(m=>rankKeys.includes(m.key)&&m.assessment==='weakened').map(m=>m.label);
      summary.push('Versus the book on the five ranking metrics: improved '+(improved.join(', ')||'none')+'; weakened '+(weakened.join(', ')||'none')+'. '+(improved.length&&weakened.length?'The higher rank involves measured tradeoffs.':'Relative rank alone does not establish a preferred trading configuration.'));
      const changed=Object.keys({...D.def,...res.fixed,...selected?.p,...run?.p}).filter(k=>finite((selected?.p||run?.p||{})[k])&&finite(D.def?.[k])&&(selected?.p||run?.p)[k]!==D.def[k]);
      summary.push('Changed settings: '+(changed.length?changed.map(k=>k+' '+(selected?.p||run?.p)[k]+' (book '+D.def[k]+')').join(', '):'none among known book defaults')+'.');
      summary.push(...ba.summary,...candidateFlags);
      return {name:run?.name||'#'+(i+1),row:c.row,parameters:{...(D.def||{}),...(run?.p||{}),...(res.fixed||{}),...(selected?.p||{})},stats:s,selectionStats:selected?.m||null,comparisons,ranking:r,beforeAfter:ba,neighborhood:n,flags:candidateFlags,summary};
    });
    if(candidates.length&&candidates.every(c=>finite(c.stats.net_profit)&&c.stats.net_profit<0))flags.push('All selected candidates have negative closed-trade net return sums on the evaluation dates; relative ranks do not support a positive performance conclusion.');
    const summary=['Selection: '+metadata.selection.from+' to '+metadata.selection.to+'; evaluation: '+metadata.evaluation.from+' to '+metadata.evaluation.to+'. '+(finite(metadata.combinations)?metadata.combinations:'Unavailable')+' combinations; '+(finite(metadata.eligible)?metadata.eligible:'unavailable')+' eligible metric outcomes.'];
    if(!candidates.length)summary.push('No eligible top-three selections were supplied.');
    return {status:'computed',metadata,book,candidates,flags,summary:summary.concat(flags),limits};
  }
  return {analyze,metricKeys,rankKeys,labels};
});
