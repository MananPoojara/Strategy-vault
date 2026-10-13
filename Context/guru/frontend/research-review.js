(function(root,factory){if(typeof module==='object'&&module.exports)module.exports=factory();else root.VaultResearch=factory();})(typeof globalThis!=='undefined'?globalThis:this,function(){
  'use strict';
  const domains=new Set(['tradingeducators.com','www.tradingeducators.com','www.bollingerbands.com','www.itl.nist.gov','help.cqg.com','www.cmegroup.com','www.nseindia.com','help.tradestation.com','catalogimages.wiley.com','www.wiley-vch.de']);
  const sourceLink=value=>{try{const u=new URL(value);return u.protocol==='https:'&&domains.has(u.hostname)&&!u.username&&!u.password;}catch{return false;}};
  const recorded=values=>[...new Set(values.filter(v=>typeof v==='string'&&v.trim().length))];
  function blockers(evidence){return [['missing_data','needs_data','Data missing'],['rule_gaps','not_testable','Rules unresolved'],['source_gaps','source_missing','Source pages missing']].map(([key,status,label])=>({status,label,items:recorded(evidence?.[key]||[])})).filter(b=>b.items.length);}
  // Plain recorded evidence for tables. No rule inference, prices or optimizer results.
  function assessment(ready,issue,evidence){
    const source=recorded([evidence?.source?.path,evidence?.pages]);
    if(typeof evidence?.source?.sha256==='string'&&/^[a-f0-9]{64}$/i.test(evidence.source.sha256))source.push('SHA-256: '+evidence.source.sha256);
    const rules=recorded([evidence?.conclusion]);
    const limitations=recorded([issue?.note,evidence?.reason,...blockers(evidence).flatMap(b=>b.items.map(t=>b.label+': '+t))]);
    const hasRecord=source.length>0||rules.length>0||recorded([evidence?.reason]).length>0;
    return [
      {key:'source',label:'Source record',status:source.length?'recorded':'unavailable',values:source.length?source:['No source location is recorded for this assessment.']},
      {key:'rules',label:'Rule assessment',status:rules.length?'recorded':'unavailable',values:rules.length?rules:['No source-specific rule assessment is recorded for this method.']},
      {key:'available_evidence',label:'Available evidence',status:ready?'computed':hasRecord?'recorded':'unavailable',values:ready?
        ['A computed run is available for the selected instrument and dates. Its existence does not certify source fidelity or independent trade verification.']:
        [hasRecord?'The source record and authored assessment shown here are available; they do not establish backtested performance.':'No source-specific evidence record or supported backtest is available for this assessment.']},
      {key:'missing_evidence',label:'Missing or unresolved evidence',status:'uncertified',values:limitations.length?limitations:
        [ready?'No source-specific independent trade audit is certified by this section.':'No complete source-faithful, independently verified backtest is established by this assessment.']},
      {key:'optimization',label:'Optimization assessment',status:ready?'unassessed':'withheld',values:[ready?
        'This computed run does not establish optimized candidates. Any parameter ranking must identify its actual search, eligibility rules and selection period.':
        'Optimization is withheld. No candidate ranking or performance conclusion is available without a supported source-faithful backtest.']},
      {key:'prerequisite',label:'Next prerequisite',status:'required',values:[ready?
        'Complete the source-specific independent trade audit before describing this result as source-certified. Parameter selection and holdout evidence must remain distinct.':
        hasRecord?'Resolve the recorded rule and data limitations, then independently verify the supported trade results before publishing optimization or performance.':
        'Establish source-specific rule and data evidence, then independently verify a supported backtest before publishing optimization or performance.']}
    ];
  }
  function review(strategy,result,issue,evidence){
    const ready=!!result&&Array.isArray(result.dates)&&result.dates.length>1&&Number.isFinite(result.stats?.trades)&&result.stats.trades>=0;
    const facts=[],caveats=[];
    if(!ready)facts.push(issue?.note||evidence?.reason||'No supported backtest result is available for this selection.');
    else facts.push('This is a computed run on the selected instrument and dates. Data admission and regression checks do not certify complete source fidelity.');
    if(evidence?.pages)facts.push('Source location: '+evidence.pages+'.');
    if(evidence?.conclusion)facts.push(evidence.conclusion);
    if(!ready)caveats.push('No independent trade-by-trade performance verification has been completed for this method. Charts, rankings and performance exports remain withheld.');
    else {
      caveats.push('No source-specific independent trade audit is certified by this section. Book claims and selected chart examples must remain separate from this run.');
      if(result.stats.trades<30)caveats.push('Only '+result.stats.trades+' closed trades are reported; the sample is too small for a firm performance conclusion.');
      caveats.push('The automatic full-history parameter ranking examines the same history used to select settings. A before/after display of those fixed choices is not an untouched holdout test.');
    }
    if(strategy?.guru==='ross')caveats.push('Cash-index bars do not establish futures ticks, contract costs, intraday order sequencing or judgment-driven management. Full corpus review remains in progress.');
    return {verdict:ready?'Computed result; source fidelity not certified':'Backtest withheld',evidence:facts,caveats,links:(evidence?.external||[]).filter(sourceLink),assessment:assessment(ready,issue,evidence)};
  }
  return {review,blockers};
});
