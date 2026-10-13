(function(root,factory){if(typeof module==='object'&&module.exports)module.exports=factory();else root.VaultGridExport=factory();})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  const labels={total_return:'Compounded model return',net_profit:'Closed-trade net return sum',avg_net_pnl:'Avg net pnl',cagr:'CAGR',sharpe:'Sharpe',sortino:'Sortino',max_dd:'Max drawdown',mar:'CAR/MDD',trades:'No. of trades',win_rate:'Win %',profit_factor:'Profit factor',avg_trade:'Average trade',exposure:'Exposure',t_stat:'t statistic',avg_bars:'Average bars',expectancy_ratio:'Expectancy ratio',monthly_thickness:'Monthly thickness',avg_hold_days:'Average hold days'};
  function fullGrid(res,schema=[]){
    if(!res||!Array.isArray(res.cols)||!Array.isArray(res.dims)||!Array.isArray(res.rows)||!Number.isSafeInteger(res.combos)||res.combos<1||res.rows.length!==res.combos)throw new Error('A complete optimization grid is required for export.');
    const nd=res.dims.length;
    if(res.dims.some((d,i)=>res.cols[i]!=='p.'+d.key))throw new Error('Optimization parameter columns do not match the grid dimensions.');
    if(res.rows.some(r=>!Array.isArray(r)||r.length!==res.cols.length))throw new Error('Optimization row columns are incomplete.');
    const settings=[...schema];
    for(const d of res.dims)if(!settings.some(s=>s.key===d.key))settings.push({key:d.key,label:d.key,def:null});
    const metrics=res.cols.slice(nd),numeric=v=>typeof v==='number'&&Number.isFinite(v)?v:null;
    return {header:['Run',...metrics.map(k=>labels[k]||k),...settings.map(s=>s.label||s.key)],metricKeys:metrics,
      rows:res.rows.map((r,i)=>[i+1,...r.slice(nd).map(numeric),...settings.map(s=>{const j=res.dims.findIndex(d=>d.key===s.key);return numeric(j>=0?r[j]:(res.fixed?.[s.key]??s.def));})])};
  }
  return {fullGrid};
});
