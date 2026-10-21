(function(root,factory){if(typeof module==='object'&&module.exports)module.exports=factory();else root.VaultLibraryDiscovery=factory();})(typeof globalThis!=='undefined'?globalThis:this,function(){
 'use strict';
 const categories=['available','needs_stock_basket','needs_data','source_review','no_trades','no_eligible_results'];
 const finite=v=>typeof v==='number'&&Number.isFinite(v);
 const clean=value=>typeof value==='string'?value.replace(/[\u0000-\u001f\u007f]/g,' ').replace(/\s+/g,' ').trim().slice(0,600):'';
 function lookup(input,id){
  if(input instanceof Map)return input.get(id);
  if(Array.isArray(input))return input.find(row=>row?.id===id);
  if(Array.isArray(input?.strategies))return input.strategies.find(row=>row?.id===id);
  return input&&Object.prototype.hasOwnProperty.call(input,id)?input[id]:undefined;
 }
 const requirements=new Set(['individual_stock','index','etf','futures','portfolio','options','leveraged_etf','treasury_etf','index_or_etf','index_etf_or_stock','declared_ohlc_adaptation','declared_index_timing']);
 function policyKnown(def){return requirements.has(def?.instrument_policy?.requirement)&&Array.isArray(def.instrument_policy.allowed_types);}
 function measuredStats(summary){
  const stats=summary?.stats;
  if(!stats||!finite(stats.trades)||!Number.isInteger(stats.trades)||stats.trades<0)return false;
  // Undefined risk ratios can accompany genuine measured results. A NaN core
  // closed-return value cannot be rescued by a finite compound-model value.
  return Object.prototype.hasOwnProperty.call(stats,'net_profit')?finite(stats.net_profit):finite(stats.total_return);
 }
 const uncertainty=/source review|unreviewed|not independently reviewed|discretionary|subjective|rules?[^.]{0,90}(?:incomplete|uncertain|not specified|not defined)|unsupported source (?:rule|assumption)|source[^.]{0,60}uncertainty/i;
 const dataGap=/missing|unavailable|requires?[^.]{0,110}(?:data|prices|quotes|contracts|premiums|intraday|volume)|coverage|histor(?:y|ical universe)|dataset|benchmark|liquidity|volatility|expiry|roll history|ordering[^.]{0,50}(?:intraday|daily)/i;
 const stockMismatch=/cash index|index or etf|index[^.]{0,80}(?:stock population|individual stock|constituent stock)|requires? a validated individual stock|selected[^.]{0,100}: (?:index|etf)\b/i;
 function classify(def,summary,readiness){
  const known=policyKnown(def),policy=def.instrument_policy||{};
  const readyStatus=clean(readiness?.status),summaryStatus=clean(summary?.readiness);
  const readyBlock=readyStatus&&readyStatus!=='ready';
  const summaryBlock=typeof summary?.blocked==='string'&&summary.blocked.trim();
  const note=clean(readyBlock?readiness.note:summaryBlock?summary.blocked:readiness?.note||def.status_note||'');
  const status=readyBlock?readyStatus:summaryBlock?(summaryStatus||'blocked'):'';
  const stockOnly=policy.requirement==='individual_stock'||policy.allowed_types?.length===1&&policy.allowed_types[0]==='individual_stock';
  if(!known)return {status:'source_review',note:note||'Source instrument requirements have not been reviewed for this method.'};
  if(status){
   if(status==='unavailable')return {status:'no_eligible_results',note:note||'The request could not be completed for this selection.'};
   if(uncertainty.test(note))return {status:'source_review',note};
   if(stockOnly&&stockMismatch.test(note))return {status:'needs_stock_basket',note};
   if(status==='needs_data'||dataGap.test(note))return {status:'needs_data',note:note||'Required input data is unavailable.'};
   if(status==='not_testable'||status==='source_review')return {status:'source_review',note:note||'The source rule is not currently testable.'};
   return {status:'no_eligible_results',note:note||'No eligible result is available for the selected instrument and period.'};
  }
  const declared=clean(def.status);
  if(declared==='not_testable'||declared==='source_review')return {status:'source_review',note:note||'Source rules are not currently testable.'};
  if(declared==='needs_data'&&readyStatus!=='ready')return {status:'needs_data',note:note||'Required data is not currently available for this method.'};
  if(!policy.allowed_types.length)return {status:'needs_data',note:note||clean(policy.source_basis)||'Required source instruments or portfolio inputs are not currently admitted.'};
  if(measuredStats(summary)){
   return summary.stats.trades>0?{status:'available',note:'Measured closed-trade results for the selected instrument and period; original book performance is not certified.'}:{status:'no_trades',note:'The measured run has no closed positions in the selected period; this does not establish that no signal or open position exists.'};
  }
  return {status:'no_eligible_results',note:'No finite measured summary is available for the selected instrument and period.'};
 }
 function authorAvailability(authorBookIds,strategyDefs,summaryById,readinessById){
  const books=new Set(Array.from(authorBookIds||[]).filter(id=>typeof id==='string'&&id));
  const seen=new Set(),defs=(Array.isArray(strategyDefs)?strategyDefs:[]).filter(def=>def&&typeof def.id==='string'&&books.has(def.book)&&!seen.has(def.id)&&seen.add(def.id));
  const bookMethods=new Set(defs.map(def=>def.book)),families=new Set();
  const counts=Object.fromEntries(categories.map(key=>[key,0])),sets=Object.fromEntries(categories.map(key=>[key,new Set()]));
  const variantReasons=defs.map(def=>{
   const family=def.book+':'+(typeof def.family==='string'&&def.family?def.family:def.id);
   const result=classify(def,lookup(summaryById,def.id),lookup(readinessById,def.id));
   families.add(family);counts[result.status]++;sets[result.status].add(family);
   return {id:def.id,book:def.book,family,status:result.status,note:result.note};
  });
  const grouped=new Map();
  for(const reason of variantReasons){const key=reason.status+'\u0000'+reason.note;let item=grouped.get(key);if(!item){item={status:reason.status,note:reason.note,variants:0,families:new Set()};grouped.set(key,item);}item.variants++;item.families.add(reason.family);}
  const reasonCounts=Array.from(grouped.values(),r=>({status:r.status,note:r.note,variants:r.variants,families:r.families.size})).sort((a,b)=>b.families-a.families||b.variants-a.variants||a.status.localeCompare(b.status)||a.note.localeCompare(b.note));
  const familyCounts=Object.fromEntries(categories.map(key=>[key,sets[key].size]));
  // Categories count variants; family sets can overlap where siblings have
  // different outcomes. Available siblings never certify every family variant.
  let status=defs.length?'no_eligible_results':'reading_only';
  if(counts.available)status='available';
  else if(defs.length){const priority=['source_review','needs_stock_basket','needs_data','no_trades','no_eligible_results'];status=priority.reduce((best,key)=>familyCounts[key]>familyCounts[best]?key:best,priority[0]);}
  const titles={available:`${familyCounts.available} available ${familyCounts.available===1?'family':'families'}`,needs_stock_basket:'Choose a stock basket',needs_data:'Required data missing',source_review:'Not backtested yet',reading_only:'Reading sources',no_trades:'No closed trades in this period',no_eligible_results:'Results unavailable for this selection'};
  const descriptions={available:'Actual closed-trade results are available for this selection. Availability does not verify the original book study.',needs_stock_basket:'These methods require actual constituent stocks. Choose a stock basket and a stock to run them.',needs_data:'Required prices, history, filters or calculation inputs are unavailable for this selection.',source_review:'Held: the rules are not fully defined, an accuracy check withholds them, or the required data or instrument is not admitted. Each strategy shows its reason.',reading_only:'Reading sources are indexed; no backtest method is registered for this author. Original reading access and content review are shown for each entry.',no_trades:'Measured runs have no closed positions in this period. Open marks or signals may still exist.',no_eligible_results:'No finite measured result is available for the selected instrument and period.'};
  const parts=[['available','with closed-trade results'],['needs_stock_basket','requiring a constituent stock'],['needs_data','needing data'],['source_review','held (reason shown on each strategy)'],['no_trades','with no closed trades'],['no_eligible_results','without an eligible result']].filter(([key])=>counts[key]).map(([key,label])=>`${counts[key]} ${counts[key]===1?'variant':'variants'} ${label}`);
  return {status,title:titles[status],text:descriptions[status]+(parts.length?' '+parts.join('; ')+'.':''),bookCount:books.size,readingBookCount:books.size-bookMethods.size,strategyCount:defs.length,familyCount:families.size,availableFamilies:familyCounts.available,availableVariants:counts.available,admittedVariants:counts.available+counts.no_trades,counts,familyCounts,reasonCounts,variantReasons};
 }
 // Returned notes are plain source text, not trusted HTML. Render with
 // textContent or the caller's HTML-escaping helper. No fetch or DOM mutation.
 return {authorAvailability};
});
