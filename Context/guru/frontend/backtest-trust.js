/* Shared presentation rules for data-backed results. No prices or returns are generated here. */
(function(root){
  const escape=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  function statusLabel(status){
    return {needs_data:'Data missing',not_testable:'Rules unresolved',calculation_unavailable:'Backtest calculation unavailable',ready:'Data checks passed'}[status] || 'Backtest unavailable';
  }
  function hasResult(row){return Boolean(row && !row.blocked && row.stats && typeof row.stats==='object');}
  function bookReference(claim){
    if(!claim) return '<p class="muted">No published performance figures are recorded for this rule.</p>';
    const fields=[['Period and instruments',claim.period],['Trades',claim.trades],['Win rate',claim.win],['Average result',claim.avg],['Holding period',claim.hold]].filter(([,value])=>value && value!=='-');
    return '<p class="rnote">These figures are reported by the book. They are reference claims, separate from this app’s backtest on the selected market and instrument. Matching the book’s performance has not been established by this run.</p><dl class="book-reference">'+fields.map(([key,value])=>'<div><dt>'+escape(key)+'</dt><dd>'+escape(value)+'</dd></div>').join('')+'</dl>';
  }
  function actionMarkers(trades,dates,barOf,colors){
    const markers=[];
    for(const trade of trades){
      const long=trade.dir>0;
      trade.legs.forEach(([i],k)=>markers.push({time:dates[barOf[i]],position:long?'belowBar':'aboveBar',shape:long?'arrowUp':'arrowDown',color:long?colors.buy:colors.sell,size:k?0.7:1}));
      if(!trade.open) markers.push({time:dates[barOf[trade.exit_i]],position:long?'aboveBar':'belowBar',shape:long?'arrowDown':'arrowUp',color:long?colors.sell:colors.buy,size:1});
    }
    return markers;
  }
  const api={escape,statusLabel,hasResult,bookReference,actionMarkers};
  if(typeof module!=='undefined' && module.exports) module.exports=api; else root.VaultTruth=api;
})(globalThis);
