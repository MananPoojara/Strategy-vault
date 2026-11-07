// Preserved, tested research presenters and widget integration. React owns the app and page DOM.
import {vaultStore} from '../state/store';
import {resolveRoute} from '../app/routes';
import {customPages} from '../app/custom-pages';
import {LIB} from './catalogue.js';
import {ATH_VERIFY} from './ath-verification.js';
import {ATH_CARD} from './ath-card-results.js';
import {ATH_MASTER_IDS} from './ath-masters.js';
import {btstVerify} from './btst-verification.js';
import {ADR_VERIFY} from './adr-verification.js';
import {buildTsWorkbook, maeOf, maeBars, tsBytes} from './ts-workbook.js';
import {buildReport, yoyMonthly} from './report-stats.js';
import {reportHTML, statisticsBlocks, tradeRows, TRADE_COLUMNS, yoyAoa} from './report-view.js';
import {buildLabXlsx} from './lab-xlsx.js';
import {labRows, resultsHTML, listHTML} from './optimize-view.js';
import {AM_COLUMNS, buildOptimizationXlsx, optimsFileName, rangeLine, zipStored} from './optimize-sheet.js';
import {authorProfileHtml} from '../services/author-profiles';
import {loadOwnerRows} from '../services/summary-loader';
import {bookContext} from '../services/book-context';
import {returnDisplay} from '../services/return-display';
import {sourceAccount,sourceAccountView,sourceTradeHeaders,sourceAccountTables,sourceAccountCopy} from '../services/source-account';
import {BANDY_SIMPLE_METHOD,simpleInstrumentContext} from '../services/bandy-source-account';
import {unitExposureMethod,unitExposureInstrumentContext,validatedUnitExposureReport} from '../services/unit-exposure-report';
import {PRISTINE_BOOK,PRISTINE_RESEARCH_MODE,pristineMethod,pristineInstrumentContext,validatedPristineReport,isValidatedPristinePair} from '../services/pristine-report';
import {pristineWorkbook,pristineTradeTable} from '../services/pristine-presenter';
import {UNIT_VISUAL_BASIS,UNIT_CLOSED_BASIS,unitExposureTables,unitExposureMetadata,unitPriceSignals} from '../services/unit-exposure-presenter';
import {tradeExitCheck,tradeExitCheckMarkup} from '../services/trade-exit-check';
import {stockChoices,basketSpan,positionLimit,priceRecoveryPeriod} from '../services/stock-selection';
import {identifiedAuthorCount} from '../services/author-identities';
import {libraryTotals} from '../services/source-review';
import {libraryProgress,reasonLabel} from '../services/library-progress';
import {visibleBacktestCount,measuredWindow} from '../services/library-count-context';
import {unavailableBookMethods} from '../services/unavailable-book-methods';
import {libraryScope,inLibraryScope} from '../services/library-scope';
import {libraryMarket,libraryMarketVariants} from '../services/library-market';
import {strategyDataWindow} from '../services/series-coverage';
import {BEST_SETTING, NO_TRADESHEET, bestActive, btstReference, mainTradesheet, referenceDefaults, universeReferenceDefaults, universeTradesheet} from '../services/btst-reference';
import {athOptParams, athOptimizeQuery, carMdd, comboCount, defaultRange, gridProblem, optGridHeight, OPT_ROWS_SHOWN, rangeValues, sortRows as athSortRows, validAthResult} from '../services/ath-optimize';
import {UNIVERSE_PERCENT_COLUMNS, UNIVERSE_TRADE_HEADERS, isUniverseOnly, normaliseRunScope, positionStats, universeAflRows, universeChoiceLabel, universeQuery, universeRunnable, universeSchema, universeText, universeTradeRows, tradePage, universeTradedLine} from '../services/btst-universe';
import {AFL_PARAMS_ALL as AFL_PARAMS} from '../services/afl-params-all';
import {ATH_CHECKED_IDS} from '../services/ath-reference';
import {ytFamilyFolder, ytFamilyReference, ytReferenceDefaults, ytRefPath, ytUniverseReferenceDefaults} from '../services/yt-reference';
import {btstFuturesSymbol, btstInstrument} from '../services/btst-instrument';
import {paramValues, renderRule} from '../services/rule-template';
import {PLAIN_LABELS, changedParams, clampParam, hhmm, inactiveParams, isCoded, paramOptions, paramSections, paramTitle, plainOptions, strategyKeys, timeOptions} from '../services/param-dialog';
import {MAIN_VARIANT_NOTES, YT_PRESETS, presetActive, ytMainVariant, ytMainVariants} from '../services/yt-family';
import {isIntradayStrategy, layerMarkers, lowerBound, tradeLayers, tradeSpans, tradeWindow} from '../services/intraday-trade';
import {orderAuthors} from '../services/author-order';
import {chooseBacktestContext,referenceBacktestContexts} from '../services/library-backtest-context';
import {instrumentControls,instrumentTickers} from '../services/instrument-controls';
import {catalogueMethodLabel,kestnerSourceAssessment} from '../services/kestner-presentation';
import {makeRuleVisualModel} from '../services/rule-evidence';
import {buildStrategyIndicators} from '../services/strategy-indicators';
import {isKestnerVisualMethod,kestnerReportLogicalRange} from '../services/kestner-visual';
import {indicatorControlsHTML,mountStrategyIndicators} from '../services/strategy-indicator-charts';
const CURATED_AUTHOR_IDS=new Set(LIB.map(author=>author.id));
function sourceAdmission(definition,ticker,parameters,region){
  if(definition.id!==BANDY_SIMPLE_METHOD)return undefined;
  const rows=region==='us'?TICK_ALL_US:TICK_ALL_IN;
  const etfs=region==='us'?rows.filter(row=>['SPY','QQQ','MDY','IWM','SMH','DIA'].includes(row.ticker)):[];
  return {definition,parameters,instrument:simpleInstrumentContext(ticker,rows,etfs)};
}

function unitAdmission(definition,ticker,parameters,region,state){
 const rows=region==='us'?TICK_ALL_US:TICK_ALL_IN,indices=region==='us'?TICK_US:TICK_IN;
 return {id:definition.id,book:definition.book,ticker,region,definition,parameters,
  instrument:unitExposureInstrumentContext(ticker,region,rows,indices||[]),
  costs:{fees_bps:Number(state.cost||0),slippage_bps:Number(state.slippage||0),rollover_bps:Number(state.rollover||0)},
  window:{from:state.from||'',to:state.to||''}};
}
function unitFor(st){
 if(!unitExposureMethod(st.v.id,st.v.book)&&!Object.hasOwn(st.d,'unit_exposure_report'))return null;
 return validatedUnitExposureReport(st.d,unitAdmission(st.v,st.tk,st.params||st.indicatorParameters,st.region,st.state||st));
}

function pristineCost(value){
 const token=value===undefined?'0':value;
 if(typeof token!=='string'||!/^[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?$/.test(token)||!Number.isFinite(Number(token)))throw Error('Use a finite numeric execution cost; unknown cost settings are not replaced.');
 return Number(token);
}
function pristineExecution(state){return {slippage:state.slippage??'0',rollover:state.rollover??'0'};}
function pristineAdmission(definition,ticker,state){
 if(state.p)throw Error('These fixed source branches accept no parameter settings.');
 const rows=state.region==='us'?TICK_ALL_US:TICK_ALL_IN,window={};
 if(state.from)window.from=state.from;if(state.to)window.to=state.to;
 return {id:definition.id,book:definition.book,ticker,region:state.region,researchMode:state.research_mode||'',definition,parameters:{},instrument:pristineInstrumentContext(ticker,state.region,rows),costs:{fees_bps:pristineCost(state.cost),slippage_bps:pristineCost(state.slippage),rollover_bps:pristineCost(state.rollover)},window};
}
function pristineCurrent(data,view){
 if(!STRAT?.pristine||STRAT.pristine.data!==data||STRAT.pristine.view!==view||!isValidatedPristinePair(data,view)||R.name!=='strategy'||R.optim||R.id!==STRAT.v.id||S.tk!==STRAT.tk||S.region!==STRAT.region||S.run_scope!=='single'||!!S.p)return false;
 const definition=ALLCAT.find(row=>row.id===R.id);if(!definition)return false;
 try{return validatedPristineReport(data,pristineAdmission(definition,S.tk,S))===view;}catch{return false;}
}

/* Report controls use recorded engine results; unavailable inputs stay empty. */
window.VaultUI=(()=>{
 const escText=s=>String(s??'').replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
 const costState=st=>({slippage:st.slippage||'0',rollover:st.rollover||'0'});
 const label=(n,type)=>typeof n!=='number'||!Number.isFinite(n)?'—':type==='percent'?(n*100).toFixed(2)+'%':n.toFixed(type==='count'?0:type==='days'?1:2);
 let currentBenchmark=null;
 const tradeHeadersFor=d=>d.source_account?sourceTradeHeaders(VaultReport.tradeHeaders):VaultReport.tradeHeaders;
 const tradeRowsFor=(d,tk)=>VaultReport.tradeRows(d,tk).map((row,i)=>(d.source_account||d.unit_exposure_report)?row.map((value,column)=>column===13?d.trades[i].ret_full_precision:value):row).map((row,i)=>{const t=d.trades?.[i];if(!t?.entry_ts)return row;const r=[...row];r[2]=t.entry_ts;if(t.exit_ts&&!t.open)r[4]=t.exit_ts;return r;});
 // Icons: Lucide (ISC) line icons and Simple Icons (CC0) brand marks, inlined from lucide-static@1.55.0 / simple-icons@16.35.0.
 const ICON={"share": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"18\" cy=\"5\" r=\"3\" /> <circle cx=\"6\" cy=\"12\" r=\"3\" /> <circle cx=\"18\" cy=\"19\" r=\"3\" /> <line x1=\"8.59\" x2=\"15.42\" y1=\"13.51\" y2=\"17.49\" /> <line x1=\"15.41\" x2=\"8.59\" y1=\"6.51\" y2=\"10.49\" /></svg>", "download": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z\" /> <path d=\"M14 2v5a1 1 0 0 0 1 1h5\" /> <path d=\"M12 18v-6\" /> <path d=\"m9 15 3 3 3-3\" /></svg>", "save": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M17 3a2 2 0 0 1 2 2v15a1 1 0 0 1-1.496.868l-4.512-2.578a2 2 0 0 0-1.984 0l-4.512 2.578A1 1 0 0 1 5 20V5a2 2 0 0 1 2-2z\" /></svg>", "saved": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M17 3a2 2 0 0 1 2 2v15a1 1 0 0 1-1.496.868l-4.512-2.578a2 2 0 0 0-1.984 0l-4.512 2.578A1 1 0 0 1 5 20V5a2 2 0 0 1 2-2z\" /> <path d=\"m9 10 2 2 4-4\" /></svg>", "back": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m12 19-7-7 7-7\" /> <path d=\"M19 12H5\" /></svg>", "png": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><rect width=\"18\" height=\"18\" x=\"3\" y=\"3\" rx=\"2\" ry=\"2\" /> <circle cx=\"9\" cy=\"9\" r=\"2\" /> <path d=\"m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21\" /></svg>", "xls": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z\" /> <path d=\"M14 2v5a1 1 0 0 0 1 1h5\" /> <path d=\"M8 13h2\" /> <path d=\"M14 13h2\" /> <path d=\"M8 17h2\" /> <path d=\"M14 17h2\" /></svg>", "csv": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z\" /> <path d=\"M14 2v5a1 1 0 0 0 1 1h5\" /> <path d=\"M10 9H8\" /> <path d=\"M16 13H8\" /> <path d=\"M16 17H8\" /></svg>", "link": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 17H7A5 5 0 0 1 7 7h2\" /> <path d=\"M15 7h2a5 5 0 1 1 0 10h-2\" /> <line x1=\"8\" x2=\"16\" y1=\"12\" y2=\"12\" /></svg>", "mail": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m22 7-8.991 5.727a2 2 0 0 1-2.009 0L2 7\" /> <rect x=\"2\" y=\"4\" width=\"20\" height=\"16\" rx=\"2\" /></svg>", "check": "<svg class=\"ic ic-line\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M20 6 9 17l-5-5\" /></svg>", "wa": "<svg class=\"ic ic-brand\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413Z\"/></svg>", "x": "<svg class=\"ic ic-brand\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M14.234 10.162 22.977 0h-2.072l-7.591 8.824L7.251 0H.258l9.168 13.343L.258 24H2.33l8.016-9.318L16.749 24h6.993zm-2.837 3.299-.929-1.329L3.076 1.56h3.182l5.965 8.532.929 1.329 7.754 11.09h-3.182z\"/></svg>", "tg": "<svg class=\"ic ic-brand\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z\"/></svg>", "gmail": "<svg class=\"ic ic-brand\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M24 5.457v13.909c0 .904-.732 1.636-1.636 1.636h-3.819V11.73L12 16.64l-6.545-4.91v9.273H1.636A1.636 1.636 0 0 1 0 19.366V5.457c0-2.023 2.309-3.178 3.927-1.964L5.455 4.64 12 9.548l6.545-4.91 1.528-1.145C21.69 2.28 24 3.434 24 5.457z\"/></svg>"};
 // Like: Lucide thumbs-up, ISC; verified official 0.557.0 SVG on 2026-10-13.
 // https://raw.githubusercontent.com/lucide-icons/lucide/0.557.0/icons/thumbs-up.svg
 ICON.like='<svg class="ic ic-line" viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden="true"><path d="M15 5.88 14 10h5.83a2 2 0 0 1 1.92 2.56l-2.33 8A2 2 0 0 1 17.5 22H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h2.76a2 2 0 0 0 1.79-1.11L12 2a3.13 3.13 0 0 1 3 3.88Z"/><path d="M7 10v12"/></svg>';
 const likesStorageKey=()=>`vault-likes:${window.VAULT_USER?.id!=null?encodeURIComponent(String(window.VAULT_USER.id)):'browser'}`;
 const likedList=()=>{try{const list=JSON.parse(localStorage.getItem(likesStorageKey())||'[]');return Array.isArray(list)?list.filter(key=>typeof key==='string'):[];}catch{return [];}};
 const isLiked=()=>!!savedKey()&&likedList().includes(savedKey());
 function toggleLike(){const key=savedKey();if(!key)return;const list=new Set(likedList()),on=!list.has(key);if(on)list.add(key);else list.delete(key);
  try{localStorage.setItem(likesStorageKey(),JSON.stringify([...list]));}catch{msg('Could not store Like in this browser.');return;}
  const button=document.querySelector('[data-action=like]');if(button){button.classList.toggle('on',on);button.setAttribute('aria-pressed',String(on));button.title=on?'Remove Like (stored in this browser)':'Like this strategy (stored in this browser)';button.innerHTML=ICON.like;}
  msg(on?'Liked. Your preference is stored in this browser.':'Like removed from this browser.');
 }
 // Saved strategies: a per-browser bookmark list (strategy, index and settings live in the saved link).
 const SAVED_KEY='vault-saved';
 const savedList=()=>{try{return JSON.parse(localStorage.getItem(SAVED_KEY)||'[]');}catch{return [];}};
 const writeSaved=list=>{try{localStorage.setItem(SAVED_KEY,JSON.stringify(list));}catch{}document.dispatchEvent(new CustomEvent('vault-saved'));};
 const savedKey=()=>R.name==='strategy'?`${R.id}|${S.region}|${S.tk}|${S.basket||''}`:'';
 const isSaved=()=>savedList().some(x=>x.key===savedKey());
 function actions(){return '';}
 // Strategy-page toolbar: Share (menu), Download (PDF of the page), Save (bookmark).
 function toolbar(){const saved=isSaved(),liked=isLiked();return `<div class="st-actbox"><div class="st-actions" role="toolbar" aria-label="Page actions">
  <button type="button" class="sa${liked?' on':''}" data-action="like" aria-pressed="${liked}" aria-label="Like" title="${liked?'Remove Like':'Like this strategy'} (stored in this browser)">${ICON.like}</button>
  <div class="sa-wrap"><button type="button" class="sa" data-action="share-menu" aria-label="Share" aria-haspopup="menu" aria-expanded="false" title="Share">${ICON.share}</button>
   <div class="share-menu" role="menu" hidden>
    <button type="button" role="menuitem" data-share="pdf">${ICON.download}<span>Send as PDF<small>Share sheet on phone, Windows and Mac</small></span></button>
    <div class="sm-sep"></div>
    <button type="button" role="menuitem" data-share="wa" class="br-wa">${ICON.wa}<span>WhatsApp</span></button>
    <button type="button" role="menuitem" data-share="x" class="br-x">${ICON.x}<span>X</span></button>
    <button type="button" role="menuitem" data-share="tg" class="br-tg">${ICON.tg}<span>Telegram</span></button>
    <button type="button" role="menuitem" data-share="mail" class="br-mail">${ICON.gmail}<span>Email</span></button>
    <div class="sm-sep"></div>
    <button type="button" role="menuitem" data-share="copy">${ICON.link}<span>Copy link</span></button>
   </div></div>
  <button type="button" class="sa" data-action="pdf" aria-label="Download page PDF" title="Download this page as PDF">${ICON.download}</button>
  <button type="button" class="sa${saved?' on':''}" data-action="save" aria-label="Save" aria-pressed="${saved}" title="${saved?'Remove from Saved':'Save to your Saved list'}">${saved?ICON.saved:ICON.save}</button><span class="sa-settings" data-react-slot="strategy-settings"></span>
 </div><span role="status" id="page-action-message"></span></div>`;}
 function detailHTML(d){return `<details class="box detailed-stats" id="detailed-stats"><summary>Detailed statistics <span>Additional gross / net measures</span></summary><div class="detail-tools"><button class="dbtn sm" data-report-export="details">Download Excel</button></div><table class="tbl detailed-table"><thead><tr><th>Statistic</th><th class="n">Net</th><th class="n">Gross</th></tr></thead><tbody>${VaultReport.detailRows(d).map(([k,n,g,t])=>`<tr><td>${escText(k)}</td><td class="n">${label(n,t)}</td><td class="n">${label(g,t)}</td></tr>`).join('')}</tbody></table><p class="rnote">${d.source_account?'Gross retains the same accepted quantities and fills, before paid costs; no re-selection or resizing.':d.kind==='basket'?'Gross is zero-cost accounting on the same accepted portfolio fills.':'Gross is the independently compounded zero-cost comparison.'} Net includes the selected fee and slippage rates. Closed-trade measures exclude open positions. Observed excursions omit entry/add-on/exit event bars where intraday order is unknown; they are partial bounds, not exact MAE/MFE. Time in market counts distinct observed daily bars with a held position, including entry/exit bars; it does not measure exact intraday duration. Undefined ratios stay unavailable. ${d.unit_exposure_report?escText(UNIT_VISUAL_BASIS):''}</p></details>`;}
 function benchmarkHTML(d,tk,px){currentBenchmark={d,tk,px};const own=!!(d.instrument&&d.benchmark_symbol&&d.benchmark_stats); /* BTST: the engine's buy & hold of the futures traded */const b=(d.unit_exposure_report||own)?{...d.benchmark_stats,from:d.dates[0],to:d.dates.at(-1)}:VaultReport.benchmark(px,d.dates[0],d.dates.at(-1)),bn=own?d.benchmark_symbol:(LABEL[tk]||tk),keys=[[d.source_account?sourceAccountCopy(d.source_account).accountReturn:d.kind==='basket'?'Portfolio equity return':'Compounded model return','total_return','percent'],['Annualised return','cagr','percent'],['Max drawdown','max_dd','percent'],['Sharpe (rf = 0)','sharpe','ratio'],['Sortino','sortino','ratio'],['Return per drawdown','mar','ratio'],['Annualised volatility','vol','percent'],['Exposure','exposure','percent']];return `<section class="box" data-benchmark-comparison data-benchmark-symbol="${escText(own?bn:tk)}"><div class="box-h">Strategy vs ${escText(bn)} <button class="dbtn sm" data-report-export="benchmark">Download Excel</button></div><p class="rnote">Computed selected strategy settings against actual ${escText(bn)} buy &amp; hold, with no benchmark execution costs. ${b?`${b.from} to ${b.to}.`:'Benchmark price coverage is missing.'}${own&&d.benchmark_basis?` ${escText(d.benchmark_basis)}.`:''} Book-reported figures remain separate reference claims.</p><table class="tbl"><thead><tr><th>Statistic</th><th class="n">Selected strategy</th><th class="n">${escText(bn)} buy &amp; hold</th></tr></thead><tbody>${keys.map(([name,key,type])=>`<tr><td>${name}</td><td class="n">${label(d.stats[key],type)}</td><td class="n">${label(b?.[key],type)}</td></tr>`).join('')}</tbody></table></section>`;}
 function tradeHTML(d,tk){const adr=adrReport(d),rows=tradeRowsFor(d,tk).map((row,n)=>adr&&adr.trades[n]?[...row,adr.trades[n].bars,adr.trades[n].accountDd]:row);return `<section class="box" data-tradesheet><div class="box-h">Tradesheet <span class="legend"><small>${rows.length} positions · newest first</small><button class="dbtn sm" id="trades-xlsx">Download Excel</button><button class="dbtn sm" id="csv">Download CSV</button>${/^ytadr_/.test(STRAT?.v?.id||'')?'<button class="dbtn sm" id="ts-download" title="The research team\'s Excel tradesheet: every number is a live formula; costs in C1:E1 are editable">Download TS</button>':''}${d.source_account?'<button class="dbtn sm" data-report-export="account-audit">Account audit Excel</button>':d.unit_exposure_report?'<button class="dbtn sm" data-report-export="unit-audit">Unit audit Excel</button><button class="dbtn sm" data-report-export="unit-daily">Daily audit Excel</button>':''}</span></div><p class="rnote">${d.source_account?sourceAccountCopy(d.source_account).tradeNote:d.unit_exposure_report?'Audited canonical-decimal return fractions; terminal-open costs include an estimated second liquidation side. Gross compounds independently at zero cost; contributions use net pretrade allocation.':'Returns and slippage are percentages of the strategy’s normalized allocation; prices/points use the actual symbol.'} ${d.source_account?'Monetary P&amp;L and Taxes are unavailable: the model has no funded capital, actual share lots or statutory tax schedule.':'Monetary Gross/Net P&amp;L and Taxes are unavailable because no quantity, monetary sizing or tax schedule was supplied.'} MAE/MFE show gross observed daily-bar bounds when available; event bars are excluded. Open exits are marked Open.</p><div class="tlog"><table class="tbl tl" id="tradesheet"><thead><tr>${[...tradeHeadersFor(d),...(adr?['Bars held','DD %']:[])].map(k=>`<th>${escText(k)}</th>`).join('')}</tr></thead><tbody>${rows.map((row,tradeIndex)=>`<tr>${row.map((v,i)=>`<td class="${[8,11,13,14,15].includes(i)||(adr&&i===18)?'n '+(v<0?'neg':'pos'):adr&&i===17?'n':''}">${v===null?'—':[8,11,13,14,15].includes(i)||(adr&&i===18)?label(v,'percent'):adr&&i===17?String(v):typeof v==='number'?v.toFixed(i===9?0:2):escText(v)}${i===16?tradeExitCheckMarkup(d.trades[tradeIndex]):''}</td>`).join('')}</tr>`).reverse().join('')}</tbody></table></div></section>`;}
 function equityNavigator(ch,d){const box=document.getElementById('equity-navigator');if(!box)return;const a=box.querySelector('[data-range=start]'),b=box.querySelector('[data-range=end]'),text=box.querySelector('output'),n=d.dates.length;if(n<2){a.disabled=b.disabled=true;text.textContent=d.dates[0]||'No dated results';return;}a.max=b.max=n-1;a.value=0;b.value=n-1;const show=()=>{let lo=+a.value,hi=+b.value;if(lo>=hi){if(document.activeElement===a)lo=Math.max(0,hi-1);else hi=Math.min(n-1,lo+1);}a.value=lo;b.value=hi;ch.timeScale().setVisibleLogicalRange({from:lo,to:hi});text.textContent=`${d.dates[lo]} — ${d.dates[hi]}`;};a.oninput=b.oninput=show;box.querySelectorAll('[data-equity-year]').forEach(btn=>btn.onclick=()=>{const y=btn.dataset.equityYear;if(y==='all'){a.value=0;b.value=n-1;}else{const indices=d.dates.map((x,i)=>x.startsWith(y)?i:-1).filter(i=>i>=0);a.value=indices[0];b.value=indices.at(-1);}show();});ch.timeScale().subscribeVisibleLogicalRangeChange(range=>{if(!range)return;const lo=Math.max(0,Math.min(n-1,Math.ceil(range.from))),hi=Math.max(lo,Math.min(n-1,Math.floor(range.to)));a.value=lo;b.value=hi;text.textContent=`${d.dates[lo]} — ${d.dates[hi]}`;});show();}
 const msg=text=>{const el=document.getElementById('page-action-message');if(el)el.textContent=text;};
 const immutable=value=>{if(Array.isArray(value))return Object.freeze(value.map(immutable));if(value&&typeof value==='object')return Object.freeze(Object.fromEntries(Object.entries(value).map(([key,v])=>[key,immutable(v)])));return value;};
 function snapshotContext(){
  if(!STRAT?.d||!STRAT?.v)throw Error('No completed backtest is available for export.');
  const state=immutable(S),region=STRAT.region||state.region,unit=unitFor({...STRAT,state,params:STRAT.indicatorParameters,region}),d=unit?STRAT.d:immutable(STRAT.d),v=immutable(STRAT.v);
  const benchmark=currentBenchmark?.d===STRAT.d?currentBenchmark:null;
  const benchmarkTk=document.querySelector('[data-benchmark-comparison]')?.getAttribute('data-benchmark-symbol')||benchmark?.tk||(region==='us'?'SPX':'NIFTY 50');
  return Object.freeze({d,v,unit,display:immutable(STRAT.display),tk:STRAT.tk,region,state,url:location.href,params:immutable(STRAT.account||unit?STRAT.indicatorParameters:STRAT.d.params||state.p||{}),px:immutable(STRAT.px),
   actualSymbol:d.kind==='basket'?d.basket:d.kind==='universe'?d.universe:d.provenance?.symbol||d.trades?.find(t=>t.accounting?.symbol)?.accounting.symbol||STRAT.tk,
   benchmarkTk,benchmarkLabel:LABEL[benchmarkTk]||benchmarkTk,benchmarkPx:immutable(benchmark?.px)});
 }
 const filePart=value=>String(value??'report').replace(/[^a-zA-Z0-9._-]+/g,'-').replace(/^-+|-+$/g,'').slice(0,90)||'report';
 const reportName=(st,kind,extension)=>`${filePart(st.v.id)}-${filePart(st.actualSymbol)}-${filePart(st.d.dates?.[0]||'period')}-${filePart(st.d.dates?.at(-1)||'end')}-${filePart(kind)}.${extension}`;
 function downloadBlob(blob,name){const link=document.createElement('a'),url=URL.createObjectURL(blob);link.href=url;link.download=name;document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),4000);}
 const csvSafeRows=rows=>rows.map(row=>row.map(v=>typeof v==='string'&&/^[\s\uFEFF]*[=+\-@]/.test(v)?"'"+v:v));
 const cellValue=v=>typeof v==='number'?(Number.isFinite(v)?v:null):v==null?null:typeof v==='string'||typeof v==='boolean'?v:String(v);
 function styledSheet(rows,percentColumns=[],rowTypes=[],precisionColumns=[]){
  const clean=rows.map(row=>row.map(cellValue)),sheet=XLSX.utils.aoa_to_sheet(clean),width=rows.reduce((n,r)=>Math.max(n,r.length),0);
  for(let r=0;r<rows.length;r++)for(let c=0;c<width;c++){
   const address=XLSX.utils.encode_cell({r,c}),value=clean[r]?.[c];if(!sheet[address])sheet[address]={t:'z'};const cell=sheet[address];
   // a plain date (YYYY-MM-DD, e.g. Entry date / Exit date) is written as an Excel date, so it sorts, filters and works in date formulas
   const isDate=r>0&&typeof value==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(value);
   if(isDate){const[y,m,d]=value.split('-').map(Number);cell.t='n';cell.v=Date.UTC(y,m-1,d)/86400000+25569;delete cell.f;}
   else if(typeof value==='string'){cell.t='s';cell.v=value;delete cell.f;}
   const percent=r>0&&typeof value==='number'&&(percentColumns.includes(c)||rowTypes[r]==='percent');
   const z=isDate?'dd-mm-yyyy':percent?'0.00%;[Red]-0.00%;0.00%':rowTypes[r]==='count'?'0':rowTypes[r]==='days'?'0.0':r>0&&precisionColumns.includes(c)?'0.000000':'0.00';
   cell.s={font:{name:'Calibri',sz:11,color:{rgb:r===0?'FFFFFF':typeof value==='number'&&value<0?'C00000':'000000'},bold:r===0},
    fill:{fgColor:{rgb:r===0?'1F3864':r%2?'FFFFFF':'F3F6FA'}},alignment:{vertical:'center',horizontal:typeof value==='number'||isDate?'right':'left',wrapText:true},
    border:{bottom:{style:'thin',color:{rgb:'DCE2EA'}}},...(typeof value==='number'||isDate?{numFmt:z}:{})};if(typeof value==='number'||isDate)cell.z=z;
  }
  sheet['!cols']=Array.from({length:width},(_,i)=>({wch:clean.reduce((n,row)=>Math.max(n,typeof row[i]==='string'?Math.min(55,row[i].length+2):0),i===0?20:14)}));
  sheet['!rows']=[{hpt:30}];if(rows.length&&width){sheet['!autofilter']={ref:XLSX.utils.encode_range({s:{r:0,c:0},e:{r:rows.length-1,c:width-1}})};}sheet['!freeze']={xSplit:0,ySplit:1,topLeftCell:'A2',activePane:'bottomLeft',state:'frozen'};
  return sheet;
 }
 async function xlsx(kind,rows,percentColumns,st,rowTypes=[],extraSheets=[]){
  msg('Preparing Excel…');await (typeof loadXlsx==='function'?loadXlsx():loadScript('vendor/xlsx-js-style-1.2.0.min.js'));
  if(!window.XLSX?.utils)throw Error('The Excel library did not load.');
  const unit=unitFor(st),unitTables=unit?unitExposureTables(unit):null;
  const account=sourceAccount(st.d,st.v.id,st.v.book,st.tk,sourceAdmission(st.v,st.tk,st.params,st.region)),tables=account?sourceAccountTables(account,st.d.trades.map(t=>t.ret_full_precision)):null,copy=account?sourceAccountCopy(account):null;
  const book=XLSX.utils.book_new(),names={trades:'Trades','detailed-statistics':'Detailed statistics','monthly-returns':'Monthly returns',equity:'Equity',benchmark:'Benchmark','account-audit':'Account audit','unit-audit':'Unit return audit','unit-daily':'Unit daily report'};
  XLSX.utils.book_append_sheet(book,styledSheet(rows,percentColumns,rowTypes,kind==='account-audit'?tables.audit.numericColumns:kind==='unit-audit'?unitTables.audit.numericColumns:kind==='unit-daily'?unitTables.daily.numericColumns:[]),names[kind]||'Report');
  for(const sheet of extraSheets)XLSX.utils.book_append_sheet(book,styledSheet(sheet.rows,[],[]),sheet.name);
  const costs=st.d.costs||{},provenance=st.d.provenance||{},context=[['Field','Value'],['Report',kind],['Strategy ID',st.v.id],['Strategy',st.v.name||st.v.family_name],['Book',st.v.book],['Actual symbol',st.actualSymbol],['Selected data key',st.tk],['Market',st.region],['From',st.d.dates?.[0]],['To',st.d.dates?.at(-1)],['Execution fees (bps per side)',costs.fees_bps??null],['Slippage (bps per side)',costs.slippage_bps??null],['Rollover (bps per event)',costs.rollover_bps??null],['Benchmark symbol',st.benchmarkTk],['Report link',st.url],[st.d.source_account?'Closed weighted-return display':'Net Profit display',st.display?.reportMode??'Portfolio realized contribution'],[st.d.source_account?'Displayed closed weighted return (fraction)':'Displayed Net Profit (fractional return)',st.display?st.display.value:st.d.stats?.net_profit??null],['Display formula',st.display?.description??st.d.accounting_basis??null],['Display unavailable reason',st.display?.unavailableReason??null],['Trade count basis','Closed positions; open exits remain marked Open.'],['Accounting basis',st.d.accounting_basis||'Recorded return-only engine report. No monetary sizing or statutory tax schedule is supplied.']];
  const sources=[['Field','Value'],['Source',provenance.source??null],['Basket',provenance.basket??null],['Source SHA-256',provenance.source_sha256??null],['Derived CSV SHA-256',provenance.csv_sha256??null],['Membership',provenance.membership??null],['Source coverage from',provenance.first??null],['Source coverage to',provenance.last??null],['Excluded source prefix',provenance.excluded_prefix_count??null],['Coverage note',provenance.coverage_note??null],['Qualification','Source identity and format admission are not certification of book authorship, historical membership or reproduced performance.'],['Unavailable fields','Monetary P&L and taxes stay empty. Observed excursions are partial daily-bar bounds; event bars cannot establish exact intraday MAE/MFE.'],['Execution costs',costs.basis??null]];
  if(account){
   context.push([copy.bandy?'Actual cash-account return (fraction)':'Actual risk-sized account return (fraction)',st.d.stats.total_return],['Gross same-quantity account return (fraction)',st.d.gross_stats?.total_return??null],['Closed-return basis','Accepted allocation-weighted net fractions of each actual entry equity; unrounded Avg × closed count equals the closed sum, not actual account return.'],['Account curve basis',copy.exportCurve],['Display-only mode',copy.exportDisplay],['Gross basis','Same accepted quantities/fills; paid costs removed only.'],['Account units','Dimensionless initial model unit1; no monetary capital, actual shares or tax certification.']);
   if(copy.bandy)context.push(['Account model',account.model],['Target equity fraction',account.sizing.target_equity_fraction],['Sizing origin',account.sizing.origin],['Cash reservation','Entry costs reserved; no added funding.'],['Terminal entry pending',account.terminal_entry_pending],['Terminal exit pending',account.terminal_exit_pending],['Uncharged open liquidation estimate (initial unit)',account.trade_accounts.reduce((sum,t)=>sum+t.open_liquidation_cost_estimate,0)],['Paid-cost basis','Actual entry/exit notionals on filled sides only. Estimates are not paid costs.'],['Observed-time basis','Exposure includes entry-through-exit observed sessions; drawdown duration counts observed sessions, not calendar days or intraday duration.'],['Closed zero-return grouping','Zero-return closed trades join non-wins in loss statistics.'],['Gross trade fraction denominator','Actual net account equity immediately before entry; not a separately resized gross account.']);
   if(kind!=='account-audit')XLSX.utils.book_append_sheet(book,styledSheet([tables.audit.headers,...tables.audit.rows],tables.audit.percentColumns,[],tables.audit.numericColumns),tables.audit.name);
   XLSX.utils.book_append_sheet(book,styledSheet([tables.daily.headers,...tables.daily.rows],[],[],tables.daily.numericColumns),tables.daily.name);
  }
  if(unit){
   context.push(...unitExposureMetadata(unit));
   if(kind!=='unit-audit')XLSX.utils.book_append_sheet(book,styledSheet([unitTables.audit.headers,...unitTables.audit.rows],unitTables.audit.percentColumns,[],unitTables.audit.numericColumns),unitTables.audit.name);
   if(kind!=='unit-daily')XLSX.utils.book_append_sheet(book,styledSheet([unitTables.daily.headers,...unitTables.daily.rows],unitTables.daily.percentColumns,[],unitTables.daily.numericColumns),unitTables.daily.name);
  }
  XLSX.utils.book_append_sheet(book,styledSheet(context), 'Metadata');XLSX.utils.book_append_sheet(book,styledSheet(sources),'Sources');
  if(st.d.kind==='basket'){XLSX.utils.book_append_sheet(book,styledSheet([['Symbol','Source key','First','Last','Status','Coverage note'],...(st.d.coverage||[]).map(c=>[c.symbol,c.ticker,c.first,c.last,c.status,c.note])]),'Constituent coverage');XLSX.utils.book_append_sheet(book,styledSheet([['Symbol','Entry','Exit','Open','Initial-equity slot','Source net return','Portfolio contribution'],...(st.d.trades||[]).map(t=>[t.symbol,t.entry,t.exit,t.open,t.allocation,t.ret,t.portfolio_contribution])],[4,5,6]),'Portfolio allocation');XLSX.utils.book_append_sheet(book,styledSheet([['Field','Value'],...Object.entries(st.d.portfolio||{}).filter(([,value])=>!Array.isArray(value))]),'Portfolio method');}
  XLSX.utils.book_append_sheet(book,styledSheet([['Parameter','Value'],...Object.entries(st.params)]),'Parameters');
  const bytes=XLSX.write(book,{bookType:'xlsx',type:'array',cellStyles:true,compression:true});downloadBlob(new Blob([bytes],{type:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'}),reportName(st,kind,'xlsx'));msg('Excel downloaded.');
 }
 async function pristineExport(data,view,kind){
  if(!pristineCurrent(data,view))throw Error('The Pristine response or current controls changed.');
  const safe=value=>typeof value==='string'&&/^[\s\uFEFF]*[=+\-@]/.test(value)?"'"+value:value;
  const filename=`${view.report.id}-${view.report.symbol}-${view.report.rows[0].date}-${view.report.rows.at(-1).date}`.replace(/[^a-zA-Z0-9._-]+/g,'-');
  if(kind==='csv'){
   const table=pristineTradeTable(data,view),rows=[table.headers,...table.rows];
   const csv=rows.map(row=>row.map(value=>'"'+String(safe(value)??'').replace(/"/g,'""')+'"').join(',')).join('\r\n');
   if(!pristineCurrent(data,view))throw Error('The selected report changed.');
   downloadBlob(new Blob([csv],{type:'text/csv;charset=utf-8'}),filename+'-positions.csv');return;
  }
  if(kind!=='workbook')throw Error('Unsupported Pristine export.');
  const tables=pristineWorkbook(data,view);await loadXlsx();
  if(!pristineCurrent(data,view))throw Error('The selected report changed while preparing Excel.');
  const book=XLSX.utils.book_new();
  for(const table of tables){const rows=[table.headers,...table.rows].map(row=>row.map(safe));XLSX.utils.book_append_sheet(book,styledSheet(rows,table.percentColumns,[],table.headers.map((_,i)=>i)),table.name);}
  const bytes=XLSX.write(book,{bookType:'xlsx',type:'array',cellStyles:true,compression:true});
  if(!pristineCurrent(data,view))throw Error('The selected report changed while preparing Excel.');
  downloadBlob(new Blob([bytes],{type:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'}),filename+'.xlsx');
 }
 const canvasBlob=canvas=>new Promise((resolve,reject)=>canvas.toBlob(blob=>blob?resolve(blob):reject(Error('The chart image could not be created.')),'image/png'));
 async function snapshot(id){
  const st=snapshotContext(),el=document.getElementById(id);if(!el)throw Error('This chart is unavailable.');msg('Preparing chart PNG…');
  const registered=typeof CHARTS!=='undefined'?CHARTS.find(c=>c.el?.id===id):null,svg=el.querySelector('svg');
  let source=registered?.ch?.takeScreenshot?.()||el.querySelector('canvas'),width=source?.width,height=source?.height;
  if(!source&&svg){
   const clone=svg.cloneNode(true),box=svg.getBoundingClientRect();width=box.width||svg.viewBox?.baseVal?.width;height=box.height||svg.viewBox?.baseVal?.height;
   clone.setAttribute('xmlns','http://www.w3.org/2000/svg');clone.setAttribute('width',width);clone.setAttribute('height',height);
   const originals=[svg,...svg.querySelectorAll('*')],copies=[clone,...clone.querySelectorAll('*')];
   originals.forEach((node,i)=>{const style=getComputedStyle(node);for(const property of ['fill','stroke','font-family','font-size','font-weight','text-anchor']){const value=style.getPropertyValue(property);if(value)copies[i]?.setAttribute(property,value);}});
   clone.querySelectorAll('script,foreignObject').forEach(node=>node.remove());
   const uri='data:image/svg+xml;base64,'+btoa(unescape(encodeURIComponent(new XMLSerializer().serializeToString(clone))));
   source=await new Promise((resolve,reject)=>{const image=new Image();image.onload=()=>resolve(image);image.onerror=()=>reject(Error('The SVG chart could not be rendered.'));image.src=uri;});
  }
  if(!source||!Number.isFinite(width)||!Number.isFinite(height)||width<=0||height<=0)throw Error('This chart has no recorded image to export.');
  const footer=st.unit?UNIT_VISUAL_BASIS:'',footerHeight=footer?44:0,scale=Math.min(2,Math.sqrt(8000000/(width*(height+footerHeight)))),canvas=document.createElement('canvas');canvas.width=Math.max(1,Math.floor(width*scale));canvas.height=Math.max(1,Math.floor((height+footerHeight)*scale));
  const ctx=canvas.getContext('2d');if(!ctx)throw Error('Canvas export is unavailable.');ctx.fillStyle='#ffffff';ctx.fillRect(0,0,canvas.width,canvas.height);ctx.drawImage(source,0,0,canvas.width,Math.max(1,Math.floor(height*scale)));
  if(footer){ctx.fillStyle='#333333';ctx.font=`${10*scale}px sans-serif`;let line='',y=(height+15)*scale;for(const word of footer.split(' ')){const next=line?line+' '+word:word;if(line&&ctx.measureText(next).width>canvas.width-20*scale){ctx.fillText(line,10*scale,y);line=word;y+=13*scale;}else line=next;}if(line)ctx.fillText(line,10*scale,y);}
  downloadBlob(await canvasBlob(canvas),reportName(st,id,'png'));msg('Chart PNG downloaded.');
 }
 // Real PDF file of the page: html2canvas renders the page, jsPDF splits it into A4 pages (vendored, loaded on first use).
 const scriptLoads=new Map();
 const loadScript=src=>{if(scriptLoads.has(src))return scriptLoads.get(src);const promise=new Promise((ok,fail)=>{const t=document.createElement('script');t.src=src;t.onload=ok;t.onerror=()=>fail(Error('could not load '+src));document.head.append(t);});scriptLoads.set(src,promise);promise.catch(()=>scriptLoads.delete(src));return promise;};
 const pdfName=()=>{const t=(document.querySelector('h1.st')?.textContent||document.title||'strategy-vault').trim();return t.replace(/[^\w\- ]+/g,'').replace(/\s+/g,'-').slice(0,80)+'.pdf';};
// html2canvas 1.4.1 cannot parse CSS color()/color-mix(); the browser resolves those to color(srgb r g b / a),
 // so in the clone it renders, every colour property is rewritten as plain rgba() (the visible page is not touched).
 const toRgb=v=>String(v||'').replace(/color\(srgb\s+([\d.e-]+)\s+([\d.e-]+)\s+([\d.e-]+)(?:\s*\/\s*([\d.e-]+%?))?\)/g,(_,r,g,b,a)=>`rgba(${Math.round(r*255)},${Math.round(g*255)},${Math.round(b*255)},${a==null?1:a.endsWith('%')?parseFloat(a)/100:+a})`);
 const COLOR_PROPS=['color','backgroundColor','backgroundImage','borderTopColor','borderRightColor','borderBottomColor','borderLeftColor','outlineColor','boxShadow','textDecorationColor','fill','stroke','columnRuleColor'];
 function plainColors(doc){const win=doc.defaultView;doc.querySelectorAll('*').forEach(el=>{const cs=win.getComputedStyle(el);for(const prop of COLOR_PROPS){const v=cs[prop];if(v&&v.includes('color('))el.style[prop]=toRgb(v);}});}
 async function pagePdf(){
  const pristine=typeof STRAT!=='undefined'?STRAT?.pristine:null;
  if(pristine?.view&&!pristineCurrent(pristine.data,pristine.view))throw Error('The Pristine report or controls changed.');
  const report=typeof STRAT!=='undefined'&&STRAT?.d?snapshotContext():null,url=location.href,name=report?reportName(report,'page','pdf'):pdfName(),root=document.getElementById('main');
  if(!root)throw Error('The report page is unavailable.');
  const assertSame=()=>{if(location.href!==url||document.getElementById('main')!==root||(pristine?.view&&!pristineCurrent(pristine.data,pristine.view))){const error=Error('The page changed while preparing the PDF. Download it again on the desired report.');error.name='ReportChangedError';throw error;}};
  await loadScript('vendor/html2canvas-1.4.1.min.js');await loadScript('vendor/jspdf-2.5.1.umd.min.js');assertSame();
  finishMetricAnimations();const closed=[...document.querySelectorAll('details:not([open])')];closed.forEach(d=>d.open=true);document.body.classList.add('pdf-capture');
  const ranges=CHARTS.map(({ch})=>({ch,range:ch.timeScale().getVisibleLogicalRange()}));
  try{await drawVisibleReportCharts();resizeVisibleReportCharts();await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));assertSame();const width=Math.max(1,root.scrollWidth||document.documentElement.scrollWidth||1),height=Math.max(1,root.scrollHeight||root.getBoundingClientRect().height||1),scale=Math.min(2,window.devicePixelRatio||1.5,Math.sqrt(16000000/(width*height)));
   const canvas=await html2canvas(root,{scale,useCORS:true,backgroundColor:toRgb(getComputedStyle(document.body).backgroundColor)||'#fff',windowWidth:document.documentElement.scrollWidth,onclone:plainColors});assertSame();
   const {jsPDF}=window.jspdf,doc=new jsPDF({unit:'pt',format:'a4',compress:true}),W=doc.internal.pageSize.getWidth(),H=doc.internal.pageSize.getHeight(),m=18,iw=W-2*m,pageH=Math.max(1,Math.floor(canvas.width*(H-2*m)/iw));
   for(let y=0,n=0;y<canvas.height;y+=pageH,n++){const slice=document.createElement('canvas');slice.width=canvas.width;slice.height=Math.min(pageH,canvas.height-y);slice.getContext('2d').drawImage(canvas,0,-y);if(n)doc.addPage();doc.addImage(slice.toDataURL('image/jpeg',.9),'JPEG',m,m,iw,slice.height*iw/canvas.width);slice.width=slice.height=1;}
   doc.setProperties({title:name.replace(/\.pdf$/,''),subject:url});return new File([doc.output('blob')],name,{type:'application/pdf'});
  }finally{document.body.classList.remove('pdf-capture');closed.forEach(d=>d.open=false);resizeVisibleReportCharts();for(const {ch,range}of ranges)if(range&&CHARTS.some(item=>item.ch===ch))ch.timeScale().setVisibleLogicalRange(range);}}
 const download=file=>downloadBlob(file,file.name);
 async function copyLink(url){try{if(!navigator.clipboard?.writeText)throw Error('Clipboard API unavailable');await navigator.clipboard.writeText(url);return true;}catch{const field=document.createElement('textarea');field.value=url;field.setAttribute('aria-label','Shareable page link');field.style.position='fixed';field.style.opacity='0';document.body.append(field);field.select();try{return !!document.execCommand?.('copy');}finally{field.remove();}}}
 const shareText=()=>`${(document.querySelector('h1.st')?.textContent||'Strategy Vault').trim()} — Strategy Vault backtest`;
 async function share(kind){const url=location.href,text=shareText(),menu=document.querySelector('.share-menu');if(menu)menu.hidden=true;
  if(kind==='copy'){try{const copied=await copyLink(url);msg(copied?'Link copied. Recipients need a Vault login to open it.':'Copy failed — select the address bar to copy the link.');}catch{msg('Copy failed — select the address bar to copy the link.');}return;}
  if(kind==='pdf'){msg('Preparing PDF…');try{const file=await pagePdf();if(navigator.canShare?.({files:[file]})){await navigator.share({files:[file],title:text,text});msg('PDF shared.');}else{download(file);msg('This browser cannot attach files to other apps, so the PDF was downloaded — attach it in WhatsApp, Telegram, X or email.');}}catch(e){if(e?.name!=='AbortError')msg('Could not create the PDF: '+(e?.message||e));else msg('');}return;}
  const enc=encodeURIComponent,links={wa:`https://wa.me/?text=${enc(text+' '+url)}`,x:`https://twitter.com/intent/tweet?text=${enc(text)}&url=${enc(url)}`,tg:`https://t.me/share/url?url=${enc(url)}&text=${enc(text)}`,mail:`mailto:?subject=${enc(text)}&body=${enc(text+'\n\n'+url+'\n\nTip: use "Send as PDF file" or the download icon to attach the page as a PDF.')}`};
  window.open(links[kind],'_blank','noopener');msg('Link shared. Recipients need a Vault login to open it; send the PDF instead for anyone without one.');}
 async function action(name){if(name==='like'){toggleLike();return;}if(name==='pdf'){msg('Preparing PDF…');try{download(await pagePdf());msg('PDF downloaded.');}catch(e){if(e?.name==='ReportChangedError'){msg(e.message);return;}msg('PDF failed, opening print instead: '+(e?.message||e));finishMetricAnimations();window.print();}return;}if(name==='share-menu'){const m=document.querySelector('.share-menu');if(m){m.hidden=!m.hidden;document.querySelector('[data-action=share-menu]')?.setAttribute('aria-expanded',String(!m.hidden));if(!m.hidden)m.querySelector('button')?.focus();}return;}if(name==='save'){const key=savedKey();if(!key)return;let list=savedList();const was=list.some(x=>x.key===key);list=was?list.filter(x=>x.key!==key):[{key,id:R.id,tk:S.tk,region:S.region,name:(document.querySelector('h1.st')?.textContent||R.id).trim(),hash:location.hash,at:new Date().toISOString().slice(0,10)},...list];writeSaved(list);const b=document.querySelector('[data-action=save]');if(b){const on=!was;b.classList.toggle('on',on);b.setAttribute('aria-pressed',on);b.title=on?'Remove from Saved':'Save to your Saved list';b.innerHTML=(on?ICON.saved:ICON.save);}msg(was?'Removed from Saved.':'Saved. Open it any time from Saved in the top menu.');return;}if(name==='share'){const link=location.href;try{await navigator.clipboard.writeText(link);msg('Page link copied. Recipients need a Vault login.');}catch{const field=document.createElement('input');field.value=link;field.setAttribute('aria-label','Shareable page link');const host=document.getElementById('page-action-message');host.replaceChildren(field);field.select();try{if(document.execCommand('copy'))msg('Page link copied. Recipients need a Vault login.');}catch{}}return;}}
 const KIND=b=>b.dataset.chartSnapshot?'png':b.id==='csv'?'csv':'xls';
 function iconize(root=document){
  root.querySelectorAll('[data-chart-snapshot],[data-report-export],#csv,#trades-xlsx').forEach(b=>{if(b.classList.contains('ibtn'))return;const label=b.textContent.trim()||'Download';const k=KIND(b);b.className=`ibtn`;b.title=label;b.setAttribute('aria-label',label);b.innerHTML=ICON[k];});
  root.querySelectorAll('.box > .chart-tools, .box > .detail-tools, .report-panel > .chart-tools').forEach(t=>{const h=t.parentElement.querySelector(':scope > .box-h');if(h&&!h.querySelector('.box-tools')){t.classList.add('box-tools');h.append(t);}});
  
 }
 document.addEventListener('click',e=>{const s=e.target.closest('[data-share]');if(s){share(s.dataset.share);return;}const m=document.querySelector('.share-menu');if(m&&!m.hidden&&!e.target.closest('.share-menu,[data-action="share-menu"]'))m.hidden=true;});
 document.addEventListener('keydown',e=>{if(e.key==='Escape'){const m=document.querySelector('.share-menu');if(m)m.hidden=true;}});

 // "Download TS" (ADR): the research team's Excel tradesheet template with live formulas (ts-workbook.js). Costs are the run's own (fees + slippage per side, charged on both sides) and stay editable in C1:E1.
 async function downloadTs(){
  const s=STRAT;if(!s?.d?.trades?.length||!s?.px?.o)throw Error('No trades or prices to build the tradesheet from.');
  msg('Preparing Excel…');const {bytes,name}=await adrTsFile({d:s.d,px:s.px,v:s.v,tk:s.tk,params:s.indicatorParameters});
  downloadBlob(new Blob([bytes],{type:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'}),name);msg('Tradesheet downloaded.');
 }
 document.addEventListener('click',async e=>{const b=e.target.closest('#ts-download');if(!b)return;try{await downloadTs();}catch(err){msg('Export failed: '+err.message);}});

 // ADR results page: the AmiBroker-style report, the DD % / Bars held trade columns, "Download report" and "YOY-Monthly Excel" (report-stats.js, report-view.js)
 async function adrExcel(kind){
  const d=STRAT?.d,rep=adrReport(d);if(!rep)throw Error('No report for this page.');
  const dmy=x=>String(x).slice(0,10).split('-').reverse().join('-'),tk=STRAT.tk;
  let bytes,name;
  if(kind==='report'){
   bytes=buildLabXlsx([{name:'Statistics',layout:'blocks',blocks:statisticsBlocks(rep,benchLabel(d))},{name:'Trades',layout:'table',header:TRADE_COLUMNS,rows:tradeRows(rep,tk)}]);
   name=`${tk} report ${dmy(d.dates[0])} to ${dmy(d.dates.at(-1))}.xlsx`;
  }else{
   const [header,...rows]=yoyAoa(yoyMonthly(d.dates,d.equity));
   bytes=buildLabXlsx([{name:'YOY-Monthly',layout:'table',header,rows}]);name=`${tk} YOY-Monthly.xlsx`;
  }
  downloadBlob(new Blob([bytes],{type:'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'}),name);msg('Excel downloaded.');
 }
 document.addEventListener('click',e=>{const m=e.target.closest('[data-rep-mode]');if(!m)return;const root=document.getElementById('adr-report');if(!root)return;root.dataset.mode=m.dataset.repMode;root.querySelectorAll('[data-rep-mode]').forEach(x=>x.classList.toggle('active',x===m));});
 document.addEventListener('click',async e=>{const b=e.target.closest('#adr-report-xlsx,#adr-yoy-xlsx');if(!b)return;try{await adrExcel(b.id==='adr-report-xlsx'?'report':'yoy');}catch(err){msg('Export failed: '+err.message);}});
 async function init(){iconize();}
 document.addEventListener('click',async event=>{const b=event.target.closest('[data-action],[data-chart-snapshot],[data-report-export],#trades-xlsx,#csv');if(!b)return;try{if(b.dataset.action)return await action(b.dataset.action);if(b.dataset.chartSnapshot)return await snapshot(b.dataset.chartSnapshot);if(!STRAT)return;const st=snapshotContext();if(st.d.kind==='universe'&&(b.id==='csv'||b.id==='trades-xlsx')){const table=[UNIVERSE_TRADE_HEADERS,...universeTradeRows(st.d.trades)];if(b.id==='csv')return downloadBlob(new Blob([VaultReport.csv(csvSafeRows(table))],{type:'text/csv;charset=utf-8'}),reportName(st,'trades','csv'));let extra=[];if(/^ytath_/.test(st.v.id)){if(!STRAT.runQuery)throw Error('The original ATH run settings are unavailable. Rerun the backtest and try again.');msg('Building the daily DetailLog…');const q=new URLSearchParams(STRAT.runQuery);q.set('detail_log','1');const traced=await fetchUniverse(q.toString());if(traced?.status!=='ready'||!Array.isArray(traced.detail_log))throw Error(traced?.note||traced?.error||'The engine did not return the daily DetailLog.');extra=[{name:'DetailLog',rows:[['Date','Information'],...traced.detail_log.map(r=>[r[0]||'',r[1]])]}];}return await xlsx('trades',table,UNIVERSE_PERCENT_COLUMNS,st,[],extra);}if(b.id==='csv')return downloadBlob(new Blob([VaultReport.csv(csvSafeRows([tradeHeadersFor(st.d),...tradeRowsFor(st.d,st.tk)]))],{type:'text/csv;charset=utf-8'}),reportName(st,'trades','csv'));if(b.id==='trades-xlsx')return await xlsx('trades',[tradeHeadersFor(st.d),...tradeRowsFor(st.d,st.tk)],[8,11,13,14,15],st);const kind=b.dataset.reportExport;if(kind==='unit-audit'||kind==='unit-daily'){const unit=unitFor(st);if(!unit)throw Error('No validated unit-exposure report is available.');const table=unitExposureTables(unit)[kind==='unit-audit'?'audit':'daily'];return await xlsx(kind,[table.headers,...table.rows],table.percentColumns,st);}if(kind==='account-audit'){const account=sourceAccount(st.d,st.v.id,st.v.book,st.tk,sourceAdmission(st.v,st.tk,st.params,st.region));if(!account)throw Error('No protected source account is available.');const table=sourceAccountTables(account,st.d.trades.map(t=>t.ret_full_precision)).audit;return await xlsx('account-audit',[table.headers,...table.rows],table.percentColumns,st);}if(kind==='details')return await xlsx('detailed-statistics',[['Statistic','Net','Gross'],...VaultReport.detailRows(st.d).map(r=>r.slice(0,3))],[],st,['header',...VaultReport.detailRows(st.d).map(r=>r[3])]);if(kind==='monthly'){const m=VaultReport.monthly(st.d),mon=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];return await xlsx('monthly-returns',[['Year',...mon,'Total'],...m.years.map(y=>[y,...mon.map((_,i)=>m.months[`${y}-${String(i+1).padStart(2,'0')}`]??null),m.annual[y]]),['Avg',...m.averages,m.annualAverage]],Array.from({length:13},(_,i)=>i+1),st);}if(kind==='equity')return await xlsx('equity',[['Date',st.d.source_account?sourceAccountCopy(st.d.source_account).bandy?'Net cash-account equity (initial unit)':'Net risk-sized account equity (initial unit)':'Net equity (normalized)',st.d.source_account?'Gross same-quantity account equity (initial unit)':'Gross equity (normalized)','Selected-symbol buy & hold'],...st.d.dates.map((date,i)=>[date,st.d.equity[i],st.d.gross_equity?.[i]??null,st.d.bh[i]])],[],st);if(kind==='benchmark'){const px=st.benchmarkPx||immutable(await ohlc(st.benchmarkTk)),benchmark=st.unit?st.d.benchmark_stats:VaultReport.benchmark(px,st.d.dates[0],st.d.dates.at(-1)),keys=['total_return','cagr','max_dd','sharpe','sortino','mar','vol','exposure'];return await xlsx('benchmark',[['Statistic','Selected strategy',st.benchmarkLabel+' buy & hold'],...keys.map(k=>[k,st.d.stats[k]??null,benchmark?.[k]??null])],[],st,['header','percent','percent','percent','ratio','ratio','ratio','percent','percent']);}}catch(e){msg('Export failed: '+e.message);}});
 const refreshHeat=()=>{const el=document.getElementById('heatbox');if(el&&STRAT?.d)el.innerHTML=heatTable(STRAT.d);};
 addEventListener('beforeprint',()=>{finishMetricAnimations();refreshHeat();});
 addEventListener('afterprint',refreshHeat);
 addEventListener('afterprint',()=>document.querySelectorAll('details[data-print-closed]').forEach(d=>{d.open=false;delete d.dataset.printClosed;}));
 return {costState,actions,toolbar,savedList,writeSaved,detailHTML,benchmarkHTML,tradeHTML,equityNavigator,init,iconize,pristineExport,ICON};
})();

// Optimization research on the strategy page: parameter ranges, Rust grid run, Claude's conclusion (top 3 by the
// percentile / group / cross-comparison method, computed in Rust), YT's conclusion and an Excel workbook whose stats
// are recomputed with Excel formulas. Uses the page's globals (API, S, R, PUB, TICKERS, esc, pct, fx, …).
const OPT = { schemas: {}, schemaSync: null, grids: {}, cache: {}, cc: {}, last: null };
const OPT_LIMIT = 50000, CC_LIMIT = 10000; // manual search limit; Claude's conclusion searches up to 10,000 settings per window
const optSchema = async (api=API_IN) => OPT.schemaSync = await (OPT.schemas[api] || (OPT.schemas[api] = Promise.all(guruIds().map(g=>fetch(`${api}/guru/${g}/params`).then(r=>{if(!r.ok)throw new Error('Parameter schema unavailable');return r.json();}))).then(sets=>Object.assign({},...sets)).catch(()=>({}))));
const decodeP = s => Object.fromEntries((s || '').split(',').filter(Boolean).map(kv => kv.split(':')).map(([k, v]) => [k, +v]));
const encodeP = p => Object.entries(p).map(([k, v]) => `${k}:${v}`).join(',');
const optKey = (id, tk) => [id, tk, S.from, S.to, S.cost, S.slippage, S.rollover, S.region, S.basket].join('|');
const nVals = g => g.step > 0 && g.max >= g.min ? Math.floor((g.max - g.min) / g.step + 1e-9) + 1 : 0;
// Optimization grid laid out like the Indicators page: settings grouped by section, "fixed" until ticked, the first 10 shown, then a scroll.
let OPT_TRIED = 0; // backtests run by optimizations in this browser session
const optTried = () => `Variants tried this session: ${n0(OPT_TRIED)}`;
const optTriedAdd = n => { OPT_TRIED += n || 0; document.querySelectorAll('.otried').forEach(e => { e.textContent = optTried(); }); };
// ADR optimizer: the lab groups the ranges table as Entry / Filters / Exits
function adrSections(secs) {
  if (!/^ytadr_/.test(STRAT?.v?.id || '')) return secs;
  const g = {E: [], F: [], X: [], S: []}; secs.forEach(s => s.params.forEach(p => g[adrGroup(p.key)].push(p)));
  return [['E', 'Entry'], ['F', 'Filters'], ['X', 'Exits'], ['S', 'Other']].filter(([k]) => g[k].length).map(([k, title]) => ({title, params: g[k]}));
}
// ATH grid: Entry, Filters, Exit, Portfolio, Engine options (headings without settings are left out)
function athSections(ps) {
  const g = {E: [], F: [], X: [], S: [], O: []}; ps.forEach(p => g[athEngineOpt(p.label) ? 'O' : athGroup(p.key, p.label)].push(p));
  return [['E', 'Entry'], ['F', 'Filters'], ['X', 'Exit'], ['S', 'Portfolio'], ['O', 'Engine options']].filter(([k]) => g[k].length).map(([k, title]) => ({title, params: g[k]}));
}
function optGrid(ps, row, id = '') {
  const secs = /^ytath_/.test(STRAT?.v?.id || '') ? athSections(ps) : adrSections(paramSections(ps)), h = optGridHeight(secs);
  return `<div class="ogrid-wrap"${h ? ` style="max-height:${h}px"` : ''}><table class="ogrid"${id ? ` id="${id}"` : ''}><thead><tr><th></th><th>Setting</th><th>From</th><th>To</th><th>Step</th><th class="n">Values</th></tr></thead><tbody>${secs.map(s => `<tr class="ogrp"><td colspan="6">${esc(s.title)}</td></tr>${s.params.map(row).join('')}`).join('')}</tbody></table></div>`;
}
/** After drawing: cut each grid's scroll box at the top of its 11th setting (long labels wrap, so the estimate above is only a start). */
function optFit() {
  document.querySelectorAll('.ogrid-wrap').forEach(w => { const r = w.querySelectorAll('tr[data-k]'); if (r.length <= OPT_ROWS_SHOWN) return;
    let cut = r[OPT_ROWS_SHOWN]; if (cut.previousElementSibling?.classList.contains('ogrp')) cut = cut.previousElementSibling; w.style.maxHeight = `${cut.offsetTop + 2}px`; });
}
window.addEventListener('resize', () => optFit());
const pLabel = (id, k) => (OPT.schemaSync?.[id] || []).find(x => x.key === k)?.label || k;
// compact column names for tables
const SHORT = { trend: 'Trend SMA', rsi_n: 'RSI period', exit_rsi: 'Exit RSI', rsi_lt: 'Entry RSI <', rsi_gt: 'Entry RSI >', hold: 'Hold bars', ma: 'Short SMA', low_n: 'Lookback', limit: 'Limit %',
  fall: 'Falling days', first_lt: 'First RSI <', add_fall: 'Add after', down: 'Down days', add_pct: 'Add %', stretch: 'Stretch %', limit_min: 'Min limit %', profit: 'Profit %', hh_n: 'High lookback',
  atr_k: 'ATR ×', band: 'Band %', stop: 'Stop %', hv: 'HV >', adx: 'ADX >', lf: 'Liquidity >', days: 'Days', up: 'Up days', rise: 'Rise %', dom: 'From day', bars: 'Bars', slow_n: '2nd RSI period',
  slow_lt: '2nd RSI <', look: 'Lookback', sum_n: 'Summed RSI', sum_lt: 'Sum <', loc: 'Close in low %', pctb_n: '%b period', pctb_lt: '%b <', di_n: '+DI period', di_lt: '+DI <', vix_rsi: 'VIX RSI >' };
const sLabel = k => SHORT[k] || k;
const n0 = x => (x ?? 0).toLocaleString('en-IN');
const bookDefaults = id => Object.fromEntries((OPT.schemaSync?.[id] || []).map(x => [x.key, x.def]));
const sameP = (a, b) => Object.keys(b).every(k => Math.abs((a[k] ?? b[k]) - b[k]) < 1e-9);

// ---------------------------------------------------------------- book periods (All / Before book / After book)
const tkSpan = tk => TICKERS.find(t => t.ticker === tk) || TICKERS[0];
const BOOKWIN = () => ({ all: { from: '', to: '' }, before: { from: '', to: `${PUB - 1}-12-31` }, after: { from: `${PUB}-01-01`, to: '' } });
const periodKind = (from = S.from, to = S.to) => !from && !to ? 'all' : !from && to === `${PUB - 1}-12-31` ? 'before' : from === `${PUB}-01-01` && !to ? 'after' : 'custom';
const PERIOD_NAME = { all: 'All', before: 'Before book', after: 'After book', custom: 'Custom dates' };

// ---------------------------------------------------------------- the stats the conclusion shows (user's list) and ranks on (five)
const CC_STATS = [
  ['cagr', 'CAGR', v => pct(v, 2, true), 1], ['net_profit', 'Net profit', v => pct(v, 2, true), 1], ['max_dd', 'Max DD', v => pct(v, 2), 0], ['win_rate', 'Win %', v => pct(v, 2), 0],
  ['sharpe', 'Sharpe', v => fx(v), 1], ['monthly_thickness', 'Monthly thickness', v => pct(v, 2, true), 1], ['trades', 'Trades', v => fx(v, 0), 0],
  ['expectancy_ratio', 'Expectancy ratio', v => fx(v), 1], ['car_mdd', 'CAR/MDD', v => fx(v), 1]];
const RANK_KEYS = ['cagr', 'sharpe', 'monthly_thickness', 'expectancy_ratio', 'car_mdd'];
const RANK_LABEL = { cagr: 'CAGR', sharpe: 'Sharpe', monthly_thickness: 'Monthly thickness', expectancy_ratio: 'Expectancy ratio', car_mdd: 'CAR/MDD' };
/** A result row of /optimize as {p,m}; preserve the engine's closed-return sum and alias only CAR/MDD. */
function rowOf(res, r) {
  const nd = res.dims.length, p = { ...bookDefaults(res.id), ...(res.fixed || {}) }, m = {};
  res.dims.forEach((d, j) => p[d.key] = r[j]);
  res.cols.forEach((c, i) => { if (i >= nd) m[c] = r[i]; });
  m.car_mdd = m.mar; return { p, m };
}
const bookRow = res => { const m = {}; res.cols.slice(res.dims.length).forEach((c, i) => m[c] = res.book[i]); m.car_mdd = m.mar; return { p: bookDefaults(res.id), m }; };
const statsM = s => ({ ...s, car_mdd: s.mar });
/** Row i of an /optimize result: full results carry `rows`, slim ones only the ranked candidates in `picked`. */
const rowAt = (res, i) => res.rows.length ? res.rows[i] : res.picked[i];

// Banner when the page runs custom (non-default) settings.
function optimBanner(v) {
  const p = decodeP(S.p); if (!Object.keys(p).length) return '';
  const book = bookDefaults(v.id);
  return `<div class="pbanner"><span class="olabel">Custom settings</span><span>${Object.entries(p).map(([k, x]) => `<span class="oset">${esc(sLabel(k))} <b>${x}</b>${book[k] !== x ? ` <span class="muted">default ${book[k] ?? '–'}</span>` : ''}</span>`).join(' ')}</span><a href="${href(R.path, { ...S, p: '' })}">Back to default settings</a></div>`;
}

// ---------------------------------------------------------------- loader: a book flipping its pages
const bookLoader = (t = 'Loading…') => `<div class="bload" role="status" aria-live="polite"><div class="bflip" aria-hidden="true"><i class="bcv l"></i><i class="bcv r"></i><i class="bpg"></i><i class="bpg"></i><i class="bpg"></i></div><p>${esc(t)}</p></div>`;
const minWait = (t0, ms) => new Promise(ok => setTimeout(ok, Math.max(0, ms - (performance.now() - t0))));

// YT strategies open the manual grid unticked on the default settings (From = To = default), so a run tests exactly the values entered.
const ytGrid = ps => Object.fromEntries(ps.map(p => [p.key, { on: false, min: p.def, max: p.def, step: p.step }]));
async function optimSection(v, tk, d, detailed = true) {
  OPT.schemaSync = await optSchema();
  const ps = OPT.schemaSync[v.id] || [];
  const opt = detailed && ps.length ? (() => { const g = OPT.grids[v.id] || (OPT.grids[v.id] = isYt(v) ? ytGrid(ps) : OptimCore.defaultGrid(ps, OPT_LIMIT));
    return `<section class="box${/^ytadr_/.test(v.id) ? ' adr-opt' : ''}" id="optim"><div class="box-h">Optimization <small>Tick the settings to vary. Unticked settings stay at the default value. Runs on the period selected above.</small></div>
    ${optGrid(ps, p => { const x = g[p.key]; return `<tr data-k="${p.key}" class="${x.on ? '' : 'offrow'}" title="Default ${esc(p.def)}"><td><input type="checkbox" data-f="on" ${x.on ? 'checked' : ''} aria-label="Vary ${esc(p.label)}"></td><td>${esc(p.label)}</td>
      ${['min', 'max', 'step'].map(f => `<td><input type="number" data-f="${f}" value="${x[f]}" step="any"></td>`).join('')}<td class="n muted" data-cnt>${x.on ? nVals(x) : 'fixed'}</td></tr>`; })}
    <form class="obar" id="oform"><span class="ocount" id="ocount"></span><button class="dbtn primary sm" type="submit" id="orun">Run optimization</button><span class="grow"></span><span class="otried muted">${optTried()}</span></form>
    <p class="ostat" id="otime"></p><div id="opt-out"></div></section>`; })() : '';
  const claude = detailed ? `<section class="box" id="claude-c"><div class="box-h">Claude’s conclusion <small>Top 3 chosen once on all the data (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD), shown for ${esc(PERIOD_NAME[periodKind()])} and split before / after the book</small></div><p class="rnote">This summary is calculated locally from Rust backtests. Opening a strategy does not call an AI model or use AI tokens. Parameter selection is research, not proof of future performance or reproduction of the book.</p><div id="cc-body">${ps.length ? bookLoader('Rust is testing the parameter grid…') : '<p class="muted">This rule has no settings to vary, so there is nothing to rank.</p>'}</div></section>` : `<section class="box" id="claude-c"><div class="box-h">Claude’s conclusion <small>Parameter research is available on the optimization page</small></div><p>Open Optimization to run the existing full-history parameter ranking, read its comparisons and download its workbook.</p><a class="dbtn sm" href="${href('/o/'+encodeURIComponent(v.id),{...S})}">Open parameter research</a></section>`;
  return `${opt}${claude}${gptSection(v,tk,d)}
    <section class="box" id="yt-c"><div class="box-h">YT’s conclusion <small>Our own call on the settings, saved in the vault</small></div>
      <textarea id="yt-text" class="yt" rows="6" placeholder="Which settings we trade, on which index, why they differ from the default settings, and what would make us stop."></textarea>
      <div class="ytbar"><button class="dbtn sm" id="yt-save">Save</button><span class="muted" id="yt-msg"></span></div></section>`;
}

function optCount() {
  const el = $('#optim'); if (!el) return 0;
  let n = 1, on = 0; el.querySelectorAll('tr[data-k]').forEach(tr => { const x = OPT.grids[R.id][tr.dataset.k], k = nVals(x); tr.querySelector('[data-cnt]').textContent = x.on ? k || '–' : 'fixed'; tr.classList.toggle('offrow', !x.on); if (x.on) { n *= k; on++; } });
  $('#ocount').innerHTML = on ? `<b class="${n > OPT_LIMIT || !n ? 'neg' : ''}">${n0(n)}</b> combinations${n > OPT_LIMIT ? `, limit ${n0(OPT_LIMIT)}` : ''}` : 'Tick at least one setting';
  $('#orun').disabled = !on || !n || n > OPT_LIMIT || (window.VAULT_APP && window.VAULT_USER?.role === 'viewer'); return n;
}

function optimInit() {
  if (!STRAT) return;
  const { v, tk } = STRAT, el = $('#optim');
  if (el) {
    const g = OPT.grids[v.id];
    el.addEventListener('input', e => { const tr = e.target.closest('tr[data-k]'); if (!tr) return; const f = e.target.dataset.f; g[tr.dataset.k][f] = f === 'on' ? e.target.checked : +e.target.value; optCount(); });
    $('#oform').addEventListener('submit', e => { e.preventDefault(); runOpt(); });
    optCount(); optFit();
    const c = OPT.cache[optKey(v.id, tk)]; if (c) showOpt(c.res);
    if (OPT.runAfterPeriod) { OPT.runAfterPeriod = false; if (!c && !$('#orun').disabled) runOpt(); }
  }
  if ($('#cc-body') && (OPT.schemaSync[v.id] || []).length && (!window.VAULT_APP || window.VAULT_USER?.role !== 'viewer')) conclusionRun(v, tk);
  const key = `${v.id}.${S.region}`, noteText = $('#yt-text'), noteMessage = $('#yt-msg');
  fetch(`${API_IN}/notes?key=${key}`).then(r => r.ok ? r.text() : '').then(t => { if (noteText && $('#yt-text') === noteText) noteText.value = t; }).catch(() => {});
  if (window.VAULT_APP && window.VAULT_USER?.role === 'viewer') { if ($('#gpt-body')) $('#gpt-body').innerHTML = '<p class="muted">Parameter research requires researcher access.</p>'; if ($('#cc-body')) $('#cc-body').innerHTML = '<p class="muted">Automatic parameter search requires researcher access. You can read saved conclusions below and run the default settings above.</p>'; if ($('#yt-save')) { $('#yt-save').disabled = true; $('#yt-save').title = 'Researcher access is required to save conclusions'; } if ($('#yt-text')) $('#yt-text').readOnly = true; if ($('#orun')) { $('#orun').disabled = true; $('#orun').title = 'Researcher access is required to optimize'; } return; }
  if ($('#yt-save')) $('#yt-save').onclick = async () => { const m = noteMessage; m.textContent = 'Saving…';
    try { const r = await fetch(`${API_IN}/notes?key=${key}`, { method: 'POST', headers: { 'Content-Type': 'text/plain;charset=utf-8' }, body: noteText.value }); m.textContent = r.ok ? `Saved at ${new Date().toLocaleTimeString()}` : 'Save failed: ' + await r.text(); } catch (err) { m.textContent = 'Save failed: ' + err.message; } };
}

const optQuery = (v, tk, w, g, slim) => { const q = new URLSearchParams({ id: v.id, ticker: tk, from: w.from || '', to: w.to || '', cost: S.cost || '0',slippage:S.slippage||'0',rollover:S.rollover||'0', ...(slim ? { slim: '1' } : {}), ...(!slim && /^ytadr_/.test(v.id) ? { sheet: '1' } : {}) });
  Object.entries(g).forEach(([k, x]) => { if (x.on) q.set('g.' + k, `${x.min}:${x.max}:${x.step}`); }); return q; };
async function fetchOpt(q, api=API) { const r = await fetch(enginePath(q.get('id'),'optimize',api)+'?'+q), b = await r.text(); if (!r.ok) { let message = b; try { message = JSON.parse(b).error || b; } catch {} throw new Error(message); } return JSON.parse(b); }

async function runOpt() {
  const pending = ytPeriod(); if (Object.keys(pending).length) { OPT.runAfterPeriod = true; return go(R.path, { ...S, ...pending }); }
  const { v, tk } = STRAT, g = OPT.grids[v.id], key=optKey(v.id,tk), out=$('#opt-out');
  const n = optCount(); $('#orun').disabled = true; $('#otime').textContent = `Running ${n0(n)} backtests…`;
  $('#opt-out').innerHTML = bookLoader(`Running ${n0(n)} backtests in Rust…`);
  const t0 = performance.now();
  try {
    const res = await fetchOpt(optQuery(v, tk, { from: S.from, to: S.to }, g, false)); await minWait(t0, 900); res.rt = performance.now() - t0; optTriedAdd(res.combos);
    OPT.cache[key] = { res }; if (STRAT && optKey(STRAT.v.id,STRAT.tk)===key && $('#opt-out')===out) showOpt(res);
  } catch (e) { if ($('#opt-out')===out) {out.innerHTML = ''; $('#otime').innerHTML = `<span class="neg">${esc(e.message)}</span>`;} }
  finally { if ($('#orun')) optCount(); }
}

const applyLink = p => `<button class="olink" data-apply="${esc(encodeP(p))}">Apply</button>`;
const setLine = (p, def) => Object.keys(def).map(k => `<span class="oset">${esc(sLabel(k))} <b class="${Math.abs((p[k] ?? def[k]) - def[k]) > 1e-9 ? 'chg' : ''}">${p[k] ?? def[k]}</b></span>`).join(' ');
const statCells = m => CC_STATS.map(([k, , f, signed]) => `<td class="n ${k === 'max_dd' ? 'neg' : signed ? cls(m[k]) : ''}">${f(m[k])}</td>`).join('');
const statHead = () => CC_STATS.map(([, t]) => `<th class="n">${t}</th>`).join('');

// ---- ADR: the research team's Excel tradesheet ("Download TS") for any run, used by the strategy page and by the optimizer's Tradesheet buttons
function saveBytes(bytes, name, type = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet') {
  const url = URL.createObjectURL(new Blob([bytes], {type})), a = document.createElement('a'); a.href = url; a.download = name; document.body.append(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(url), 4000);
}
async function adrTsFile({d, px, v, tk, params}) {
  const fmt = x => String(x).slice(0, 10), dmy = x => fmt(x).split('-').reverse().join('-');
  const num = k => Number(params?.[k]), code = {'NIFTY 50': 'NF', NIFTYBANK: 'BN', 'NIFTY BANK': 'BN'}[tk] || String(tk).replace(/\s/g, '').slice(0, 6);
  const exitShort = num('exit_n') > 0 ? `Ncandle-${num('exit_n')}` : num('exit_type') === 1 ? `ADR${num('exit_ad')}` : 'Exit';
  const tab = `${code}_En_ADR${Number.isFinite(num('thr')) ? ' thr ' + num('thr') : ''}_EX_${exitShort}`.slice(0, 31);
  const lines = c => ruleLines(v, params || {}).filter(r => r[0] === c).map(r => r[1]);
  const dates = d.dates || [], cost = d.costs || {}, side = (Number(cost.fees_bps) || 0) + (Number(cost.slippage_bps) || 0);
  await loadXlsx(); if (!window.XLSX?.utils) throw Error('The Excel library did not load.');
  const book = buildTsWorkbook(XLSX, {ticker: tk, tabName: tab,
    trades: d.trades.filter(t => Number.isFinite(t.avg_px) && Number.isFinite(t.exit_px)).map(t => ({dir: t.dir === -1 ? 'Short' : 'Long', entry: fmt(t.entry), entryPx: t.avg_px, exit: fmt(t.exit), exitPx: t.exit_px})),
    bars: {d: px.d.map(fmt), o: px.o, h: px.h, l: px.l, c: px.c},
    rules: {timeframe: 'Daily', index: tk, period: `${dmy(dates[0])} to ${dmy(dates.at(-1))}`, position: 'Long', filter: lines('F').join('; ') || 'Without filter', setup: '—', entry: lines('E').join('; '), exit: lines('X').join('; '), aflPath: ''},
    costs: {impact: 2 * side / 10000, rollLong: 0, rollShort: 0}});
  return {bytes: await tsBytes(XLSX, book), name: `TS_${tab}.xlsx`};
}

// ---- ADR optimizer, laid out like the lab (optimize-view.js): Now / #1-#3, 3D surface, downloads, sortable result list. The older block stays in the page (hidden) because its handlers still drive the conclusion workbook and the GPT review.
const ADR_OPT = {};
const adrLabel = (id, k) => OPT.schemaSync?.[id]?.find(p => p.key === k)?.label || sLabel(k);
const adrGroup = k => /^(st\d?_|filter|flt_|athc_|bear|bull)/.test(k) ? 'F' : /^(exit_|pb$|hsl$|buffer$)/.test(k) ? 'X' : /^(thr|n_entry|lookback|consec|entry_|avg_|prev_|rsi_|adx_|roc_|adr_on|patch|candle|deadstock|penny|turn)/.test(k) ? 'E' : 'S';
function adrOptHTML(res) {
  const v = STRAT.v, labels = res.dims.map(d => adrLabel(v.id, d.key)), position = (res.fixed?.entry_type ?? bookDefaults(v.id)?.entry_type ?? 0) >= 0.5 ? 'Short' : 'Long';
  const lab = labRows(res, position, bookDefaults(v.id)), top = (res.rank?.top3 || []).map(c => c.row);
  const sort = {col: labels.length + 2, dir: -1};      // the lab lists the combinations on CAR/MDD, highest first
  Object.assign(ADR_OPT, {res, labels, lab, position, top, metric: 'CAR', sort});
  return `<div class="lab-results">${resultsHTML(res, labels, lab, top, 'CAR', sort)}</div>`;
}
const adrRunQuery = (res, k) => { const p = rowOf(res, res.rows[k]).p, S_ = S; return new URLSearchParams({id: res.id, ticker: STRAT.tk, from: res.from, to: res.to, cost: S_.cost || '0', slippage: S_.slippage || '0', rollover: S_.rollover || '0', ...Object.fromEntries(Object.entries(p).map(([a, b]) => ['p.' + a, b]))}); };
async function adrRunOf(res, k) { const r = await fetch(enginePath(res.id, 'run') + '?' + adrRunQuery(res, k)); if (!r.ok) throw new Error((await r.text()).slice(0, 160)); return { d: await r.json(), p: rowOf(res, res.rows[k]).p }; }
const adrComboName = (A, k) => A.labels.map((l, i) => `${l} ${A.lab.rows[k].settings[i]}`).join(' · ');
const fileSafe = s => s.replace(/[\\/:*?"<>|]/g, ' ').replace(/\s+/g, ' ').trim();
function adrOptSheet() {
  const {res, labels, lab, position} = ADR_OPT, v = STRAT.v, g = OPT.grids[v.id] || {}, tk = STRAT.tk, dmy = x => String(x).slice(0, 10).split('-').reverse().join('-');
  const lines = {E: [], F: [], X: [], S: []};
  res.dims.forEach((d, i) => lines[adrGroup(d.key)].push(rangeLine(labels[i], g[d.key]?.min ?? d.values[0], g[d.key]?.max ?? d.values.at(-1), g[d.key]?.step ?? (d.values[1] - d.values[0]))));
  const bps = (Number(S.cost || 0) + Number(S.slippage || 0));
  const bytes = buildOptimizationXlsx({labels, rows: lab.rows.map(r => ({no: r.no, am: r.am, settings: r.settings})), rules: {timeframe: 'Daily', index: tk, period: `${dmy(res.from)} to ${dmy(res.to)}`, brokerage: bps > 0 ? `${bps} bps per side` : 0, openPosition: 1,
    aflPath: 'Strategy Vault · YT Strategies', strategy: v.name || v.id, position, filter: lines.F.join('; '), setup: lines.S.join('; '), entry: lines.E, exit: lines.X}});
  saveBytes(bytes, optimsFileName(LABEL[tk] || tk, position, ''));
}
/** daily bars for a tradesheet: the optimizer page loads none (the strategy page does), so fetch the full history from the engine */
async function adrPx() {
  const px = STRAT.px || await fullOhlc(STRAT.tk).catch(() => null) || await ohlc(STRAT.tk);
  if (!px?.d) throw new Error('Price history unavailable for ' + STRAT.tk);
  return px;
}
async function adrTsRow(k) {
  const A = ADR_OPT, {d, p} = await adrRunOf(A.res, k), px = await adrPx();
  const {bytes, name} = await adrTsFile({d, px, v: STRAT.v, tk: STRAT.tk, params: p}); saveBytes(bytes, name);
}
async function adrTsAll(btn) {
  const A = ADR_OPT, n = A.lab.rows.length, one = $('#adr-ts-mode')?.value === 'one', px = await adrPx(), tk = STRAT.tk, label0 = btn.textContent;
  const files = [], sheets = [], sum = [['No.', 'Combination', 'Net Profit %', 'CAR', 'MaxSysDD', 'CAR/MDD', '# trades', '# winners', '# losers', 'Avg. profit/loss %', 'Sharpe', 'Buy & hold CAGR']];
  btn.disabled = true;
  try {
    for (let k = 0; k < n; k++) {
      btn.textContent = `Running ${k + 1} of ${n}…`;
      const {d} = await adrRunOf(A.res, k), rep = buildReport({dates: d.dates, close: d.close, equity: d.equity, bh: d.bh, trades: d.trades}, px, {side: ((Number(d.costs?.fees_bps) || 0) + (Number(d.costs?.slippage_bps) || 0)) / 10000}), s = rep.system, a = rep.all, nn = String(k + 1).padStart(2, '0'), name = adrComboName(A, k);
      sum.push([k + 1, name, 100 * s.net_profit, 100 * s.cagr, 100 * s.max_sys_dd, s.car_mdd, a.trades, a.winners.count, a.losers.count, 100 * a.avg, s.sharpe_trades, rep.buy_hold ? 100 * rep.buy_hold.cagr : null]);
      const sheet = {name: one ? `${nn} ${name}`.slice(0, 31) : 'Tradesheet', layout: 'table', header: TRADE_COLUMNS, rows: tradeRows(rep, tk)};
      if (one) sheets.push(sheet);
      else files.push([`${nn} ${fileSafe(name)}`.slice(0, 90) + '.xlsx', buildLabXlsx([sheet])]);
    }
    btn.textContent = 'Building the workbook…';
    const summary = {name: 'Summary', layout: 'table', header: sum[0], rows: sum.slice(1)};
    if (one) saveBytes(buildLabXlsx([summary, ...sheets]), `tradesheets ${tk}.xlsx`);
    else { files.unshift(['00 Summary.xlsx', buildLabXlsx([summary])]); saveBytes(zipStored(files), `tradesheets ${tk}.zip`, 'application/zip'); }
  } finally { btn.disabled = false; btn.textContent = label0; }
}
function adrOptWire(out, res) {
  if (out.__adr) out.removeEventListener('click', out.__adr);
  out.__adr = async e => {
    const A = ADR_OPT, t = e.target;
    const stats = t.closest('[data-stats]'); if (stats) return go(R.path, { ...S, ...ytPeriod(), p: encodeP(rowOf(res, res.rows[+stats.dataset.stats]).p) });
    const ts = t.closest('[data-tsrow]'); if (ts) { const b = ts, l = b.textContent; b.textContent = '…'; try { await adrTsRow(+ts.dataset.tsrow); } catch (err) { console.error('Tradesheet download failed', err); b.title = 'Failed: ' + (err?.message || err); b.textContent = 'Failed'; return; } b.textContent = l; return; }
    const th = t.closest('th[data-sort]'); if (th) { const c = +th.dataset.sort; A.sort = A.sort && A.sort.col === c ? {...A.sort, dir: -A.sort.dir} : {col: c, dir: c < A.labels.length ? 1 : -1}; $('#adr-list').innerHTML = listHTML(A.labels, A.lab, A.sort); return; }
    if (t.closest('[data-showall]')) { A.sort = {...(A.sort || {col: -1, dir: 1}), all: true}; $('#adr-list').innerHTML = listHTML(A.labels, A.lab, A.sort); return; }
    if (t.closest('#adr-opt-sheet')) return adrOptSheet();
    const all = t.closest('#adr-ts-all'); if (all) return adrTsAll(all).catch(err => { all.textContent = 'Failed: ' + err.message; });
    if (t.closest('#adr-concl')) return $('#oxlsx')?.click();
    if (t.closest('#adr-gpt')) return $('#gpt-grid-review')?.click();
  };
  out.addEventListener('click', out.__adr);
}

// Manual optimization result: the default settings and the top 3 by Claude's ranking, the parameter map, the top 25 by score.
function showOpt(res) {
  const { v } = STRAT, out = $('#opt-out'); if (!out) return;
  const def = bookDefaults(v.id), rk = res.rank, rows = res.rows.map(r => rowOf(res, r)), top = (rk.top3 || []).map(c => ({ ...rows[c.row], c }));
  $('#otime').textContent = `${n0(res.combos)} settings on ${res.from} to ${res.to} in ${res.ms} ms (Rust, ${res.threads} threads). ${n0(rk.eligible)} had ${rk.min_trades}+ trades (${fx(rk.per_year, 1)} a year) and were ranked.`;
  const tbl = `<table class="tbl ocmp cc-tbl"><thead><tr><th></th><th>Settings</th>${statHead()}<th class="n">Score</th><th></th></tr></thead><tbody>
    <tr class="book"><td>Default</td><td class="cset">${setLine({}, def)}</td>${statCells(bookRow(res).m)}<td class="n">–</td><td></td></tr>
    ${top.map((t, k) => `<tr class="${k ? '' : 'pick'}"><td>#${k + 1}</td><td class="cset">${setLine(t.p, def)}</td>${statCells(t.m)}<td class="n">${fx(t.c.score, 1)}</td><td class="n">${sameP(t.p, def) ? '<span class="muted">default</span>' : applyLink(t.p)}</td></tr>`).join('')}</tbody></table>`;
  const dims = res.dims, big = [...dims].sort((a, b) => b.values.length - a.values.length);
  OPT.hx ??= {}; const hx = OPT.hx[v.id] && dims.find(d => d.key === OPT.hx[v.id][0]) ? OPT.hx[v.id] : [big[0]?.key, big[1]?.key || '', 'sharpe'];
  const sel = (id, cur, opts) => `<select id="${id}">${opts.map(([k, t]) => `<option value="${k}" ${k === cur ? 'selected' : ''}>${esc(t)}</option>`).join('')}</select>`;
  const dOpts = dims.map(d => [d.key, sLabel(d.key)]);
  const map = dims.length ? `<div class="ohead"><h4>Parameter map</h4><span class="osel">${sel('hx-x', hx[0], dOpts)} across, ${sel('hx-y', hx[1], [['', 'nothing'], ...dOpts])} down, showing ${sel('hx-m', hx[2], [['sharpe', 'Sharpe'], ['cagr', 'CAGR'], ['max_dd', 'Max DD'], ['mar', 'CAR/MDD'], ['expectancy_ratio', 'Expectancy ratio'], ['monthly_thickness', 'Monthly thickness'], ['trades', 'Trades'], ['win_rate', 'Win %']])}</span></div><div id="hx"></div>` : '';
  const legacy = `<div class="ohead"><h4>Default settings and Claude’s top 3 on this run</h4><span><button class="dbtn primary sm" id="grid-xlsx">Download all ${n0(res.combos)} rows</button> <button class="dbtn sm" id="oxlsx">Conclusion workbook</button> <button class="dbtn sm" id="gpt-grid-review">GPT analysis of this grid</button></span></div>${tbl}
    <p class="cap">Score = average percentile on the five ranking stats (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) among the ${n0(rk.eligible)} settings with ${rk.min_trades}+ trades (${fx(rk.per_year, 1)} a year). Highlighted values differ from the default settings.</p>${map}`;
  const adr = /^ytadr_/.test(v.id) && res.sheet?.length;
  out.innerHTML = adr ? adrOptHTML(res) + `<div id="opt-legacy" hidden>${legacy}</div>` : legacy;
  if (adr) adrOptWire(out, res);
  const an = { res, rows, pick: top[0], def };
  const drawHx = () => { OPT.hx[v.id] = [$('#hx-x').value, $('#hx-y').value, $('#hx-m').value]; $('#hx').innerHTML = heatGrid(an, ...OPT.hx[v.id]); };
  if (dims.length) { drawHx(); ['#hx-x', '#hx-y', '#hx-m'].forEach(s => $(s).onchange = drawHx); }
  out.onclick = e => { const b_ = e.target.closest('[data-apply]'); if (b_) go(R.path, { ...S, ...ytPeriod(), p: b_.dataset.apply }); };
  if ($('#grid-xlsx')) $('#grid-xlsx').onclick=async()=>{const b=$('#grid-xlsx');b.disabled=true;try{await exportFullGrid(res,v,STRAT.tk);}catch(err){b.textContent='Export failed: '+err.message;}finally{b.disabled=false;}};
  if ($('#oxlsx')) $('#oxlsx').onclick = async () => { const b = $('#oxlsx'); b.disabled = true; b.textContent = 'Building Excel…';
    const tk=STRAT.tk,win={from:S.from,to:S.to},context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first,selectionLabel:'Manual grid window'};
    try { await exportXlsx(await conclusionData(v, tk, res, win, context)); b.textContent = 'Conclusion workbook'; } catch (err) { b.textContent = 'Excel failed: ' + err.message; } finally { b.disabled = false; } };
  if ($('#gpt-grid-review')) $('#gpt-grid-review').onclick=async()=>{
    const b=$('#gpt-grid-review'),body=$('#gpt-body'),tk=STRAT.tk,key=optKey(v.id,tk),win={from:S.from,to:S.to},context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first,selectionLabel:'Manual grid window'};
    OPT.gptGeneration=(OPT.gptGeneration||0)+1;b.disabled=true;b.textContent='Comparing the recorded grid…';
    try{const D=await conclusionData(v,tk,res,win,context);if($('#gpt-body')===body&&optKey(v.id,tk)===key){OPT.last=D;gptConclusionWire(D);body.scrollIntoView({block:'start',behavior:'smooth'});}b.textContent='GPT analysis of this grid';}catch(e){b.textContent='Analysis failed: '+e.message;}finally{b.disabled=false;}
  };
  anim(out, { opacity: [0, 1], transform: ['translateY(6px)', 'translateY(0)'] }, { duration: .3 });
}

// 2-D slice of the grid: other parameters held at the #1 setting (or the book).
function heatGrid(an, xk, yk, mk) {
  const res = an.res, dims = res.dims, at = an.pick ? an.pick.p : an.def, xd = dims.find(d => d.key === xk), yd = dims.find(d => d.key === yk && d.key !== xk);
  const look = new Map(an.rows.map(r => [dims.map(d => r.p[d.key]).join('|'), r]));
  const get = (xv, yv) => look.get(dims.map(d => d.key === xk ? xv : yd && d.key === yd.key ? yv : at[d.key]).join('|'));
  const ys = yd ? yd.values : [null], vals = ys.flatMap(y => xd.values.map(x => get(x, y)?.m[mk])).filter(isFinite), cap = Math.max(1e-9, ...vals.map(Math.abs));
  const f = mk === 'trades' ? x => fx(x, 0) : /cagr|max_dd|win_rate|thickness/.test(mk) ? x => (x * 100).toFixed(1) : x => fx(x);
  const bg = x => x == null || !isFinite(x) ? 'transparent' : `color-mix(in srgb, ${mk !== 'max_dd' && x >= 0 ? 'var(--chart)' : 'var(--red)'} ${Math.round(Math.min(1, Math.abs(x) / cap) * 42 + 4)}%, transparent)`;
  const fixed = dims.filter(d => d.key !== xk && d.key !== yd?.key).map(d => `${esc(sLabel(d.key))} ${at[d.key]}`).join(', ');
  return `<div class="hxwrap"><table class="oheat"><thead><tr><th class="y">${yd ? esc(sLabel(yd.key)) : ''}</th>${xd.values.map(x => `<th>${x}</th>`).join('')}</tr></thead><tbody>
    ${ys.map(y => `<tr><th class="y">${y ?? ''}</th>${xd.values.map(x => { const r = get(x, y), val = r?.m[mk]; return `<td style="background:${bg(val)}" class="${r && sameP(r.p, an.def) ? 'hbook' : ''} ${r && an.pick && r.p === an.pick.p ? 'hpick' : ''}" title="${r ? esc(dims.map(d => `${sLabel(d.key)} ${r.p[d.key]}`).join(', ')) + `, ${r.m.trades} trades` : ''}"${r ? ` data-apply="${esc(encodeP(r.p))}"` : ''}>${val == null ? '–' : f(val)}</td>`; }).join('')}</tr>`).join('')}</tbody></table></div>
    <p class="cap"><span class="key kpick"></span>#1 <span class="key kbook"></span>default setting. Click a cell to apply it.${fixed ? ` Other settings held at ${fixed}.` : ''}</p>`;
}

// ---------------------------------------------------------------- Claude's conclusion
// One search on ALL the data picks the top 3. The same settings (and the defaults) are then shown on the period selected
// above, and split before / after the book's publication: the settings never change with the period.
async function conclusionRun(v, tk) {
  const el = $('#cc-body'); if (!el) return;
  const key = [API, v.id, tk, S.cost, S.slippage, S.rollover].join('|'), viewKey=optKey(v.id,tk), context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first}, win={from:S.from,to:S.to}, t0 = performance.now(), gptGeneration=OPT.gptGeneration||0;
  el.innerHTML = bookLoader('Claude is testing every setting on all the data…');
  try {
    const res = await (OPT.cc[key] || (OPT.cc[key] = fetchOpt(optQuery(v, tk, { from: '', to: '' }, OptimCore.defaultGrid(OPT.schemaSync[v.id], CC_LIMIT), true),context.api)));
    const data = await conclusionData(v, tk, res, win, context);
    await minWait(t0, 1200);
    if (STRAT?.v.id !== v.id || STRAT.tk !== tk || optKey(v.id,tk)!==viewKey || $('#cc-body')!==el) return;
    $('#cc-body').innerHTML = conclusionHtml(data); conclusionWire(data); if(gptGeneration===(OPT.gptGeneration||0)){OPT.last = data;gptConclusionWire(data);}
    anim($('#cc-body'), { opacity: [0, 1], transform: ['translateY(6px)', 'translateY(0)'] }, { duration: .35 });
  } catch (e) { delete OPT.cc[key]; if (STRAT?.v.id === v.id && STRAT.tk === tk && optKey(v.id,tk)===viewKey && $('#cc-body')===el) {if($('#gpt-body'))$('#gpt-body').innerHTML=`<p class="muted">Parameter analysis unavailable: ${esc(e.message)}</p>`;$('#cc-body').innerHTML = `<div class="empty"><b>No conclusion for this rule on ${esc(LABEL[tk] || tk)}</b>${esc(e.message)}</div>`;} }
}

/** Default settings + top 3 of `res`: full runs on the viewed window `win`, and the same settings before / after the book. */
async function conclusionData(v, tk, res, win, context={api:API,cost:S.cost,slippage:S.slippage||'0',rollover:S.rollover||'0',pub:PUB,us:isUS(),first:tkSpan(tk).first}) {
  const {api,cost,slippage='0',rollover='0',pub,us,first}=context;
  const def = bookDefaults(v.id), W = {all:{from:'',to:''},before:{from:'',to:`${pub-1}-12-31`},after:{from:`${pub}-01-01`,to:''}};
  const top = (res.rank.top3 || []).map((c, k) => ({ ...rowOf(res, rowAt(res, c.row)), c, name: `#${k + 1}` }));
  const runs = [{ name: 'Default', p: def, book: true }, ...top];
  const stat = async (r, w) => { const d = await runData(v.id, tk, w.from, w.to, cost, r.book ? '' : encodeP(r.p),api,{slippage,rollover}); return d?.error ? { error: d.error } : { d, m: statsM(d.stats) }; };
  await Promise.all(runs.map(async r => {
    const cur = await stat(r, win); if (cur.error) throw new Error(cur.error); r.d = cur.d; r.m = cur.m;
    [r.before, r.after] = await Promise.all([stat(r, W.before), stat(r, W.after)]);
  }));
  const vixFirst = us ? '1990' : '2009', start = v.needs_vix && vixFirst > first.slice(0, 4) ? vixFirst : first.slice(0, 4);
  const noBefore = +start > pub - 1 ? `${v.needs_vix && start === vixFirst ? (us ? 'VIX' : 'India VIX') : LABEL[tk] || tk} data starts in ${start}, after the book (${pub})` : null;
  return { v, tk, res, runs, top, def, win, noBefore, cost:Number(cost || 0),slippage:Number(slippage),rollover:Number(rollover), region:us?'us':'in', context:{...context}, selectionLabel:context.selectionLabel || 'All history', kind: !win.from&&!win.to?'all':!win.from&&win.to===`${pub-1}-12-31`?'before':win.from===`${pub}-01-01`&&!win.to?'after':'custom', pub, made: new Date().toISOString().slice(0, 10) };
}

const dSet = (p, def) => { const ch = Object.keys(def).filter(k => Math.abs((p[k] ?? def[k]) - def[k]) > 1e-9); return ch.length ? ch.map(k => `${sLabel(k)} ${p[k]} (default ${def[k]})`).join(', ') : 'the default settings'; };
const holdText = (a, b) => !Number.isFinite(a) || !Number.isFinite(b) ? '' : b > a ? 'Sharpe increased after the book' : b < a ? 'Sharpe decreased after the book' : 'Sharpe unchanged after the book';
const baText = (r, pub) => r.before?.m && r.after?.m ? `Sharpe ${fx(r.before.m.sharpe)} before ${pub}, ${fx(r.after.m.sharpe)} after (CAGR ${pct(r.before.m.cagr, 2, true)} → ${pct(r.after.m.cagr, 2, true)}): ${holdText(r.before.m.sharpe, r.after.m.sharpe)}` : '';

/** The short summary: four lines, numbers first. */
function ccSummary(D) {
  const [b, t1] = [D.runs[0], D.runs[1]], L = [], rk = D.res.rank;
  if (!t1) return [`No setting traded often enough to rank (needs ${rk.min_trades}+ trades, ${fx(rk.per_year, 1)} a year, on the selection dates).`];
  const bookRank = D.top.findIndex(t => sameP(t.p, D.def));
  L.push(bookRank >= 0 ? `Chosen on ${D.selectionLabel || 'All history'} (${D.res.from.slice(0, 4)}–${D.res.to.slice(0, 4)}): the default settings rank #${bookRank + 1} of ${n0(rk.eligible)} distinct settings.`
    : `Chosen on ${D.selectionLabel || 'All history'} (${D.res.from.slice(0, 4)}–${D.res.to.slice(0, 4)}): #1 changes ${dSet(t1.p, D.def)}, average percentile ${fx(t1.c.score, 1)} on the five ranking stats (weakest ${fx(t1.c.weakest, 1)}).`);
  L.push(`${PERIOD_NAME[D.kind]}: #1 CAGR ${pct(t1.m.cagr, 2, true)} vs default ${pct(b.m.cagr, 2, true)}, Sharpe ${fx(t1.m.sharpe)} vs ${fx(b.m.sharpe)}, max DD ${pct(t1.m.max_dd, 1)} vs ${pct(b.m.max_dd, 1)}, expectancy ${fx(t1.m.expectancy_ratio)} vs ${fx(b.m.expectancy_ratio)}.`);
  if (D.noBefore) L.push(`No before-book test: ${D.noBefore}.`);
  else { if (baText(t1, D.pub)) L.push(`#1 ${baText(t1, D.pub)}.`); if (baText(b, D.pub)) L.push(`Default ${baText(b, D.pub)}.`); }
  return L;
}

function conclusionHtml(D, analyst = null) {
  const r = D.res, rk = r.rank, def = D.def, d0 = D.runs[0].d;
  const head = `<div class="cc-head"><span class="cc-chip">${esc(D.selectionLabel || 'All history')} · ${r.from} to ${r.to}</span><span class="cc-chip">${n0(r.combos)} settings tested</span><span class="cc-chip">${n0(rk.eligible)} distinct ranked (${rk.min_trades}+ trades, ${fx(rk.per_year, 1)} a year)</span><span class="cc-chip">Showing ${esc(PERIOD_NAME[D.kind])} · ${d0.dates[0]} to ${d0.dates[d0.dates.length - 1]}</span><span class="cc-chip">${esc(D.region)} · ${esc(D.cost)} fees + ${esc(D.slippage??0)} slippage bps per side</span></div>`;
  const sum = `<ul class="cc-sum">${(analyst ? analyst.summary : ccSummary(D)).map(t => `<li>${esc(t)}</li>`).join('')}</ul>`;
  const tbl = `<div class="tlog"><table class="tbl cc-tbl"><thead><tr><th></th><th>Settings</th>${statHead()}<th class="n">Score</th><th></th></tr></thead><tbody>
    ${D.runs.map((x, k) => `<tr class="${x.book ? 'book' : k === 1 ? 'pick' : ''}"><td>${x.name}</td><td class="cset">${x.book ? '<span class="muted">default baseline</span>' : setLine(x.p, def)}</td>${statCells(x.m)}<td class="n">${x.c ? fx(x.c.score, 1) : '–'}</td><td class="n">${x.book || sameP(x.p, def) ? '' : applyLink(x.p)}</td></tr>`).join('')}</tbody></table></div>`;
  const BA = CC_STATS;
  const side = (x, k) => x?.m ? BA.map(([f, , fm, sg], j) => `<td class="n ${f === 'max_dd' ? 'neg' : sg ? cls(x.m[f]) : ''}${k === 'a' && !j ? ' ba-split' : ''}">${fm(x.m[f])}</td>`).join('') : `<td class="n muted${k === 'a' ? ' ba-split' : ''}" colspan="${BA.length}">${esc(k === 'b' && D.noBefore ? 'no data before the book' : (x?.error || 'no trades').replace(/^data gap.*/, 'data gap').slice(0, 60))}</td>`;
  const ba = `<h4 class="cc-h">Same settings before and after the book (${D.pub})</h4><div class="tlog"><table class="tbl cc-tbl cc-ba-t"><thead><tr><th rowspan="2"></th><th class="n grp" colspan="${BA.length}">Before ${D.pub}</th><th class="n grp ba-split" colspan="${BA.length}">${D.pub} onward</th></tr>
    <tr>${BA.map(([, t]) => `<th class="n">${t}</th>`).join('')}${BA.map(([, t], j) => `<th class="n${j ? '' : ' ba-split'}">${t}</th>`).join('')}</tr></thead><tbody>
    ${D.runs.map((x, k) => `<tr class="${x.book ? 'book' : k === 1 ? 'pick' : ''}"><td>${x.name}</td>${side(x.before, 'b')}${side(x.after, 'a')}</tr>`).join('')}</tbody></table></div>
    <p class="cap">These settings were selected on ${esc(D.selectionLabel || 'All history')} (${r.from} to ${r.to}). Before/after results are descriptive; the split uses January 1 of the publication year, not a confirmed publication day. It is not an untouched holdout.${D.noBefore ? ` ${esc(D.noBefore)}.` : ''}</p>`;
  return `${head}${sum}${tbl}<p class="cap">Score = average percentile on CAGR, Sharpe, monthly thickness, expectancy ratio and CAR/MDD on the selection dates. Indistinguishable rounded summary outcomes count once; this does not prove identical trades. #1 is highlighted; changed values are in colour; Apply loads that setting on this page.</p>${ba}
    <div class="cc-actions"><button class="dbtn sm" id="cc-more" aria-expanded="false" aria-controls="cc-detail">Detailed stats</button><button class="dbtn sm" id="cc-xlsx">Download Excel</button></div>
    <div id="cc-detail" class="cc-detail" hidden></div>`;
}

function gptConclusionHtml(D, A = VaultAnalyst.analyze(D)) {
  return conclusionHtml(D,A).replaceAll('cc-more','gpt-more').replaceAll('cc-detail','gpt-detail').replaceAll('cc-xlsx','gpt-xlsx')
    + A.candidates.map(c=>`<div class="gpt-finding"><h4 class="cc-h">${esc(c.name)} · analyst assessment</h4><ul class="cc-sum">${[c.summary[0],c.summary[3],c.summary[4],...c.flags].map(t=>`<li>${esc(t)}</li>`).join('')}</ul></div>`).join('');
}
function gptDetailHtml(D,A) {
  const metricFormat=(k,v)=>CC_STATS.find(s=>s[0]===k)[2](v);
  return detailHtml(D)+A.candidates.map(c=>`<h4 class="cc-h">${esc(c.name)} · change from default settings on evaluation dates</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Statistic</th><th class="n">Default</th><th class="n">Candidate</th><th class="n">Difference</th><th>Assessment</th></tr></thead><tbody>${c.comparisons.map(m=>`<tr><td>${esc(m.label)}</td><td class="n">${metricFormat(m.key,m.reference)}</td><td class="n">${metricFormat(m.key,m.value)}</td><td class="n">${m.delta==null?'–':/cagr|net_profit|max_dd|win_rate|thickness/.test(m.key)?fx(m.delta*100,2)+' pp':fx(m.delta,2)}</td><td>${esc(m.assessment)}</td></tr>`).join('')}</tbody></table></div><p class="cap">${esc(c.neighborhood.note)}</p>${c.neighborhood.available?`<div class="tlog"><table class="tbl"><thead><tr><th>One-step sensitivity · ${c.neighborhood.inspected} neighbors</th><th>Candidate</th><th>Neighbor minimum</th><th>Neighbor maximum</th></tr></thead><tbody>${c.neighborhood.metrics.map(m=>`<tr><td>${esc(m.label)}</td><td>${metricFormat(m.key,m.value)}</td><td>${metricFormat(m.key,m.min)}</td><td>${metricFormat(m.key,m.max)}</td></tr>`).join('')}</tbody></table></div>`:''}`).join('')
    +gptRecordsHtml(D)+`<h4 class="cc-h">Research limits</h4><ul>${A.limits.map(t=>`<li>${esc(t)}</li>`).join('')}</ul><p class="cap">Monthly thickness = average closed-trade return / average calendar holding days × 30. It is not observed monthly profit. MaxDD and CAR/MDD use the all-history peak; the underwater display resets its peak each year. Formula definitions, monthly results and closed-trade records are in the workbook.</p>`;
}
function gptRecordsHtml(D) {
  return D.runs.map(run=>{
    const months=OptimCore.monthly(run.d.dates,run.d.equity),years=[...new Set(Object.keys(months).map(k=>k.slice(0,4)))],trades=run.d.trades.filter(t=>!t.open);
    return `<h4 class="cc-h" data-gpt-monthly>${esc(run.name)} · month-on-month returns</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Year</th>${Array.from({length:12},(_,i)=>`<th class="n">${i+1}</th>`).join('')}</tr></thead><tbody>${years.map(y=>`<tr><td>${y}</td>${Array.from({length:12},(_,i)=>{const v=months[y+'-'+String(i+1).padStart(2,'0')];return `<td class="n ${cls(v)}">${v==null?'–':pct(v,2,true)}</td>`;}).join('')}</tr>`).join('')}</tbody></table></div><details data-gpt-trades><summary>${esc(run.name)} · closed trades (${trades.length})</summary><p class="cap">First ${Math.min(100,trades.length)} closed trades shown. All closed trades and verification formulas are in the workbook.</p><div class="tlog"><table class="tbl"><thead><tr><th>Entry</th><th>Exit</th><th class="n">Return after modeled costs</th><th>Exit reason</th></tr></thead><tbody>${trades.slice(0,100).map(t=>`<tr><td>${esc(t.entry)}</td><td>${esc(t.exit)}</td><td class="n ${cls(t.ret)}">${pct(t.ret,3,true)}</td><td>${esc(t.reason)}</td></tr>`).join('')}</tbody></table></div></details>`;
  }).join('');
}
function gptConclusionWire(D) {
  const body=$('#gpt-body');if(!body)return;
  const A=VaultAnalyst.analyze(D);body.innerHTML=gptConclusionHtml(D,A);
  body.onclick=e=>{const b=e.target.closest('[data-apply]');if(b)go(R.path,{...S,...ytPeriod(),p:b.dataset.apply});};
  $('#gpt-more').onclick=()=>{const det=$('#gpt-detail'),open=det.hidden;det.hidden=!open;$('#gpt-more').setAttribute('aria-expanded',String(open));$('#gpt-more').textContent=open?'Hide detailed stats':'Detailed stats';if(open&&!det.innerHTML)det.innerHTML=gptDetailHtml(D,A);};
  $('#gpt-xlsx').onclick=async()=>{const b=$('#gpt-xlsx');b.disabled=true;b.textContent='Building Excel…';try{await exportXlsx(D,true);b.textContent='Download GPT Excel';}catch(e){b.textContent='Excel failed: '+e.message;}finally{b.disabled=false;}};
}

function ratioToLeader(res,c,s) {
  const key=RANK_KEYS[s],g=(res.rank.groups||[]).find(g=>(g.stat==='mar'?'car_mdd':g.stat)===key);
  if(!g?.rows.length)return null;
  const leader=rowOf(res,rowAt(res,g.rows[0])).m[key],value=rowOf(res,rowAt(res,c.row)).m[key];
  return Number.isFinite(leader)&&leader>0&&Number.isFinite(value)?value/leader:null;
}

function conclusionWire(D) {
  const body = $('#cc-body');
  body.onclick = e => { const b = e.target.closest('[data-apply]'); if (b) go(R.path, { ...S, ...ytPeriod(), p: b.dataset.apply }); };
  if ($('#cc-more')) $('#cc-more').onclick = () => { const det = $('#cc-detail'), open = det.hidden; det.hidden = !open; $('#cc-more').setAttribute('aria-expanded', open); $('#cc-more').textContent = open ? 'Hide detailed stats' : 'Detailed stats';
    if (open && !det.innerHTML) det.innerHTML = detailHtml(D); if (open) anim(det, { opacity: [0, 1], transform: ['translateY(-4px)', 'translateY(0)'] }, { duration: .3 }); };
  if ($('#cc-xlsx')) $('#cc-xlsx').onclick = async () => { const b = $('#cc-xlsx'); b.disabled = true; b.textContent = 'Building Excel…';
    try { await exportXlsx(D); b.textContent = 'Download Excel'; } catch (err) { b.textContent = 'Excel failed: ' + err.message; } finally { b.disabled = false; } };
}

// ---------------------------------------------------------------- detailed stats: equity, underwater (peak reset each year), full table, method
const RUN_COL = ['var(--bh)', 'var(--chart)', 'var(--blue)', '#8a6a2f'];
/** Drawdown from a peak that restarts every calendar year at the previous year-end equity. */
function ddYearly(dates, eq) { let peak = 1, y = dates[0].slice(0, 4); return eq.map((v, i) => { const yy = dates[i].slice(0, 4); if (yy !== y) { y = yy; peak = eq[i - 1]; } peak = Math.max(peak, v); return v / peak - 1; }); }
function lineSvg(dates, series, { h = 300, fmt = v => v.toFixed(2), log = false } = {}) {
  const W = 1240, P = { l: 58, r: 12, t: 10, b: 24 }, n = dates.length, step = Math.max(1, Math.floor(n / 600)), idx = []; for (let i = 0; i < n; i += step) idx.push(i); if (idx[idx.length - 1] !== n - 1) idx.push(n - 1);
  const tr = v => log ? Math.log(Math.max(v, 1e-6)) : v, all = series.flatMap(s => idx.map(i => tr(s.vals[i]))).filter(isFinite), lo = Math.min(...all), hi = Math.max(...all, lo + 1e-9);
  const X = i => P.l + i / (n - 1 || 1) * (W - P.l - P.r), Y = v => P.t + (hi - tr(v)) / (hi - lo) * (h - P.t - P.b);
  const ticks = [0, .25, .5, .75, 1].map(f => lo + f * (hi - lo)), years = []; let py = ''; dates.forEach((d, i) => { const y = d.slice(0, 4); if (y !== py && (i === 0 || +y % Math.max(1, Math.ceil(n / 252 / 8)) === 0)) years.push([i, y]); py = y; });
  return `<svg viewBox="0 0 ${W} ${h}" width="100%" class="cc-svg" role="img">${ticks.map(t => `<line x1="${P.l}" x2="${W - P.r}" y1="${Y(log ? Math.exp(t) : t)}" y2="${Y(log ? Math.exp(t) : t)}" stroke="var(--rule)"/><text x="${P.l - 6}" y="${Y(log ? Math.exp(t) : t) + 4}" class="evt" text-anchor="end">${fmt(log ? Math.exp(t) : t)}</text>`).join('')}
    ${years.map(([i, y]) => `<text x="${X(i)}" y="${h - 6}" class="evt" text-anchor="middle">${y}</text>`).join('')}
    ${series.map(s => `<path d="${idx.map((i, k) => `${k ? 'L' : 'M'}${X(i).toFixed(1)},${Y(s.vals[i]).toFixed(1)}`).join('')}" fill="${s.fill ? s.color : 'none'}" fill-opacity="${s.fill ? .18 : 0}" stroke="${s.color}" stroke-width="${s.w || 1.6}"/>`).join('')}</svg>
    <p class="cap">${series.map(s => `<span class="cc-key"><i style="background:${s.color}"></i>${esc(s.name)}</span>`).join(' ')}</p>`;
}
function detailHtml(D) {
  const runs = D.runs, d0 = runs[0].d, rk = D.res.rank;
  const eq = lineSvg(d0.dates, runs.map((x, k) => ({ name: x.name, color: RUN_COL[k], vals: x.d.equity, w: k === 1 ? 2.4 : 1.6 })), { fmt: v => v.toFixed(2) + '×', log: true });
  const uw = lineSvg(d0.dates, [runs[1], runs[0]].filter(Boolean).map(x => ({ name: `${x.name} (peak resets each year)`, color: x.book ? 'var(--bh)' : 'var(--red)', vals: ddYearly(x.d.dates, x.d.equity), fill: !x.book })), { h: 230, fmt: v => (v * 100).toFixed(0) + '%' });
  const years = [...new Set(d0.dates.map(d => d.slice(0, 4)))], ydd = runs.map(x => { const dd = ddYearly(x.d.dates, x.d.equity), o = {}; x.d.dates.forEach((d, i) => { const y = d.slice(0, 4); o[y] = Math.min(o[y] ?? 0, dd[i]); }); return o; });
  const yret = runs.map(x => { const o = {}; let base = 1, y = x.d.dates[0].slice(0, 4); x.d.dates.forEach((d, i) => { const yy = d.slice(0, 4); if (yy !== y) { o[y] = x.d.equity[i - 1] / base - 1; base = x.d.equity[i - 1]; y = yy; } }); o[y] = x.d.equity[x.d.equity.length - 1] / base - 1; return o; });
  const ytbl = `<div class="tlog"><table class="tbl cc-y"><thead><tr><th>Year</th>${runs.map(x => `<th class="n">${x.name} return</th><th class="n">${x.name} max DD</th>`).join('')}</tr></thead><tbody>
    ${years.map(y => `<tr><td>${y}</td>${runs.map((x, k) => `<td class="n ${cls(yret[k][y])}">${pct(yret[k][y], 1, true)}</td><td class="n neg">${pct(ydd[k][y], 1)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;
  const more = [['net_profit', 'Net profit', v => pct(v, 2, true)], ['cagr', 'CAGR', v => pct(v, 2, true)], ['sharpe', 'Sharpe', fx], ['sortino', 'Sortino', fx], ['max_dd', 'Max DD', v => pct(v, 2)], ['max_dd_days', 'Longest DD (bars)', v => fx(v, 0)], ['car_mdd', 'CAR/MDD', fx],
    ['trades', 'Trades', v => fx(v, 0)], ['win_rate', 'Win %', v => pct(v, 2)], ['avg_trade', 'Average trade', v => pct(v, 3, true)], ['avg_win', 'Average win', v => pct(v, 3, true)], ['avg_loss', 'Average loss', v => pct(v, 3)], ['payoff', 'Payoff (win ÷ loss)', fx],
    ['profit_factor', 'Profit factor', fx], ['expectancy_ratio', 'Expectancy ratio', fx], ['avg_hold_days', 'Average holding days', v => fx(v, 1)], ['monthly_thickness', 'Monthly thickness', v => pct(v, 2, true)], ['exposure', 'Time in market', v => pct(v, 1)], ['t_stat', 't-stat of trades', fx], ['max_consec_losses', 'Most losses in a row', v => fx(v, 0)]];
  const stbl = `<table class="tbl cc-full"><thead><tr><th>Stat</th>${runs.map(x => `<th class="n">${x.name}</th>`).join('')}</tr></thead><tbody>${more.map(([k, t, f]) => `<tr><td>${t}</td>${runs.map(x => `<td class="n">${f(x.m[k])}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
  const groups = (rk.groups || []).map(g => `<h4 class="cc-h">Top 5 by ${esc(g.label)}</h4><div class="tlog"><table class="tbl cc-tbl"><thead><tr><th>Rank</th><th>Settings</th>${RANK_KEYS.map(k=>`<th class="n">${esc(RANK_LABEL[k])}</th>`).join('')}</tr></thead><tbody>${g.rows.map((i,j)=>{const x=rowOf(D.res,rowAt(D.res,i));return `<tr><td>${j+1}</td><td>${esc(dSet(x.p,D.def))}</td>${RANK_KEYS.map(k=>`<td class="n">${CC_STATS.find(s=>s[0]===k)[2](x.m[k])}</td>`).join('')}</tr>`;}).join('')}</tbody></table></div>`).join('');
  const pool = (rk.pool || []).slice(0, 25), matrix = `<div class="tlog"><table class="tbl cc-m"><thead><tr><th>#</th><th>Settings</th>${RANK_KEYS.map(k => `<th class="n">${RANK_LABEL[k]}<br><small>value · gap to leader · pct</small></th>`).join('')}<th class="n">Score</th><th class="n">Weakest</th></tr></thead><tbody>
    ${pool.map((c, k) => `<tr class="${k < 3 ? 'pick' : ''}"><td class="rk">${k + 1}</td><td class="cset">${esc(dSet(rowOf(D.res, rowAt(D.res, c.row)).p, D.def))}</td>${RANK_KEYS.map((stat,s)=>{const x=rowOf(D.res,rowAt(D.res,c.row)),g=rk.groups.find(g=>(g.stat==='mar'?'car_mdd':g.stat)===stat),leader=g?.rows.length?rowOf(D.res,rowAt(D.res,g.rows[0])).m[stat]:null,value=x.m[stat],gap=Number.isFinite(value)&&Number.isFinite(leader)?value-leader:null,fmt=CC_STATS.find(q=>q[0]===stat)[2];return `<td class="n">${fmt(value)} · ${gap==null?'–':/cagr|thick/.test(stat)?fx(gap*100,2)+' pp':fx(gap,2)} · <b>${fx(c.pct[s],0)}</b></td>`;}).join('')}<td class="n"><b>${fx(c.score, 1)}</b></td><td class="n">${fx(c.weakest, 0)}</td></tr>`).join('')}</tbody></table></div>`;
  const steps = [['Search', `${n0(D.res.combos)} settings: varied parameters over the recorded grid, with other parameters held at their recorded fixed values, each one backtested on ${D.res.from} to ${D.res.to} in Rust.`],
    ['Filter', `${n0(rk.eligible)} traded regularly enough and were ranked: an average-rate cutoff of ${fx(rk.per_year, 1)} trades a year (half the default setting’s yearly rate, kept between 1 and 5 a year), ${rk.min_trades} closed trades in this period, and never fewer than 10.`],
    ['Percentiles', 'Each of the five stats (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) was turned into a percentile among the ranked settings, 100 = best.'],
    ['Groups', 'The top 5 settings by each stat form five groups (below). Together they are the candidates.'],
    ['Cross-compare', 'Each candidate is compared with every group’s leader on that group’s stat: its value as a share of a positive leader’s, and its percentile. Nonpositive leader ratios are withheld; raw stats remain available.'],
    ['Top 3', 'The three candidates with the highest average percentile are selected. Ties use the higher weakest percentile, then CAGR; a high average can still conceal a weak individual metric.']];
  return `<h4 class="cc-h">Equity, growth of 1 (log scale)</h4>${eq}
    <h4 class="cc-h">Underwater: drawdown with the peak reset each calendar year</h4>${uw}
    <div class="cc-two"><div><h4 class="cc-h">All stats</h4>${stbl}</div><div><h4 class="cc-h">Year by year</h4>${ytbl}</div></div>
    <h4 class="cc-h">How the top 3 were chosen</h4><ol class="evsteps">${steps.map(([h, t]) => `<li><b>${h}</b><span>${t}</span></li>`).join('')}</ol>
    ${groups}<h4 class="cc-h">Cross-comparison of the candidates</h4>${matrix}`;
}

// ---------------------------------------------------------------- Excel (xlsx-js-style: SheetJS with cell styles, loaded on first use)
const loadXlsx = () => window.XLSX ? Promise.resolve() : new Promise((ok, no) => { const s = document.createElement('script'); s.src = 'vendor/xlsx-js-style-1.2.0.min.js'; s.onload = ok; s.onerror = () => no(new Error('could not load the Excel library')); document.head.appendChild(s); });
// minimal palette: black text, dark blue for titles and headers, green / red only to sign numbers; no fills
const XC = { teal: '1F3864', tealDark: '1F3864', soft: 'FFFFFF', ink: '000000', ink2: '000000', ink3: '595959', rule: 'BFBFBF', zebra: 'FFFFFF', green: '1E7B34', red: 'C00000', white: 'FFFFFF', best: 'FFFFFF' };
const RUN_FILL = ['FFFFFF', 'FFFFFF', 'FFFFFF', 'FFFFFF'];
const xFont = (o = {}) => ({ name: 'Calibri', sz: 11, color: { rgb: XC.ink }, ...o });
const xBorder = rgb => ({ top: { style: 'thin', color: { rgb } }, bottom: { style: 'thin', color: { rgb } }, left: { style: 'thin', color: { rgb } }, right: { style: 'thin', color: { rgb } } });
const XS = {
  title: { font: xFont({ sz: 20, bold: true, color: { rgb: XC.teal } }) },
  sub: { font: xFont({ sz: 11, color: { rgb: XC.ink2 } }) },
  h2: { font: xFont({ sz: 13, bold: true, color: { rgb: XC.tealDark } }), border: { bottom: { style: 'medium', color: { rgb: XC.teal } } } },
  th: { font: xFont({ bold: true, color: { rgb: XC.white } }), fill: { fgColor: { rgb: XC.teal } }, alignment: { horizontal: 'center', vertical: 'center', wrapText: true }, border: xBorder(XC.tealDark) },
  thL: { font: xFont({ bold: true, color: { rgb: XC.white } }), fill: { fgColor: { rgb: XC.teal } }, alignment: { horizontal: 'left', vertical: 'center' }, border: xBorder(XC.tealDark) },
  key: { font: xFont({ bold: true }), border: xBorder(XC.rule), alignment: { vertical: 'top' } },
  val: { font: xFont(), border: xBorder(XC.rule), alignment: { horizontal: 'left', vertical: 'top', wrapText: true } },
  bullet: { font: xFont(), alignment: { wrapText: true, vertical: 'top', indent: 1 }, border: { left: { style: 'thick', color: { rgb: XC.teal } } } },
  muted: { font: xFont({ sz: 10, italic: true, color: { rgb: XC.ink3 } }), alignment: { wrapText: true, vertical: 'top' } },
};
const xCell = (z, v, zebra, signed) => ({ font: xFont({ color: { rgb: signed && typeof v === 'number' && isFinite(v) && v !== 0 ? (v > 0 ? XC.green : XC.red) : XC.ink } }), fill: { fgColor: { rgb: zebra ? XC.zebra : XC.white } }, border: xBorder(XC.rule), numFmt: z, alignment: { horizontal: typeof v === 'number' ? 'right' : 'left', vertical: 'center' } });
// light heatmap: pale green for gains, pale red for losses, a little deeper for larger moves (never dark)
const heatFill = (v, cap) => { if (typeof v !== 'number' || !isFinite(v) || !cap || Math.abs(v) < 1e-12) return XC.white; const t = .12 + Math.min(1, Math.abs(v) / cap) * .28, [r, g, b] = v > 0 ? [99, 190, 123] : [248, 105, 107]; const mix = c => Math.round(255 + (c - 255) * t).toString(16).padStart(2, '0'); return (mix(r) + mix(g) + mix(b)).toUpperCase(); };
const xDate = s => (Date.UTC(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)) - Date.UTC(1899, 11, 30)) / 864e5; // Excel serial day
/** A worksheet built cell by cell, so every cell can carry a style. */
function xSheet() {
  const ws = {}, merges = [], rows = []; let maxR = 0, maxC = 0;
  const put = (r, c, v, s = {}) => {
    let cell;
    if (v && typeof v === 'object' && v.f) cell = { t: 'n', f: v.f };
    else if (typeof v === 'number') cell = isFinite(v) ? { t: 'n', v } : { t: 's', v: '' };
    else cell = { t: 's', v: v == null ? '' : String(v) };
    cell.s = s; if (s.numFmt) cell.z = s.numFmt;
    ws[XLSX.utils.encode_cell({ r, c })] = cell; maxR = Math.max(maxR, r); maxC = Math.max(maxC, c);
  };
  return { ws, put, merge: (r, c1, c2, r2 = r) => merges.push({ s: { r, c: c1 }, e: { r: r2, c: c2 } }), height: (r, h) => { rows[r] = { hpt: h }; },
    done(cols, filter, freeze) { ws['!ref'] = XLSX.utils.encode_range({ s: { r: 0, c: 0 }, e: { r: maxR, c: maxC } }); ws['!merges'] = merges; ws['!rows'] = rows; ws['!cols'] = cols.map(w => ({ wch: w })); if (filter) ws['!autofilter'] = { ref: filter }; if (freeze) ws['!freeze'] = freeze; return ws; } };
}
function xTable(sh, r0, c0, head, rows, fmts = [], signed = [], heat = null) {
  head.forEach((h, j) => sh.put(r0, c0 + j, h, j ? XS.th : XS.thL)); sh.height(r0, 30);
  rows.forEach((row, i) => row.forEach((v, j) => { const st = xCell(fmts[j], v, i % 2 === 1, signed[j]); if (heat && heat[j]) { st.fill = { fgColor: { rgb: heatFill(v, heat[j]) } }; if (v === 0) st.font = xFont({ color: { rgb: XC.ink3 } }); } sh.put(r0 + 1 + i, c0 + j, v, st); }));
  return r0 + rows.length + 2;
}
const xH2 = (sh, r, t, w) => { sh.put(r, 0, t, XS.h2); for (let c = 1; c <= w; c++) sh.put(r, c, '', XS.h2); };

/**
 * Workbook: Summary (default + top 3, coloured), Formulas (each stat recomputed with Excel formulas from the trade and
 * daily sheets, next to the engine value), Method (groups + cross-comparison), MoM, Drawdown (peak reset each year),
 * Daily (equity, return, peaks, drawdowns as formulas) and one Trades sheet per run.
 */
async function exportFullGrid(res,v,tk) {
  const context={region:S.region,cost:Number(S.cost||0),slippage:Number(S.slippage||0),rollover:Number(S.rollover||0)};
  const table=VaultGridExport.fullGrid(res,OPT.schemaSync[v.id]||[]);
  await loadXlsx();
  const X=XLSX.utils, wb=X.book_new(), sh=X.aoa_to_sheet([table.header,...table.rows]);
  sh['!autofilter']={ref:X.encode_range({s:{r:0,c:0},e:{r:table.rows.length,c:table.header.length-1}})};
  sh['!cols']=table.header.map(t=>({wch:Math.max(14,Math.min(28,t.length+2))}));
  const percent=/^(total_return|cagr|max_dd|win_rate|avg_trade|avg_net_pnl|exposure|monthly_thickness|gross_total_return|gross_avg_trade|gross_win_rate|gross_max_dd|ledger_gross_return|fee_return|slippage_return)$/;
  for(let r=1;r<=table.rows.length;r++) for(let c=1;c<=table.metricKeys.length;c++) {
    const cell=sh[X.encode_cell({r,c})];if(cell){const format=percent.test(table.metricKeys[c-1])?'0.00%':'0.00';cell.z=format;cell.s=xCell(format,cell.v,r%2===0,percent.test(table.metricKeys[c-1]));}
  }
  table.header.forEach((_,c)=>{const cell=sh[X.encode_cell({r:0,c})];if(cell)cell.s=XS.th;});
  X.book_append_sheet(wb,sh,'All optimization rows');
  X.book_append_sheet(wb,X.aoa_to_sheet([
    ['Field','Value'],['Strategy',v.id],['Method',v.name],['Market',context.region],['Instrument',tk],['From',res.from],['To',res.to],['Cost bps per side',context.cost],['Slippage bps per side',context.slippage],['Rollover bps per actual roll',context.rollover],...(res.experiment?[['Experiment',res.experiment.kind],['Source study reproduced',false],['Experiment basis',res.experiment.basis]]:[]),['Grid combinations',res.combos],['Rows exported',table.rows.length],['Layout','All metric columns first; all varied and fixed parameter columns at the end.'],['Ranking','Existing local full-history ranking; selection is not an untouched holdout test.'],['Data','No simulated prices or missing outcomes added by export. Undefined statistics are blank.']
  ]),'Run metadata');
  XLSX.writeFile(wb,`optimization-all-${v.id}-${tk.replace(/\s/g,'')}-${res.from}_${res.to}.xlsx`);
}

async function exportXlsx(D, analyst = false) {
  await loadXlsx();
  const A=analyst?VaultAnalyst.analyze(D):null;
  const { v, tk, runs, res } = D, X = XLSX.utils, wb = X.book_new(), P = '0.00%', N2 = '0.00', I = '#,##0', DF = 'yyyy-mm-dd', keys = Object.keys(D.def);
  const W = 14, names = runs.map(x => x.name === 'Default' ? 'Default' : `Top ${x.name.slice(1)}`);
  const dates = runs[0].d.dates, nd = dates.length;
  const fmtOf = k => /cagr|net_profit|max_dd|win_rate|thickness|total_return/.test(k) ? P : k === 'trades' ? I : N2;

  // ---- Summary
  const sh = xSheet(); let r = 0;
  sh.put(r, 0, `${analyst?'GPT':'Claude'}’s conclusion: ${familyDisplayName(v)}${v.variant ? ' · ' + v.variant : ''}`, XS.title); sh.height(r, 30); r++;
  sh.put(r, 0, `${LABEL[tk] || tk} · showing ${PERIOD_NAME[D.kind]} ${dates[0]} to ${dates[nd - 1]} · settings chosen on ${D.selectionLabel || 'All history'} ${res.from} to ${res.to} · ${BOOK[v.book]?.title || ''} (${D.pub}) · fees ${D.cost ?? 0} + slippage ${D.slippage ?? 0} bps a side · rollover ${D.rollover ?? 0} bps per actual roll · generated ${D.made}`, XS.sub); r += 2;
  if(res.experiment) {sh.put(r,0,res.experiment.basis,XS.val);sh.merge(r,0,10);sh.height(r,64);r++;}
  xH2(sh, r, `Default settings vs ${analyst?'GPT':'Claude'}’s top 3`, W); r++;
  const best = CC_STATS.map(([k, , , signed]) => { const vals = runs.map(x => x.m[k]).filter(isFinite); return k === 'max_dd' ? Math.max(...vals) : k === 'trades' || k === 'win_rate' ? null : Math.max(...vals); });
  ['Run', 'Settings', ...CC_STATS.map(s => s[1]), 'Score (avg percentile)'].forEach((h, j) => sh.put(r, j, h, j ? XS.th : XS.thL)); sh.height(r, 32); r++;
  runs.forEach((x, i) => {
    const fill = { fgColor: { rgb: RUN_FILL[i] } }, base = { border: xBorder(XC.rule), fill, alignment: { vertical: 'center', wrapText: true } };
    sh.put(r, 0, names[i], { ...base, font: xFont({ bold: true }) });
    sh.put(r, 1, x.book ? 'Default settings · research baseline' : keys.map(k => `${sLabel(k)} ${x.p[k]}${Math.abs(x.p[k] - D.def[k]) > 1e-9 ? ' *' : ''}`).join(' · '), { ...base, font: xFont({ sz: 10 }) });
    CC_STATS.forEach(([k, , , signed], j) => { const val = x.m[k], top = best[j] != null && val === best[j];
      sh.put(r, 2 + j, val, { ...base, numFmt: fmtOf(k), alignment: { horizontal: 'right', vertical: 'center' }, font: xFont({ bold: top, color: { rgb: signed && isFinite(val) && val !== 0 ? (val > 0 ? XC.green : XC.red) : k === 'max_dd' ? XC.red : XC.ink } }), fill: top ? { fgColor: { rgb: XC.best } } : fill }); });
    sh.put(r, 2 + CC_STATS.length, x.c ? x.c.score : '–', { ...base, numFmt: '0.0', alignment: { horizontal: 'right' } });
    sh.height(r, 34); r++;
  });
  sh.put(r, 0, 'Bold = best of the four in that column. * = differs from the default settings. Every stat is recomputed with formulas on the Formulas sheet.', XS.muted); sh.merge(r, 0, W); r += 2;
  xH2(sh, r, 'Summary', W); r++;
  (A ? [...A.summary,...A.candidates.flatMap(c=>c.summary.map(t=>c.name+': '+t)),...A.limits] : ccSummary(D)).forEach(t => { sh.put(r, 0, t, XS.bullet); for (let c = 1; c <= W; c++) sh.put(r, c, '', XS.bullet); sh.merge(r, 0, W); sh.height(r, Math.max(20, Math.ceil(t.length / 170) * 16 + 6)); r++; });
  r++;
  xH2(sh, r, `Same settings before and after the book (${D.pub})`, W); r++;
  if (D.noBefore) { sh.put(r, 0, `No before-book test: ${D.noBefore}.`, XS.muted); sh.merge(r, 0, W); r++; }
  const ms = ['cagr', 'net_profit', 'max_dd', 'win_rate', 'sharpe', 'monthly_thickness', 'trades', 'expectancy_ratio', 'car_mdd'], lines = [];
  runs.forEach((x, i) => [['before', `before ${D.pub}`], ['after', `${D.pub} onward`]].forEach(([k, t]) => { if (x[k]?.m) lines.push([`${names[i]} · ${t}`, x.book ? 'Default settings' : keys.map(q => `${sLabel(q)} ${x.p[q]}`).join(' · '), ...ms.map(f => x[k].m[f] ?? null)]); }));
  r = xTable(sh, r, 0, ['Run · period', 'Settings', ...ms.map(k => CC_STATS.find(s_ => s_[0] === k)[1])], lines, [null, null, ...ms.map(fmtOf)], [0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1]);
  xH2(sh, r, 'Definitions', W); r++;
  [['CAGR', 'final equity ^ (365.25 ÷ calendar days in the test) − 1'], ['Net profit', 'sum of closed-trade net returns; excludes the open position'], ['Max DD', 'largest fall of daily equity from its running (all-time) peak'], ['Win %', 'closed trades with a return above 0 ÷ closed trades'],
    ['Sharpe', 'mean daily return ÷ standard deviation of daily returns (n − 1) × √252, risk-free rate 0'], ['Monthly thickness', 'average trade return ÷ average holding days × 30 (holding days = calendar days entry to exit, a same-day trade counts 1)'],
    ['No. of trades', 'closed trades (a trade still open at the end is excluded)'], ['Expectancy ratio', '(win % × average win − loss % × |average loss|) ÷ |average loss|  (Van Tharp: expected gain per unit of average loss)'], ['CAR/MDD', 'CAGR ÷ |max DD|'],
    ['Score', 'average of the five percentiles (CAGR, Sharpe, monthly thickness, expectancy ratio, CAR/MDD) among the settings that trade often enough (half the default’s yearly rate, 1 to 5 a year, at least 10 trades); see the Method sheet']].forEach(([k, t]) => { sh.put(r, 0, k, XS.key); sh.put(r, 1, t, XS.val); sh.merge(r, 1, W); for (let c = 2; c <= W; c++) sh.put(r, c, '', XS.val); r++; });
  X.book_append_sheet(wb, sh.done([22, 54, ...Array(W).fill(13)]), 'Summary');

  // ---- Trades per run (dates as Excel dates so holding days are a formula)
  const tName = i => `Trades ${names[i]}`.slice(0, 31), tRows = [];
  runs.forEach((x, i) => {
    const t = xSheet(), closed = x.d.trades.filter(q => !q.open);
    t.put(0, 0, `Trades · ${names[i]}`, XS.title); t.height(0, 26);
    t.put(1, 0, x.book ? 'Default settings' : keys.map(k => `${sLabel(k)} ${x.p[k]}`).join(' · '), XS.sub);
    ['#', 'Entry', 'Exit', 'Return', 'Holding days', 'Win', 'Equity after', 'Pieces', 'Avg entry', 'Exit price', 'Exit reason'].forEach((h, j) => t.put(3, j, h, j ? XS.th : XS.thL)); t.height(3, 30);
    closed.forEach((q, k) => { const rr = 4 + k, e = rr + 1, z = k % 2 === 1;
      t.put(rr, 0, k + 1, xCell('0', 1, z)); t.put(rr, 1, xDate(q.entry), xCell(DF, 1, z)); t.put(rr, 2, xDate(q.exit), xCell(DF, 1, z)); t.put(rr, 3, q.ret, xCell(P, q.ret, z, 1));
      t.put(rr, 4, { f: `MAX(1,C${e}-B${e})` }, xCell('0', 1, z)); t.put(rr, 5, { f: `IF(D${e}>0,1,0)` }, xCell('0', 1, z)); t.put(rr, 6, { f: k ? `G${e - 1}*(1+D${e})` : `1+D${e}` }, xCell('0.0000', 1, z));
      t.put(rr, 7, q.legs.length, xCell('0', 1, z)); t.put(rr, 8, q.avg_px, xCell('#,##0.00', 1, z)); t.put(rr, 9, q.exit_px, xCell('#,##0.00', 1, z)); t.put(rr, 10, q.reason, xCell(null, 's', z)); });
    tRows.push(closed.length);
    X.book_append_sheet(wb, t.done([6, 12, 12, 10, 12, 6, 12, 8, 12, 12, 40], `A4:K${4 + closed.length}`, { ySplit: 4 }), tName(i));
  });

  // ---- Daily: equity per run with daily return, all-time peak, drawdown, and the peak reset each calendar year
  const de = xSheet(), C = X.encode_col, per = 5, col0 = i => 1 + i * per; // per run: Equity, Return, Peak, DD, Year peak (+ year DD)
  de.put(0, 0, 'Date', XS.thL);
  runs.forEach((x, i) => ['Equity', 'Daily return', 'Peak', 'Drawdown', 'Yearly DD'].forEach((h, j) => de.put(0, col0(i) + j, `${names[i]} · ${h}`, XS.th)));
  const yp = i => C(col0(runs.length) + i); // hidden helper columns: the yearly-reset peak per run
  runs.forEach((x, i) => de.put(0, col0(runs.length) + i, `${names[i]} · year peak`, XS.th));
  for (let k = 0; k < nd; k++) {
    const rr = k + 1, e = rr + 1; de.put(rr, 0, xDate(dates[k]), { numFmt: DF });
    runs.forEach((x, i) => { const c = col0(i), E = C(c), R_ = C(c + 1), PK = C(c + 2), Y = yp(i);
      de.put(rr, c, x.d.equity[k], { numFmt: '0.000000' });
      de.put(rr, c + 1, { f: k ? `${E}${e}/${E}${e - 1}-1` : `${E}${e}-1` }, { numFmt: '0.0000%' });
      de.put(rr, c + 2, { f: k ? `MAX(${PK}${e - 1},${E}${e})` : `MAX(1,${E}${e})` }, { numFmt: '0.000000' });
      de.put(rr, c + 3, { f: `${E}${e}/${PK}${e}-1` }, { numFmt: '0.00%' });
      de.put(rr, col0(runs.length) + i, { f: k ? `IF(YEAR(A${e})<>YEAR(A${e - 1}),MAX(${E}${e - 1},${E}${e}),MAX(${Y}${e - 1},${E}${e}))` : `MAX(1,${E}${e})` }, { numFmt: '0.000000' });
      de.put(rr, c + 4, { f: `${E}${e}/${Y}${e}-1` }, { numFmt: '0.00%' }); });
  }
  X.book_append_sheet(wb, de.done([12, ...Array(runs.length * (per + 1)).fill(14)], null, { xSplit: 1, ySplit: 1 }), 'Daily');

  // ---- Formulas: every stat recomputed from the Trades and Daily sheets, beside the engine's number
  const fs = xSheet(); let q = 0, L = nd + 1; // last Daily row (Excel)
  fs.put(q, 0, 'Stats recomputed with Excel formulas', XS.title); fs.height(q, 28); q++;
  fs.put(q, 0, 'Each formula reads only the Trades and Daily sheets of this workbook. "Engine" is the number from the Rust API. Small differences (≤ 0.1%) come from the API rounding prices and returns to 6 decimals.', XS.muted); fs.merge(q, 0, 8); fs.height(q, 30); q += 2;
  runs.forEach((x, i) => {
    xH2(fs, q, names[i], 5); q++;
    ['Stat', 'Formula result', 'Engine', 'Difference', 'Check', 'Formula'].forEach((h, j) => fs.put(q, j, h, j ? XS.th : XS.thL)); q++;
    const T = `'${tName(i)}'`, n = tRows[i], rT = c => `${T}!${c}5:${c}${4 + Math.max(1, n)}`, c = col0(i), E = C(c), Rr = C(c + 1), D_ = C(c + 3), first = q + 1;
    const rowsF = [
      ['CAGR', `Daily!${E}${L}^(365.25/(Daily!A${L}-Daily!A2))-1`, 'cagr', P],
      ['Net profit', `SUM(${rT('D')})`, 'net_profit', P],
      ['Max DD', `MIN(Daily!${D_}2:${D_}${L})`, 'max_dd', P],
      ['Win %', `IFERROR(COUNTIF(${rT('D')},">0")/COUNT(${rT('D')}),"")`, 'win_rate', P],
      ['Sharpe', `IFERROR(AVERAGE(Daily!${Rr}2:${Rr}${L})/STDEV(Daily!${Rr}2:${Rr}${L})*SQRT(252),"")`, 'sharpe', N2],
      ['Monthly thickness', `IFERROR(AVERAGE(${rT('D')})/AVERAGE(${rT('E')})*30,"")`, 'monthly_thickness', P],
      ['No. of trades', `COUNT(${rT('D')})`, 'trades', I],
      ['Expectancy ratio', `IFERROR((B${first + 3}*IFERROR(AVERAGEIF(${rT('D')},">0"),0)-(1-B${first + 3})*ABS(IFERROR(AVERAGEIF(${rT('D')},"<=0"),0)))/ABS(IFERROR(AVERAGEIF(${rT('D')},"<=0"),0)),"")`, 'expectancy_ratio', N2],
      ['CAR/MDD', `IFERROR(B${first}/ABS(B${first + 2}),"")`, 'car_mdd', N2]];
    rowsF.forEach(([t, f, k, z], j) => { const e = q + 1, zb = j % 2 === 1;
      fs.put(q, 0, t, { ...XS.key }); fs.put(q, 1, { f }, xCell(z, 1, zb)); fs.put(q, 2, x.m[k], xCell(z, 1, zb));
      fs.put(q, 3, { f: `IF(AND(ISNUMBER(B${e}),ISNUMBER(C${e})),B${e}-C${e},"")` }, xCell('0.000000', 1, zb));
      fs.put(q, 4, { f: `IF(AND(ISNUMBER(B${e}),ISNUMBER(C${e})),IF(ABS(B${e}-C${e})<=0.001*MAX(1,ABS(C${e})),"match","differs"),"unavailable")` }, { ...xCell(null, 's', zb), font: xFont({ bold: true, color: { rgb: XC.green } }) });
      fs.put(q, 5, '=' + f, { font: xFont({ sz: 9, color: { rgb: XC.ink3 } }) }); q++; });
    q++;
  });
  X.book_append_sheet(wb, fs.done([20, 16, 14, 14, 10, 120]), 'Formulas');

  // ---- Method: the groups and the cross-comparison behind the top 3
  const rk = res.rank, ms_ = xSheet(); let m = 0;
  ms_.put(m, 0, 'How the top 3 were chosen', XS.title); ms_.height(m, 28); m += 2;
  [`1. Search: ${n0(res.combos)} settings (every parameter over its range) backtested on ${res.from} to ${res.to}.`, `2. Filter: ${n0(rk.eligible)} settings are ranked: at least ${fx(rk.per_year, 1)} trades a year (half the default setting’s yearly rate, between 1 and 5 a year), i.e. ${rk.min_trades} closed trades here, never fewer than 10.`,
    '3. Percentiles: CAGR, Sharpe, monthly thickness, expectancy ratio and CAR/MDD are each turned into a percentile (0–100, 100 = best) among the ranked settings.',
    '4. Groups: the top 5 by each stat form five groups; together they are the candidates.', '5. Cross-compare: each candidate is set against each group leader on that group’s stat (ratio only when the leader is positive, and percentile).',
    '6. Top 3: highest average percentile across the five stats; ties go to the higher weakest percentile, then CAGR.'].forEach(t => { ms_.put(m, 0, t, XS.val); ms_.merge(m, 0, 9); m++; });
  m++;
  const cand = i => rowOf(res, rowAt(res, i));
  (rk.groups || []).forEach(g => { const k = g.stat === 'mar' ? 'car_mdd' : g.stat; xH2(ms_, m, `Top 5 by ${g.label}`, 9); m++;
    m = xTable(ms_, m, 0, ['#', 'Settings', ...RANK_KEYS.map(x => RANK_LABEL[x])], g.rows.map((i, j) => { const c = cand(i); return [j + 1, keys.map(x => `${sLabel(x)} ${c.p[x]}`).join(' · '), ...RANK_KEYS.map(x => c.m[x])]; }),
      ['0', null, P, N2, P, N2, N2], [0, 0, 1, 1, 1, 1, 1]); });
  xH2(ms_, m, 'Cross-comparison of every candidate (value as % of the group leader · percentile)', 9); m++;
  m = xTable(ms_, m, 0, ['Rank', 'Settings', ...RANK_KEYS.flatMap(x => [`${RANK_LABEL[x]} % of leader`, `${RANK_LABEL[x]} percentile`]), 'Score', 'Weakest'],
    (rk.pool || []).map((c, j) => [j + 1, keys.map(x => `${sLabel(x)} ${cand(c.row).p[x]}`).join(' · '), ...RANK_KEYS.flatMap((x, s) => [ratioToLeader(res,c,s), c.pct[s]]), c.score, c.weakest]),
    ['0', null, ...RANK_KEYS.flatMap(() => [P, '0.0']), '0.0', '0.0'], [], [0, 0, ...RANK_KEYS.flatMap(() => [0, 100]), 100, 100]);
  X.book_append_sheet(wb, ms_.done([8, 60, ...Array(12).fill(14)]), 'Method');

  // ---- Month on month: one grid per top-3 setting. Years down, months across; Year = compounded year return, Avg = average month;
  // the Avg row averages each month over all years. Totals and averages are Excel formulas (blank = no data that month).
  const mo = xSheet(); let o = 0;
  mo.put(o, 0, 'Month-on-month P&L', XS.title); mo.height(o, 28); o++;
  mo.put(o, 0, 'Month-end equity ÷ previous month-end equity − 1. Light green = gain, light red = loss (deeper = larger); 0.00% = no position that month. Year = the 12 months compounded; Avg = average month of that year; the Avg row = average of that month over all years.', XS.muted); mo.merge(o, 0, 14); mo.height(o, 30); o += 2;
  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const numSt = (v, bold, cap) => ({ fill: { fgColor: { rgb: cap ? heatFill(v, cap) : XC.white } }, font: xFont({ bold, color: { rgb: typeof v === 'number' && isFinite(v) && Math.abs(v) > 1e-12 ? (v > 0 ? XC.green : XC.red) : XC.ink } }), numFmt: P, border: xBorder(XC.rule), alignment: { horizontal: 'right' } });
  const avg = a => { const b = a.filter(x => x != null && isFinite(x)); return b.length ? b.reduce((p, q) => p + q, 0) / b.length : null; };
  runs.forEach((x, i) => { if (x.book) return;
    const m = OptimCore.monthly(x.d.dates, x.d.equity), years = [...new Set(Object.keys(m).map(k => k.slice(0, 4)))];
    xH2(mo, o, `${names[i]} · ${keys.map(k => `${sLabel(k)} ${x.p[k]}`).join(' · ')}`, 14); o++;
    ['Year', ...MON, 'Year', 'Avg month'].forEach((h, j) => mo.put(o, j, h, j ? XS.th : XS.thL)); o++;
    const top = o + 1, rowVals = [], capM = Math.max(1e-9, ...Object.values(m).filter(isFinite).map(Math.abs)), capY = Math.max(1e-9, ...years.map(y => Math.abs(MON.reduce((p, _, k) => p * (1 + (m[`${y}-${String(k + 1).padStart(2, '0')}`] ?? 0)), 1) - 1)));
    years.forEach(y => { const e = o + 1, vals = MON.map((_, k) => m[`${y}-${String(k + 1).padStart(2, '0')}`] ?? null); rowVals.push(vals);
      mo.put(o, 0, +y, { font: xFont({ bold: true }), border: xBorder(XC.rule) });
      vals.forEach((v, k) => mo.put(o, k + 1, v == null ? '' : v, numSt(v, false, capM)));
      const yr = vals.reduce((p, v) => v == null ? p : p * (1 + v), 1) - 1, am = avg(vals);
      mo.put(o, 13, { f: `SUMPRODUCT(PRODUCT(1+B${e}:M${e}))-1` }, numSt(yr, true, capY));
      mo.put(o, 14, { f: `AVERAGE(B${e}:M${e})` }, numSt(am, false, capM)); o++; });
    const bot = o, col = j => X.encode_col(j);
    mo.put(o, 0, 'Avg', { font: xFont({ bold: true, color: { rgb: XC.teal } }), border: { ...xBorder(XC.rule), top: { style: 'medium', color: { rgb: XC.teal } } } });
    for (let j = 1; j <= 14; j++) { const v = j <= 12 ? avg(rowVals.map(r_ => r_[j - 1])) : j === 13 ? avg(rowVals.map(r_ => r_.reduce((p, q) => q == null ? p : p * (1 + q), 1) - 1)) : avg(rowVals.map(avg));
      mo.put(o, j, { f: `AVERAGE(${col(j)}${top}:${col(j)}${bot})` }, { ...numSt(v, true), border: { ...xBorder(XC.rule), top: { style: 'medium', color: { rgb: XC.teal } } } }); }
    o += 3; });
  X.book_append_sheet(wb, mo.done([10, ...Array(12).fill(9), 10, 11]), 'MoM');

  // ---- Drawdown by year (peak reset each calendar year) and the calendar-year returns
  const yrs = [...new Set(dates.map(d => d.slice(0, 4)))], ydd = runs.map(x => { const dd = ddYearly(x.d.dates, x.d.equity), w = {}; x.d.dates.forEach((d, i) => { const y = d.slice(0, 4); w[y] = Math.min(w[y] ?? 0, dd[i]); }); return w; });
  const yret = runs.map(x => { const w = {}; let base = 1, y = x.d.dates[0].slice(0, 4); x.d.dates.forEach((d, i) => { const yy = d.slice(0, 4); if (yy !== y) { w[y] = x.d.equity[i - 1] / base - 1; base = x.d.equity[i - 1]; y = yy; } }); w[y] = x.d.equity[x.d.equity.length - 1] / base - 1; return w; });
  const dd = xSheet(); let z = 0; dd.put(z, 0, 'Drawdown by year (peak reset each calendar year)', XS.title); dd.height(z, 28); z += 2;
  const capD = Math.max(...ydd.flatMap(w => Object.values(w)).map(Math.abs), 1e-9), capY = Math.max(...yret.flatMap(w => Object.values(w)).map(Math.abs), 1e-9);
  xTable(dd, z, 0, ['Year', ...names.flatMap(n => [`${n} return`, `${n} max DD`])], yrs.map(y => [y, ...runs.flatMap((_, i) => [yret[i][y] ?? null, ydd[i][y] ?? null])]),
    [null, ...runs.flatMap(() => [P, P])], [], [0, ...runs.flatMap(() => [capY, capD])]);
  X.book_append_sheet(wb, dd.done([8, ...Array(runs.length * 2).fill(14)]), 'Drawdown');

  // Sheet order: Summary, Formulas, Method, MoM, Drawdown, Trades…, Daily
  const order = ['Summary', 'Formulas', 'Method', 'MoM', 'Drawdown', ...runs.map((_, i) => tName(i)), 'Daily'];
  wb.SheetNames = order.filter(n => wb.SheetNames.includes(n));
  XLSX.writeFile(wb, `${analyst?'gpt-':''}conclusion-${v.id}-${tk.replace(/\s/g, '')}-${dates[0]}_${dates[nd - 1]}.xlsx`);
}

const cv = n => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
const rgba_ = (hex, a) => { const h = hex.replace('#', ''); return `rgba(${parseInt(h.slice(0, 2), 16)},${parseInt(h.slice(2, 4), 16)},${parseInt(h.slice(4, 6), 16)},${a})`; };
let API = window.VAULT_APP ? '/api/in' : (new URLSearchParams(location.search).get('api') || `http://${location.hostname || 'localhost'}:8090`).replace(/\/$/, '') + '/api';
const API_IN = API, API_US = window.VAULT_APP ? '/api/us' : `http://${location.hostname || 'localhost'}:8091/api`, BOOKCTX = { stc9: ['1995-01-01', '2009-12-31', 2010], etfss: ['2000-01-03', '2004-06-16', 2004], pctb: ['1994-01-03', '2004-06-30', 2004], rsi25: ['1993-01-29', '2003-12-31', 2004], spmt: ['1997-09-09', '2006-11-30', 2007], vixr: ['1993-01-28', '2001-08-06', 2001], otvt: ['1993-01-28', '1997-10-01', 1999], ptjmt: ['1993-01-28', '1997-10-31', 1999], ats: ['1993-01-28', '1997-10-31', 1998], hfm: ['1993-01-29', '1994-12-31', 1995], btp: ['1993-01-29', '1999-12-31', 1999], ptjdt: ['1993-01-29', '1999-12-31', 1999], ws5: ['1993-01-29', '2003-06-30', 2003], cwin: ['1994-01-03', '2002-12-31', 2003], ss: ['1993-01-29', '1995-12-31', 1996], oiss: ['1993-01-29', '2004-06-30', 2004], lrd: ['1995-01-01', '1996-12-31', 2001], alpha: ['1993-01-29', '2018-12-31', 2020], ststw: ['1995-01-03', '2007-12-31', 2008], hpetf: ['1993-01-29', '2008-12-31', 2009], hmrw: ['1993-01-29', '2011-12-31', 2012], radge: ['1993-01-29', '2011-12-31', 2012], covel: ['1993-01-29', '2006-12-31', 2007], tharp: ['1993-01-29', '1998-12-31', 1999], afactor: ['2000-01-03', '2004-12-31', 2005] }; // US default window = each book's own test period, + publication year
let BOOK_FROM = '1995-01-01', BOOK_TO = '2009-12-31';
const bookCtx = id => {
 [BOOK_FROM,BOOK_TO,PUB]=bookContext(id,BOOKCTX,BOOK);
 TABS.working.pre=PUB?`${PUB-1}-12-31`:undefined;
 TABS.working.metric.label=PUB?`Sharpe since ${PUB}`:'Publication year unconfirmed';
 TABS.working.note=PUB?`Compare Sharpe since publication (${PUB}) with the period before it. Missing history remains unavailable; this is a research comparison, not a certified holdout.`:'The publication year is not established for this book.';
};
const srcOf = v => v.api === 'yt' ? 'Team-supplied AFL rules' : v.guru === 'ross' ? (sourceEvidence(v)?.pages || 'Source reference') : v.session ? `Session ${v.session}` : 'Part III';
let TICK_IN = [], TICK_US = null;
const IN_INDICES = ['NIFTY 50', 'NIFTYBANK', 'NIFTYMIDCAP100'], US_INDICES = ['SPX', 'NDX', 'DJI']; // the Indian indices the vault shows (Nifty IT and Midcap Select are left out)
const isUS = () => API === API_US;
const $ = s => document.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const pct = (v, d, s = false) => v == null || !isFinite(v) ? '–' : (s && v > 0 ? '+' : '') + (v * 100).toFixed(2) + '%'; // every percentage at 2 decimals
const fx = (v, d = 2) => v == null || !isFinite(v) ? '–' : v.toFixed(d);
const cls = v => v == null ? '' : v >= 0 ? 'pos' : 'neg';
const med = a => { const s = a.filter(isFinite).sort((x, y) => x - y), n = s.length; return n ? (n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2) : null; };
const LABEL = { 'NIFTY 50': 'Nifty 50', NIFTYBANK: 'Bank Nifty', NIFTYIT: 'Nifty IT', NIFTYMIDCAP100: 'Midcap 100', NIFTYMIDCAPSELECT: 'Midcap Select', SPY: 'SPY (S&P 500)', QQQ: 'QQQ (Nasdaq 100)', MDY: 'MDY (S&P MidCap 400)', IWM: 'IWM (Russell 2000)', SMH: 'SMH (Semiconductors)', SPX: 'S&P 500', NDX: 'Nasdaq 100', DJI: 'Dow Jones' };
const CLOTH = ['#1f3a5f', '#7a2e2e', '#2f5140', '#8a6a2f', '#4a5563', '#5b3b5e'];

// ---------------------------------------------------------------- the library (hierarchy: guru → book → strategy → variant)
// Only Connors / Swing Trading College IX is converted. The rest are clearly marked previews (no numbers) so the layout
// shows how the vault grows; they become live automatically once their strategies exist in the API.
let BOOK_COVERAGE={authors:{}};
let BOOK_FILE_INDEX={files:{}};
let BOOK_CATEGORIES={books:{},source_titles:{}};
let SOURCE_REVIEWS={sources:{},catalogue:{}};
let LIBRARY_PROGRESS=null;
let PROGRESS_CACHE=null,PROGRESS_DEFS=null,PROGRESS_SNAPSHOT=null;
function currentLibraryProgress(){if(!PROGRESS_CACHE||PROGRESS_DEFS!==ALLCAT||PROGRESS_SNAPSHOT!==LIBRARY_PROGRESS){PROGRESS_DEFS=ALLCAT;PROGRESS_SNAPSHOT=LIBRARY_PROGRESS;PROGRESS_CACHE=libraryProgress(ALLCAT,LIBRARY_PROGRESS);}return PROGRESS_CACHE;}
const recordProgress=record=>record.kind==='catalogue'?currentLibraryProgress().book(record.id):currentLibraryProgress().source(record.id,recordAuthorIds(record));
const recordAuthorIds=row=>row.authorIds?.length?row.authorIds:[row.authorId];
function scopedLibraryRecords(records){const groups=new Map();for(const row of records)for(const id of recordAuthorIds(row)){if(!groups.has(id))groups.set(id,[]);groups.get(id).push({...row,authorId:id,author:GURU[id]||row.author,authorName:GURU[id]?.name||row.authorName});}return groups;}
let LIBRARY_RECORD_CACHE=null,LIBRARY_AUTHOR_CACHE=null;
let SOURCE_ROW_BY_ID=new Map();
const CATEGORY_LABELS={'rule-based':'Rule-based',psychology:'Psychology','money-management':'Money management'};
const categoryLookup=(id,item,author,kind)=>kind==='source'?BOOK_CATEGORIES.source_titles?.[id]||item?.category_review:BOOK_CATEGORIES.books?.[id];
const libraryRecords=()=>LIBRARY_RECORD_CACHE||(LIBRARY_RECORD_CACHE=VaultLibraryBrowser.records(LIB,BOOK_COVERAGE,categoryLookup));
function authorLibraryRecords(id){if(!LIBRARY_AUTHOR_CACHE)LIBRARY_AUTHOR_CACHE=scopedLibraryRecords(libraryRecords());return LIBRARY_AUTHOR_CACHE.get(id)||[];}
function libraryEntryCounts(id){const rows=authorLibraryRecords(id);return {total:rows.length,catalogue:rows.filter(r=>r.kind==='catalogue').length,reading:rows.filter(r=>r.kind==='source').length};}
function mergeResearchLibrary(data){
  if(data?.schema_version!==1||!data.authors||Array.isArray(data.authors))return;
  for(const [id,author] of Object.entries(data.authors)){
    if(!/^[A-Za-z0-9_-]{1,96}$/.test(id)||!Array.isArray(author.source_titles))continue;
    let g=GURU[id];
    if(!g){g={id,name:author.name==='?'?'Recorded author unconfirmed':String(author.name||'Recorded credit unconfirmed'),books:[],source_group:true,tagline:'Reading sources from the research share. Recorded credits and book identities are under review; no strategy performance is inferred from the inventory.'};LIB.push(g);GURU[id]=g;}
    const old=BOOK_COVERAGE.authors[id]||{};
    const sources=author.source_titles.filter(s=>s&&typeof s.id==='string'&&typeof s.title==='string').map(s=>({...s,metadata_title:s.title,title:s.title.split(/\s*\|\|\s*snippet:/i)[0].trim(),files:Array.isArray(s.files)?s.files:[]}));
    BOOK_COVERAGE.authors[id]={...old,...author,id,name:g.name,source_titles:sources,counts:{...old.counts,...author.counts,catalogue_books:g.books.length},catalogue_books:old.catalogue_books||[]};
  }
}
function applyReadingReviews(data){
  if(data?.schema_version!==1||!data.sources)return;
  SOURCE_REVIEWS={sources:{...SOURCE_REVIEWS.sources,...data.sources},catalogue:{...SOURCE_REVIEWS.catalogue,...data.catalogue}};
  for(const author of Object.values(BOOK_COVERAGE.authors))for(const source of author.source_titles||[]){
    const reviewed=data.sources[source.id];if(!reviewed)continue;
    Object.assign(source,{summary:reviewed.summary||source.summary,review:reviewed.review,entry_kind_label:reviewed.entry_kind_label,source_identity:reviewed.source_identity,summary_status:'bounded_actual_source_review'});
    if(reviewed.source_identity?.title)source.title=reviewed.source_identity.title;
    if(reviewed.source_identity?.credit)source.author_metadata=reviewed.source_identity.credit;
    source.category_review={categories:reviewed.categories||[],certainty:'bounded_actual_source_sections',evidence:reviewed.category_evidence||[]};
    BOOK_CATEGORIES.source_titles??={};BOOK_CATEGORIES.source_titles[source.id]=source.category_review;
  }
  for(const g of LIB)for(const b of g.books){
    const reviewed=data.catalogue?.[b.id];if(reviewed?.source_ids)b.sourceIds=[...new Set([...(b.sourceIds||[]),...reviewed.source_ids])];
    if(reviewed?.credit)b.author_credit=reviewed.credit;
    const actual=bookSources(g,b).find(s=>s.review);
    if(actual){SOURCE_REVIEWS.catalogue[b.id]={...reviewed,summary:reviewed?.summary||actual.summary};BOOK_CATEGORIES.books[b.id]=reviewed?.categories?{categories:reviewed.categories,certainty:'bounded_actual_source_sections',evidence:reviewed.category_evidence||[]}:actual.category_review;}
  }
}
// Whole-share review: each trading source is re-homed under the people credited on it (one card per person;
// curated pages keep their id). Sources with no credited person go to one collection; non-trading ones leave the trading scope.
function applyShareReview(data){
  if(data?.schema_version!==1||!data.sources)return;
  const group=(id,name)=>{let g=GURU[id];if(!g){g={id,name,books:[],source_group:true,tagline:'Reading sources from the research share, each checked from its first pages. No strategy performance is inferred from the inventory.'};LIB.push(g);GURU[id]=g;}return g;};
  for(const [id,name] of Object.entries(data.people||{}))if(/^[A-Za-z0-9_-]{1,96}$/.test(id))group(id,name).person=true;
  for(const id of data.curated||[])if(GURU[id])GURU[id].person=true;
  const moved=new Map();
  for(const author of Object.values(BOOK_COVERAGE.authors))author.source_titles=(author.source_titles||[]).filter(source=>{
    const review=data.sources[source.id];if(!review)return true;
    if(review.not_trading){source.reviewed_scope='pending';return true;}
    if(!moved.has(source.id))moved.set(source.id,{...source,reviewed_scope:'trading',reviewed_type:review.type});
    return false;
  });
  for(const [id,source] of moved){
    const ids=data.sources[id].people.filter(pid=>GURU[pid]);
    const owners=ids.length?ids:[group('uncredited','Sources with no credited author').id];
    source.author_metadata=ids.length?ids.map(pid=>GURU[pid].name).join(' · '):'No credited author';
    for(const pid of owners){const g=GURU[pid];BOOK_COVERAGE.authors[pid]??={id:pid,name:g.name,source_titles:[],catalogue_books:[]};BOOK_COVERAGE.authors[pid].source_titles.push(source);}
  }
  for(const g of LIB)for(const b of g.books){const ids=(data.catalogue?.[b.id]||[]).filter(id=>id!==g.id&&GURU[id]);if(ids.length)b.coAuthorIds=ids;}
  // A source the review identified as a copy of a catalogue book merges into that book (reviewed source id).
  for(const [id,review] of Object.entries(data.sources))if(review.book&&BOOK[review.book])BOOK[review.book].sourceIds=[...new Set([...(BOOK[review.book].sourceIds||[]),id])];
  for(let i=LIB.length-1;i>=0;i--){const g=LIB[i];if(g.source_group&&!g.person&&g.id!=='uncredited'&&!g.books.length&&!BOOK_COVERAGE.authors[g.id]?.source_titles?.length){LIB.splice(i,1);delete GURU[g.id];}}
}
function finalizeReadingIndex(){
  SOURCE_ROW_BY_ID=new Map();
  for(const g of LIB){const sources=BOOK_COVERAGE.authors[g.id]?.source_titles||[];
    for(const b of g.books)for(const id of b.sourceIds||[]){const source=sources.find(s=>s.id===id);if(source){source.catalogue_candidates??=[];if(!source.catalogue_candidates.some(c=>(typeof c==='string'?c:c.id)===b.id))source.catalogue_candidates.push({id:b.id,match:'explicit_verified_original_reading_copy_association'});}}
    for(const source of sources)if(!SOURCE_ROW_BY_ID.has(source.id))SOURCE_ROW_BY_ID.set(source.id,{author:g,source});
  }
  LIBRARY_RECORD_CACHE=null;LIBRARY_AUTHOR_CACHE=null;
}
const canonicalCategory=c=>({rule_based:'rule-based',money_management:'money-management'})[c]||c;
function categoryTags(b,kind='catalogue'){const entry=categoryLookup(b.id,b,null,kind),cats=(entry?.categories||[]).map(canonicalCategory);return cats.length?`<div class="book-types">${cats.map(c=>`<span title="Topic classification; independent backtest verification is separate">${esc(CATEGORY_LABELS[c]||c)}</span>`).join('')}</div>`:'<p class="category-pending">Topic review pending</p>';}
function sourcePdfLinks(sources){
  const paths=new Set(sources.flatMap(b=>[...(b.local_copies||[]).map(x=>x.path),...(b.files||[]).flatMap(f=>[...(f.local_candidates||[]),...(f.verified_local_copy?.path?[f.verified_local_copy.path]:[])]),...(b.verified_local_copy?.path?[b.verified_local_copy.path]:[])]));
  const admitted=[...paths].map(path=>BOOK_FILE_INDEX.files[path.replace(/^Books\//,'')]).filter(Boolean);
  return admitted.length?`<details class="source-pdfs"><summary>Read original files (${admitted.length})</summary><ul>${admitted.map(f=>`<li><a class="olink" href="/api/app/books/files/${encodeURIComponent(f.id)}" target="_blank" rel="noopener" onclick="event.stopPropagation()">${esc(f.filename)}</a>${f.disposition==='attachment'?' <small>Download original document</small>':''}</li>`).join('')}</ul><p class="muted">Files are served from the checked local reading cache. Filename-based catalogue associations may require bibliographic review; a readable copy does not certify authorship, rules or published performance.</p></details>`:'';
}
function bookSources(g,b){const sources=(BOOK_COVERAGE.authors[g.id]?.source_titles||[]).filter(s=>b.sourceIds?.includes(s.id)||(s.catalogue_candidates||[]).some(c=>(typeof c==='string'?c:c.id)===b.id));return sources.length?sources:b.readerPaths?.length?[{local_copies:b.readerPaths.map(path=>({path}))}]:[];}
const pristineReviewedSource=()=>ALLCAT.some(v=>pristineMethod(v.id,v.book)&&v.instrument_policy?.source_rule_reviewed===true);
const pristineSourceNote='Selected TPM1 daily CBS/COG and Bullish Gap Surprise branches are source-reviewed under declared supplied-series conventions. Other Pristine sources and original book performance do not inherit this review.';
function pristineAuthorStatus(state){if(!state)return;if(pristineReviewedSource()){state.title='Selected TPM1 rules reviewed';state.text=pristineSourceNote+' Choose an actual US stock and explicit research mode; basket admission remains separate.';}else{state.title='Rule review pending';state.text='Original Pristine books are readable. Complete trading rules and execution assumptions still need review; no verified strategy is registered.';}}
function readingSummary(g,b){if(g.id==='pristine'&&b.id==='p_tpm1'&&pristineReviewedSource())return pristineSourceNote;if(g.id==='kestner'&&b.id==='lkqts')return b.blurb;return SOURCE_REVIEWS.catalogue?.[b.id]?.summary||BOOK_COVERAGE.authors[g.id]?.catalogue_books?.find(x=>x.id===b.id)?.summary||b.blurb||BOOK_BLURB[b.id]||'This book is indexed as reading material. Its content and objective strategy rules have not completed independent review; no performance is attributed to it.';}
function readingReviewHTML(source){
  const review=source?.review;if(!review)return '';
  const evidence=(review.evidence||[]).map(e=>`<li><strong>${esc(e.file||'Source document')}</strong>${e.page_range?` · PDF ${esc(Array.isArray(e.page_range)?e.page_range.map(p=>Array.isArray(p)?p[0]===p[1]?String(p[0]):p.join('–'):p).join(', '):e.page_range)}`:''}<p>${esc(e.finding||'')}</p></li>`).join('');
  const outline=(review.outline||[]).slice(0,8),rules=review.rules||[];
  return `<section class="box" data-reading-review><div class="box-h">Source content review <small>${esc(review.status_label||'Bounded source review')}</small></div><p class="rnote">Reviewed source sections are listed below. ${esc(review.reading_limits||'This is a bounded content review; it does not certify the complete book or original trading performance.')}</p>${outline.length?`<figure class="reading-topic-map" data-reading-concept-map><figcaption>Topics in the reviewed sections <small>Educational topic diagram · no backtest results</small></figcaption><div>${outline.map(t=>`<article><strong>${esc(t.label)}</strong><p>${esc(t.detail||'')}</p></article>`).join('')}</div></figure>`:''}${rules.length?`<h3>Rules and conversion assessment</h3><div class="tlog"><table class="tbl"><thead><tr><th>Method</th><th>What is specified</th><th>What is needed</th></tr></thead><tbody>${rules.map(r=>`<tr><td><strong>${esc(r.name)}</strong><p>${r.complete?'Complete for the stated scope':esc(r.completeness_kind||'Incomplete entry/exit system')}</p></td><td>${[r.inputs&&`Inputs: ${Array.isArray(r.inputs)?r.inputs.join(', '):r.inputs}`,r.entry&&`Entry: ${r.entry}`,r.exit&&`Exit: ${r.exit}`].filter(Boolean).map(t=>`<p>${esc(t)}</p>`).join('')}</td><td>${[Array.isArray(r.data_requirement)?r.data_requirement.join('; '):r.data_requirement,Array.isArray(r.missing)?r.missing.join('; '):r.missing].filter(Boolean).map(t=>`<p>${esc(t)}</p>`).join('')||'Original-book study and full implementation verification remain separate.'}</td></tr>`).join('')}</tbody></table></div>`:''}${evidence?`<details><summary>Reviewed pages and source evidence</summary><ul>${evidence}</ul></details>`:''}</section>`;
}
const BOOK = {}; LIB.forEach(g => g.books.forEach((b, k) => { b.guru = g; b.c = CLOTH[(LIB.indexOf(g) * 2 + k) % CLOTH.length]; BOOK[b.id] = b; }));
const GURU = Object.fromEntries(LIB.map(g => [g.id, g]));
const bookYear = b => b.year || 'Edition year unconfirmed';
const ownerOf = id => ALLCAT.find(v => v.id === id)?.api || (pristineMethod(id,PRISTINE_BOOK)?'pristine':id.startsWith('jr_') ? 'ross' : id.startsWith('ch_') ? 'chande' : /^yt[a-z]+_/.test(id) ? 'yt' : 'connors');
const enginePath = (id, endpoint, api = API) => `${api}/guru/${ownerOf(id)}/${endpoint}`;
const guruIds = () => GURUS_API.map(g => g.id);
function sourceEvidence(v) { return v.guru==='chande'?window.ChandeEvidence.methods[v.id]:window.JoeRossEvidence.methods[v.id]; }
// A table comparison records its source discrepancies; it is not source-performance certification.
const gateBadge = () => ''; // verification badges hidden (user request 22 Oct 2026); gate data kept for a future UI
const OPEN_FROM = { SPX: '2014-01-01', NDX: '2001-01-01', DJI: '2016-01-01' };
function gateNotice(v, tk) {
  const g = v.gate;
  const clamp = g?.open_from && OPEN_FROM[tk] ? `<p data-open-price-coverage><b>${esc(LABEL[tk] || tk)}: opening prices before ${OPEN_FROM[tk]} are excluded by policy.</b> The actual calculated dates are shown with the results. An earlier requested start does not add earlier price observations.</p>` : '';
  const declared=v.instrument_policy?.declared_application&&!String(tk||'').startsWith('basket:')&&S.run_scope!=='basket'?`<div class="rnote" data-declared-application><b>Declared application · not the author's test.</b> ${esc(v.instrument_policy.declared_application)}</div>`:'';
  if(declared)return declared+gateNotice({...v,instrument_policy:{...v.instrument_policy,declared_application:''}},tk);
  if(v.instrument_policy?.source_rule_reviewed)return `<div class="rnote" data-rule-review><b>Source rules reviewed · computed market application.</b> Selected trade ledgers matched an independent reference calculation. ${esc(v.instrument_policy.source_basis)} ${clamp} ${v.gate?.book?`<details><summary>Earlier source-study comparison</summary><p>${esc(v.gate.book)}</p><p>${esc(v.gate.note)}</p><p>Original book-study reproduction remains unverified; these figures are separate from this computed backtest.</p></details>`:''}</div>`;
  if (!g) return '';
  return clamp ? `<div class="rnote gate un">${clamp}</div>` : ''; // the unverified-gate notice was removed (user request 22 Oct 2026)
}
function calculatedPeriodHTML(d) {
  return Array.isArray(d?.dates)&&d.dates.length ? `<span data-calculated-period><b>Calculated period: ${esc(d.dates[0])} to ${esc(d.dates.at(-1))}.</b> </span>` : '';
}
function experimentNotice(v,d) { return v.guru==='chande'?`<div class="rnote" data-chande-experiment><b>OHLC market adaptation · original futures study not reproduced.</b><p>${esc(d?.experiment?.basis||window.ChandeEvidence.basis)}</p></div>`:''; }
function sourceBlockers(v) {
  const blocks=VaultResearch.blockers(sourceEvidence(v));
  return blocks.length?`<div data-source-blockers>${blocks.map(b=>`<div data-blocker="${b.status}"><h4>${esc(b.label)}</h4><ul>${b.items.map(t=>`<li>${esc(t)}</li>`).join('')}</ul></div>`).join('')}</div>`:'';
}
function sourceResearchAuthors() {
  return `<section class="box" data-source-research><div class="box-h">Source research <small>Book references awaiting complete rules and market data</small></div><p>These sources are being reviewed. They have no implemented backtests and are excluded from performance rankings.</p><a class="research-author" href="${guruLink('ross')}"><div class="cover">${pframe(GURU.ross)}<div><h3>Joe Ross</h3><p>20 source methods · 40 directional references · 0 implemented backtests</p><p><b>Data missing · Rules unresolved</b></p><span class="dbtn sm">Open source review</span></div></div></a></section>`;
}
function familyDisplayName(v){return catalogueMethodLabel(v,ALLCAT);}
function gptSection(v,tk,d=null,issue=null) {
  const e=sourceEvidence(v),recorded=kestnerSourceAssessment(v,d,issue,e),detailed=R.path.startsWith('/o/');
  if(recorded){
    const assessment=`<div class="tlog"><table class="tbl gpt-evidence"><thead><tr><th>Research check</th><th>Evidence / finding</th><th>Status</th></tr></thead><tbody>${recorded.rows.map(r=>`<tr data-review-record="${esc(r.key)}"><td>${esc(r.label)}</td><td>${r.values.map(t=>`<div>${esc(t)}</div>`).join('')}</td><td>${esc(r.status)}</td></tr>`).join('')}</tbody></table></div>`;
    const tools=detailed&&!issue?`<h4 class="cc-h">Separate grid calculation</h4><div id="gpt-body">${bookLoader('Preparing a separate comparison from the requested Rust grid…')}</div>`:!issue?`<p>Parameter research is separate from the recorded source-rule review. A supported grid calculation is not a saved authored conclusion or an untouched holdout study.</p><a class="dbtn sm" href="${href('/o/'+encodeURIComponent(v.id),{...S})}">Open parameter research</a>`:'';
    return `<section class="box" id="gpt-c" data-source-audit data-recorded-source-review><div class="box-h">${esc(recorded.title)} <small>${esc(recorded.subtitle)}</small></div><p><b>${esc(recorded.verdict)}</b></p><p class="rnote">${esc(LABEL[tk]||tk)} · ${esc(v.name)}. ${esc(recorded.note)}</p><h4 class="cc-h">Recorded source scope and saved conclusion</h4>${assessment}${tools}</section>`;
  }
  const review=VaultResearch.review(v,d,issue,e);
  const assessment=`<div class="tlog"><table class="tbl gpt-evidence"><thead><tr><th>Research check</th><th>Evidence / finding</th><th>Status</th></tr></thead><tbody>${review.assessment.map(r=>`<tr><td>${esc(r.label)}</td><td>${r.values.map(t=>`<div>${esc(t)}</div>`).join('')}</td><td>${esc(r.status)}</td></tr>`).join('')}</tbody></table></div>`;
  return `<section class="box" id="gpt-c" data-source-audit><div class="box-h">GPT’s conclusion <small>Independent analyst assessment · five-percentile method from prompt.md</small></div><p><b>${esc(review.verdict)}</b></p><p class="rnote">${esc(btstFuturesSymbol(tk) && btstReference(v.id) ? btstFuturesSymbol(tk) : LABEL[tk])} · ${esc(v.name)}. Calculated from Rust results and reviewed source evidence; opening this page does not call an AI model.</p>${detailed&&!issue?`<div id="gpt-body">${bookLoader('Preparing the analyst comparison from the same Rust runs…')}</div>`:!issue?`<p>Book and top-three statistics, percentile groups, analyst tradeoffs, detailed charts and a formula workbook are available in Optimization.</p><a class="dbtn sm" href="${href('/o/'+encodeURIComponent(v.id),{...S})}">Open GPT parameter research</a>`:''}<h4 class="cc-h">Source and data assessment</h4>${assessment}<details class="gpt-source-detail"><summary>Detailed source findings</summary>${review.evidence.map(t=>`<p>${esc(t)}</p>`).join('')}<ul>${review.caveats.map(t=>`<li>${esc(t)}</li>`).join('')}</ul>${e?`<p class="rnote">${esc(e.source.path)} · ${esc(e.pages)}. Reviewed core sources; full corpus review remains in progress.</p>`:''}</details>${review.links.length?`<p class="rnote">External evidence: ${review.links.map((url,i)=>`<a href="${esc(url)}" target="_blank" rel="noopener noreferrer">Source ${i+1}</a>`).join(' · ')}</p>`:''}</section>`;
}


// ---------------------------------------------------------------- state (kept in the URL hash so any view can be shared)
const DEF = { mode: 'strategies', market: 'india', tk: 'NIFTY 50', tab: 'sharpe', q: '', guru: '', book: '', style: '', dir: '', hold: '', verdict: '', min: '0', family: '', grp: '', sort: 'primary', view: 'grid' };
let S = { ...DEF };
const readHash = () => { const q = new URLSearchParams(location.hash.slice(1)); S = { ...DEF }; for (const k in DEF) if (q.has(k)) S[k] = q.get(k); };
const writeHash = () => { const q = new URLSearchParams(); for (const k in DEF) if (S[k] !== DEF[k]) q.set(k, S[k]); history.replaceState(null, '', '#' + q); };

let CAT = [], TICKERS = [], GURUS_API = [], TICK_ALL_US = [];
const TRUST = window.VaultTruth;
// Plain-language headline for a withheld run; falls back to the status label. Wording only.
const heldLabel = issue => ['needs_data','not_testable'].includes(issue.status) && reasonLabel(issue.note) || TRUST.statusLabel(issue.status);
function summaryContext(tk,from='',to='') {
 const ticker=[...TICK_ALL_IN,...TICK_ALL_US].find(item=>item.ticker===tk);
 return {ticker:tk,label:LABEL[tk]||ticker?.symbol||tk,from,to,first:ticker?.first,last:ticker?.last};
}
const summaryKey=(api,tk,from='',to='')=>`${api}|${tk}|${from}|${to}`+(guruIds().includes('pristine')?`|pristine:${S.research_mode||''}:${S.cost??'0'}:${S.slippage??'0'}:${S.rollover??'0'}:${S.p||''}`:'');
const SUMMARY_ISSUES={},READINESS={},SUM={},SUM_FAILURE_GENERATION={},READINESS_FAILURE_GENERATION={};
const ownedDefinitions=(variants,owner,id='')=>variants.filter(v=>(v.api||ownerOf(v.id))===owner&&(!id||v.id===id));
const summaryIssues=(tk,from='',to='')=>SUMMARY_ISSUES[summaryKey(API,tk,from,to)]||[];
function ownerQuery(owner,query){
 if(owner!=='pristine')return query;
 const selected=new URLSearchParams(query);selected.set('research_mode',S.research_mode||'');selected.set('cost',S.cost??'0');selected.set('slippage',S.slippage??'0');selected.set('rollover',S.rollover??'0');return selected;
}
// A distinct owner has a distinct inclusive-session protocol. Do not weaken
// the shared legacy loader's date/window semantics to admit this model.
async function pristineOwnerRows(url,definitions,kind){
 try{
  if(S.p)throw Error('These fixed Pristine branches accept no parameter settings.');
  const response=await fetch(url);if(!response.ok)throw Error(await response.text());
  const payload=await response.json(),rows=kind==='readiness'?payload?.strategies:payload,selected=new URL(url,location.href).searchParams;
  if(!Array.isArray(rows)||rows.length!==definitions.length)throw Error('Pristine owner response shape is unavailable.');
  const expected=new Set(definitions.map(definition=>definition.id)),seen=new Set();
  for(const row of rows){
   if(!row||!expected.has(row.id)||seen.has(row.id))throw Error('Pristine owner response identity is unavailable.');seen.add(row.id);
   if(kind==='summary'&&!row.blocked){const model=row.pristine_summary;if(!model||model.id!==row.id||model.book!==PRISTINE_BOOK||model.owner!=='pristine'||model.model!=='pristine_daily_supplied_series_summary_v1'||model.research_mode!==selected.get('research_mode')||model.qualified_ticker!==selected.get('ticker')||model.market!=='us'||model.asset_type!=='individual_stock'||model.book_performance_reproduced!==false||model.native_historical_dollars_verified!==false||model.historical_membership_verified!==false)throw Error('Pristine summary model does not match current research scope.');}
  }
  return {rows,retryableFailure:false};
 }catch(error){return {rows:definitions.map(definition=>({id:definition.id,...(kind==='readiness'?{status:'unavailable',note:error.message}:{blocked:error.message,readiness:'unavailable'})})),retryableFailure:true,issue:{owner:'pristine',status:'unavailable',kind:'request_failed',note:error.message||'Pristine owner data is unavailable.'}};}
}

async function getReadiness(tk, from = '', to = '', id = '', parameters = {}, api = API) {
 const owners=id?[ownerOf(id)]:guruIds(),variants=[...ALLCAT],generation=busy;
 if(!owners.length)return {};
 const query=new URLSearchParams({ticker:tk,from,to,cost:S.cost||'0',...VaultUI.costState(S),...(id?{id}:{})});
 const settings=typeof parameters==='string'?decodeP(parameters):parameters;
 for(const [key,value] of Object.entries(settings))query.set('p.'+key,value);
 const key=api+'|'+(id?owners[0]:'all')+'?'+query+(owners.includes('pristine')?'|research_mode='+S.research_mode:''),context=summaryContext(tk,from,to);
 // Failed requests are shared within a render, then retried on the next user navigation.
 if(READINESS_FAILURE_GENERATION[key]!=null&&READINESS_FAILURE_GENERATION[key]!==generation)delete READINESS[key];
 return READINESS[key]||(READINESS[key]=(async()=>{
  const outcomes=await Promise.all(owners.map(owner=>owner==='pristine'?pristineOwnerRows(api+'/guru/pristine/readiness?'+ownerQuery(owner,query),ownedDefinitions(variants,owner,id),'readiness'):loadOwnerRows({owner,definitions:ownedDefinitions(variants,owner,id),context,url:api+'/guru/'+owner+'/readiness?'+query,kind:'readiness',fetcher:fetch,wait:ms=>new Promise(resolve=>setTimeout(resolve,ms))})));
  if(outcomes.some(outcome=>outcome.issue))READINESS_FAILURE_GENERATION[key]=generation;else delete READINESS_FAILURE_GENERATION[key];
  return Object.fromEntries(outcomes.flatMap(outcome=>outcome.rows).map(row=>[row.id,row]));
 })());
}
async function getSum(tk, from = '', to = '', api = API) {
 const owners=guruIds(),variants=[...ALLCAT],generation=busy;
 if(!owners.length)return {};
 const key=summaryKey(api,tk,from,to),context=summaryContext(tk,from,to),query=new URLSearchParams({ticker:tk,from,to});
 if(SUM_FAILURE_GENERATION[key]!=null&&SUM_FAILURE_GENERATION[key]!==generation)delete SUM[key];
 return SUM[key]||(SUM[key]=(async()=>{
  const outcomes=await Promise.all(owners.map(owner=>owner==='pristine'?pristineOwnerRows(api+'/guru/pristine/summary?'+ownerQuery(owner,query),ownedDefinitions(variants,owner),'summary'):loadOwnerRows({owner,definitions:ownedDefinitions(variants,owner),context,url:api+'/guru/'+owner+'/summary?'+query,kind:'summary',fetcher:fetch,wait:ms=>new Promise(resolve=>setTimeout(resolve,ms))})));
  SUMMARY_ISSUES[key]=outcomes.flatMap(outcome=>outcome.issue?[outcome.issue]:[]);
  if(outcomes.some(outcome=>outcome.issue))SUM_FAILURE_GENERATION[key]=generation;else delete SUM_FAILURE_GENERATION[key];
  const rows=Object.fromEntries(outcomes.flatMap(outcome=>outcome.rows).map(row=>[row.id,row]));
  await btstCardRows(rows,tk,from,to,api);
  return rows;
 })());
}
// BTST cards show the run their page opens with: the reference sheet's settings and the team's 0.03% brokerage
// (the summary endpoint runs the engine defaults with no cost, which would not match the page).
async function btstCardRows(rows,tk,from,to,api){
 const ids=Object.keys(rows).filter(id=>rows[id]?.stats&&referenceDefaults(id,tk));let i=0;
 const sample=(eq,n=121)=>!eq?.length?null:Array.from({length:Math.min(n,eq.length)},(_,k)=>eq[Math.round(k*(eq.length-1)/Math.max(1,Math.min(n,eq.length)-1))]);
 const worker=async()=>{while(i<ids.length){const id=ids[i++],ref=referenceDefaults(id,tk);
  try{const d=await runData(id,tk,from,to,'0',encodeP(ref.params),api,{slippage:String(ref.cost),rollover:'0'});
   if(d&&!d.error&&d.stats){const sp=sample(d.equity);rows[id]={...rows[id],stats:d.stats,...(sp?{spark:sp}:{}),costs:d.costs||rows[id].costs,btst_reference:true};}}catch(e){/* keep the summary row */}}};
 await Promise.all(Array.from({length:4},worker));
}
function summaryNoticeHTML(issues) {
 if(!issues.length)return '';
 const notes=[...new Set(issues.map(issue=>issue.note))],dates=issues.some(issue=>['empty_range','invalid_range'].includes(issue.kind));
 return `<section class="summary-unavailable" data-summary-unavailable role="status"><strong>Data coverage</strong>${notes.map(note=>`<p>${esc(note)}</p>`).join('')}${dates?`<a class="dbtn sm" data-summary-reset-dates href="${href(R.path,{...S,from:'',to:'',tab:'sharpe'})}">Use available history</a>`:''}</section>`;
}
function listSummaryIssues() {
 if(S.tab==='awaiting')return [];
 const tab=TABS[S.tab],tickers=S.tk==='ALL'?TICKERS.map(item=>item.ticker):[S.tk];
 return tickers.flatMap(tk=>{const window=S.tab==='sharpe'&&(S.from||S.to)?{from:S.from||'',to:S.to||''}:tab.win(tk);return summaryIssues(tk,window.from,window.to);});
}
// "before the book" numbers come from the per-strategy endpoint: the summary endpoint crashes when the end date
// is before the first India VIX bar (VIX strategies), while /run answers that case cleanly.
const PRE = {};
function preMap(tk, to, api = API, cat = CAT) {
  const k = `${api}|${tk}|${to}`;
  if (PRE[k]) return PRE[k];
  const ids = cat.filter(v => v.status === 'live' && !v.needs_vix).map(v => v.id), out = {}; // VIX data starts after 2009: nothing to ask
  let i = 0;
  const worker = async () => { while (i < ids.length) { const id = ids[i++];
    try { const r = await fetch(enginePath(id,'run',api)+'?' + new URLSearchParams({ id, ticker: tk, to })); if (r.ok) out[id] = { stats: (await r.json()).stats }; } catch (e) { /* leave missing */ } } };
  return PRE[k] = Promise.all(Array.from({ length: 6 }, worker)).then(() => out);
}
const lastOf = tk => [...TICKERS, ...TICK_IN, ...(TICK_US || [])].find(t => t.ticker === tk)?.last || '2026-05-15';
const shiftYears = (d, y) => { const t = new Date(d); t.setFullYear(t.getFullYear() - y); return t.toISOString().slice(0, 10); };

// tabs: which data window each one uses and which number it ranks by
let PUB = 2010;
const TABS = {
  trending: { name: 'Trending', note: 'Ranked by the return of the last 12 months on the selected index: what is working right now.',
    win: tk => ({ from: shiftYears(lastOf(tk), 1), to: '' }), extra: () => ({ from: '', to: '' }),
    metric: { label: '12M return', get: r => r.s.total_return, fmt: v => pct(v, 1, true), bins: [-.1, -.05, 0, .05, .1, .2] }, keep: r => r.s.trades > 0 },
  sharpe: { name: 'Top Sharpe', note: 'Full history on the selected index, ranked by Sharpe ratio (rf = 0, daily, √252).',
    win: () => ({ from: '', to: '' }), metric: { label: 'Sharpe', get: r => r.s.sharpe, fmt: v => fx(v), bins: [0, .25, .5, .75, 1, 1.5] } },
  working: { name: 'Still working?', note: `Out-of-sample check: Sharpe since the book was published (${PUB}) next to Sharpe before it. A strategy that only worked before publication has decayed.`,
    win: () => ({ from: `${PUB}-01-01`, to: '' }), pre: `${PUB - 1}-12-31`,
    metric: { label: `Sharpe since ${PUB}`, get: r => r.s.sharpe, fmt: v => fx(v), bins: [0, .25, .5, .75, 1, 1.5] } },
  sig: { name: 'Significant', note: 'Only rule sets whose average trade is statistically significant: t-stat ≥ 2 with at least 30 closed trades.',
    win: () => ({ from: '', to: '' }), metric: { label: 't-stat', get: r => r.s.t_stat, fmt: v => fx(v), bins: [2, 2.5, 3, 4, 5, 6] }, keep: r => r.s.t_stat >= 2 && r.s.trades >= 30 },
  awaiting: { name: 'Unavailable backtests', note: 'Book rules with missing market data, incomplete calculation support or insufficient trading specifications. Coding a rule does not establish a verified backtest. Open a strategy to see the reason for this market and period.' },
};
const SORTS = { primary: 'Tab ranking', sharpe: 'Sharpe', cagr: 'CAGR', maxdd: 'Smallest drawdown', win: 'Win rate', pf: 'Profit factor', trades: 'Most trades', t: 't-stat', name: 'Name (A–Z)', course: 'Order in the book' };
const sortVal = (r, k) => ({ primary: TABS[S.tab].metric?.get(r), sharpe: r.s.sharpe, cagr: r.s.cagr, maxdd: r.s.max_dd, win: r.s.win_rate, pf: r.s.profit_factor, trades: r.s.trades, t: r.s.t_stat }[k]);
function sortRows(rows) {
  const k = S.sort;
  if (k === 'name') return rows.sort((a, b) => a.v.family_name.localeCompare(b.v.family_name));
  if (k === 'course') return rows.sort((a, b) => a.v.ord - b.v.ord);
  // Rank admitted modeled outcomes by the selected metric; source comparison status does not raise rank.
  const ver = r => 0;
  return rows.sort((a, b) => (ver(b) - ver(a)) || ((sortVal(b, k) ?? -1e9) - (sortVal(a, k) ?? -1e9)));
}
const verdictOf = s => s.trades < 30 ? { k: 'thin', t: 'THIN DATA', c: '#8a3a2e' } : s.t_stat >= 2 ? { k: 'sig', t: 'SIGNIFICANT', c: cv('--green') } : { k: 'none', t: 'NO EDGE', c: '#6b6f78' };
const holdOf = s => s.avg_bars <= 5 ? 'short' : s.avg_bars <= 20 ? 'swing' : 'long';
const HOLD = { short: '≤ 5 bars', swing: '6–20 bars', long: '> 20 bars' };

// ---------------------------------------------------------------- rows: one per (variant × index) after market, tab and filters
async function rows(S = getS()) {
  if (S.market !== 'india') return [];
  if (S.region === 'all' && TICK_US) {
    const markets = [['in', API_IN, TICK_IN], ['us', API_US, TICK_US]];
    const parts = await Promise.all(markets.map(([region, api, ticks]) => marketRows({ ...S, tk: 'ALL', region }, region, api, ticks, libraryMarketVariants(ALLCAT, libraryMarket({ ...S, region }, R.name)))));
    return S.tab === 'awaiting' ? parts.flat() : sortRows(parts.flat());
  }
  return marketRows(S, S.region, API, TICKERS, CAT);
}
async function marketRows(S, region, api, tickers, CAT) {
  const T = TABS[S.tab], tks = S.tk === 'ALL' ? tickers.map(t => t.ticker) : [S.tk];
  const qq = S.q.trim().toLowerCase();
  const textOk = v => !qq || `${v.name} ${v.family_name} ${v.variant || ''} ${v.group} ${v.dir} ${BOOK[v.book]?.title || ''} ${GURU[v.guru]?.name || ''} ${(v.rules || []).join(' ')}`.toLowerCase().includes(qq);
  const metaOk = v => (R.name === 'yt') === isYt(v) && textOk(v) && (!S.guru || v.guru === S.guru) && (!S.book || v.book === S.book) && (!S.style || v.style === S.style) && (!S.dir || v.dir === S.dir) && (!S.family || v.family === S.family) && (!S.grp || ytMainVariant(v.group) === S.grp);
  if (S.tab === 'awaiting') {
    const availability = await Promise.all(tks.map(tk => getReadiness(tk,S.from,S.to,'',{},api)));
    return CAT.filter(metaOk).map(v => {
      const issues=tks.map((tk,i)=>({tk,...availability[i][v.id]})).filter(item=>item.status!=='ready');
      return {v,tk:issues[0]?.tk,s:null,issues,region};
    }).filter(row=>row.issues.length);
  }
  const out = [];
  for (const tk of tks) {
    const w = S.tab==='sharpe'&&(S.from||S.to)?{from:S.from||'',to:S.to||''}:T.win(tk), main = await getSum(tk, w.from, w.to, api), ex = T.extra ? T.extra(tk) : null, extra = ex ? await getSum(tk, ex.from, ex.to, api) : T.pre ? await preMap(tk, T.pre, api, CAT) : null;
    for (const v of CAT) {
      if (v.status !== 'live' || !metaOk(v)) continue;
      const m = main[v.id]; if (!m?.stats || !m.stats.trades) continue; // no data (volume rules on Nifty IT) or no closed trades in the window
      const r = { v, tk, s: m.stats, sp: m.spark, x: extra?.[v.id]?.stats || null, w, region };
      if (T.keep && !T.keep(r)) continue;
      if (S.hold && holdOf(r.s) !== S.hold) continue;
      if (S.verdict && verdictOf(r.s).k !== S.verdict) continue;
      if (r.s.trades < +S.min) continue;
      out.push(r);
    }
  }
  return sortRows(out);
}

// ---------------------------------------------------------------- small charts
function spark(r, w = 300, h = 86) {
  const a = (r.sp || []).map(v => Math.max(v, 1e-4)); if (a.length < 2) return ''; // equity <= 0 (a total loss) sits at the log-scale floor
  const L = Math.log, lo = Math.min(...a.map(L)), hi = Math.max(...a.map(L)), n = a.length;
  const X = i => i / (n - 1) * w, Y = v => h - 4 - (L(v) - lo) / ((hi - lo) || 1) * (h - 10);
  const P = a.map((v, i) => `${i ? 'L' : 'M'}${X(i).toFixed(1)},${Y(v).toFixed(1)}`).join('');
  const cov = isYt(r.v) && btstReference(r.v.id) ? coverageWindow(r.v, r.tk) : null; // BTST: the futures data's own period
  const end = new Date(r.w.to || cov?.to || lastOf(r.tk)), start = new Date(end - r.s.years * 365.25 * 864e5);
  const f = (new Date(`${PUB}-01-01`) - start) / (end - start);
  const mark = f > .03 && f < .97 ? `<rect x="${f * w}" y="0" width="${(1 - f) * w}" height="${h}" fill="var(--blue-soft)" opacity=".55"/><line x1="${f * w}" x2="${f * w}" y1="0" y2="${h}" stroke="var(--chart)" stroke-dasharray="3 3" stroke-width="1"/>` : '';
  const small = w < 200; // table rows: no axis labels, the dashed line still marks the book
  return `<svg viewBox="0 0 ${w} ${h + (small ? 0 : 14)}" width="100%" class="strip" role="img" aria-label="Equity curve${mark ? `; shaded part is after the book (${PUB})` : ''}">${mark}<path d="${P}L${w},${h}L0,${h}Z" fill="var(--chart)" opacity=".08"/><path d="${P}" fill="none" stroke="var(--chart)" stroke-width="1.6"/>
    ${small ? '' : `<text x="0" y="${h + 12}">${start.getFullYear()}</text>${mark ? `<text x="${f * w}" y="${h + 12}" text-anchor="middle" fill="var(--chart)">book ${PUB}</text>` : ''}<text x="${w}" y="${h + 12}" text-anchor="end">${end.getFullYear()}</text>`}</svg>`;
}
// strip plot: every strategy of a guru/book as a dot on the tab's metric axis
function stripPlot(vals, w = 300, h = 34) {
  const M = TABS[S.tab].metric; if (!vals.length || !M) return '';
  const b = M.bins, lo = Math.min(b[0], ...vals), hi = Math.max(b[b.length - 1], ...vals), X = v => 6 + (v - lo) / ((hi - lo) || 1) * (w - 12);
  return `<svg viewBox="0 0 ${w} ${h + 12}" width="100%" class="strip" role="img" aria-label="Spread of ${esc(M.label)}"><line x1="6" x2="${w - 6}" y1="${h / 2}" y2="${h / 2}" stroke="var(--rule)"/>
    ${vals.map((v, i) => `<circle cx="${X(v).toFixed(1)}" cy="${h / 2 + ((i % 3) - 1) * 6}" r="4" fill="var(--chart)" opacity=".55"/>`).join('')}
    <line x1="${X(med(vals))}" x2="${X(med(vals))}" y1="3" y2="${h - 3}" stroke="var(--ink)" stroke-width="1.5"/>
    <text x="6" y="${h + 10}">${M.fmt(lo)}</text><text x="${w - 6}" y="${h + 10}" text-anchor="end">${M.fmt(hi)}</text></svg><div class="stripcap"><b>median ${M.fmt(med(vals))}</b> · each dot is one strategy</div>`;
}
function distBox(rs) {
  const M = TABS[S.tab].metric;
  if (!M || !rs.length) return `<div class="h"><b>Where this view sits</b><span>${rs.length} RESULTS</span></div><div class="muted" style="font-size:13px;padding:20px 0">${S.tab === 'awaiting' ? 'Results are withheld until the listed data and rule requirements are resolved.' : 'Nothing to chart for this selection.'}</div>`;
  const b = M.bins, edges = [-Infinity, ...b, Infinity], vals = rs.map(M.get), cnt = Array(edges.length - 1).fill(0);
  vals.forEach(v => { for (let i = 0; i < cnt.length; i++) if (v >= edges[i] && v < edges[i + 1]) { cnt[i]++; break; } });
  const mx = Math.max(...cnt), W = 420, H = 86, bw = W / cnt.length, md = med(vals);
  const lbl = i => i === 0 ? `<${M.fmt(b[0])}` : i === cnt.length - 1 ? `≥${M.fmt(b[b.length - 1])}` : `${M.fmt(edges[i])}–${M.fmt(edges[i + 1])}`;
  const mi = cnt.findIndex((_, i) => md >= edges[i] && md < edges[i + 1]);
  const top = rs.slice(0, 12).map(M.get);
  return `<div class="h"><b>Where this view sits</b><span>${rs.length} RESULTS · ${S.tk === 'ALL' ? 'ALL INDICES' : esc(LABEL[S.tk]).toUpperCase()}</span></div>
    <svg viewBox="0 0 ${W} ${H + 18}" width="100%" class="strip" role="img" aria-label="Distribution of ${esc(M.label)}">${cnt.map((c, i) => { const hh = c / (mx || 1) * (H - 16); const inTop = top.some(v => v >= edges[i] && v < edges[i + 1]);
      return `<rect x="${i * bw + 2}" y="${H - hh}" width="${bw - 4}" height="${Math.max(1, hh)}" fill="${inTop ? 'var(--blue)' : '#dfe6ef'}"><title>${lbl(i)}: ${c}</title></rect><text x="${i * bw + bw / 2}" y="${H + 14}" text-anchor="middle">${lbl(i)}</text>${c ? `<text x="${i * bw + bw / 2}" y="${H - hh - 3}" text-anchor="middle" fill="var(--ink)">${c}</text>` : ''}`; }).join('')}
      ${mi >= 0 ? `<line x1="${mi * bw + bw / 2}" x2="${mi * bw + bw / 2}" y1="0" y2="${H}" stroke="var(--ink)" stroke-dasharray="2 2"/>` : ''}</svg>
    <div class="foot"><span>Dashed line = median ${M.fmt(md)} · top 12 shown first: ${M.label} ${M.fmt(Math.min(...top))} to ${M.fmt(Math.max(...top))}</span><span>dark = where they sit</span></div>`;
}

// ---------------------------------------------------------------- shared pieces
const appLink = r => stratLink(r.v.id, r.tk, r.region); // opens the single-strategy page inside this page
// ADR results page: the AmiBroker-style report (report-stats.js / report-view.js), computed once per run
// the 4 ATH variants the research team verifies get the same Results as ADR: Report, report and YOY Excel, lab-style optimizer with the optimization sheet
const ATH_RESULT_IDS=['ytath_chk_pctswing_turnover_running_buy','ytath_chk_all_st_variations_all_ranking','ytath_chk_19pct_swing_sl_probability','ytath_plus_pct_buy_pctswing_exit_nse500_2'];
const isAth4=id=>ATH_RESULT_IDS.includes(id??STRAT?.v?.id??'');
const isAdr=()=>/^ytadr_/.test(STRAT?.v?.id||'')||isAth4();
/** a stock-universe run as the report maths want it: one entry per trade with its bar numbers, the engine's own money (shares, pnl, weight) and the benchmark filled across gaps */
function athRunOf(d){
 const at=new Map(d.dates.map((x,i)=>[String(x).slice(0,10),i]));let last=1;
 let bh=(d.bh||[]).map(x=>Number.isFinite(x)?(last=x):last);
 const px=STRAT?.benchPx;
 if(d.kind==='universe'&&'benchmark_label' in d&&d.benchmark_label==null)bh=null;   // the basket has no index (F&O): no Buy&Hold
 else if(!('benchmark_label' in d)&&!(d.bh||[]).some(x=>Number.isFinite(x)&&x!==1)&&px?.d&&px.c){      // an older run without its benchmark: NIFTY 50 closes at or before each date, from the first date
  const pd=px.d.map(x=>String(x).slice(0,10));let j=0,base=null;
  bh=d.dates.map(x=>{const k=String(x).slice(0,10);while(j+1<pd.length&&pd[j+1]<=k)j++;if(pd[j]>k)return base??1;if(base==null)base=px.c[j];return px.c[j]/base;});
 }
 const trades=d.trades.filter(t=>at.has(String(t.entry).slice(0,10))).map(t=>({legs:[[at.get(String(t.entry).slice(0,10))]],exit_i:at.get(String(t.exit).slice(0,10))??d.dates.length-1,ret:t.ret,dir:1,entry:t.entry,exit:t.exit,avg_px:t.entry_px,exit_px:t.exit_px,open:!!t.open,reason:t.reason,symbol:t.symbol,pnl:t.pnl,shares:t.shares,weight:t.weight}));
 return {dates:d.dates,close:null,equity:d.equity,bh,trades,portfolio:true};
}
const ADR_REPORTS=new WeakMap();
function adrReport(d){
 if(!isAdr()||!d?.equity?.length||!d.dates||!d.bh||!Array.isArray(d.trades))return null;
 if(ADR_REPORTS.has(d))return ADR_REPORTS.get(d);
 const side=((Number(d.costs?.fees_bps)||0)+(Number(d.costs?.slippage_bps)||0))/10000;
 let rep;
 if(isAth4())rep=buildReport(athRunOf(d),null,{capital:d.model_initial_equity||1000000,side});
 else{if(!d.close)return null;rep=buildReport({dates:d.dates,close:d.close,equity:d.equity,bh:d.bh,trades:d.trades},STRAT.px,{side});}
 ADR_REPORTS.set(d,rep);return rep;
}
// the Buy&Hold index of a stock-universe run follows its basket (engine `benchmark_label`: NSE 500, NIFTY 50, or null = no index); older runs: NIFTY 50
const uniBench=d=>d&&'benchmark_label' in d?d.benchmark_label:'NIFTY 50';
const benchLabel=d=>isAth4()?(uniBench(d)??'no index'):STRAT.tk;
const adrReportHTML=d=>{const r=adrReport(d);return r?reportHTML(r,benchLabel(d)):'';};
const RTAG = { F: 'IF', E: 'ENTRY', A: 'ADD', X: 'EXIT' };
// rule lines [section, text]; YT templates filled with the schema defaults plus `overrides` (rule-template.ts)
const ruleLines = (v, overrides = {}) => { const sch = OPT.schemaSync?.[v.id]?.length ? OPT.schemaSync[v.id] : isYt(v) && universeRunnable(uniOf(v.id), v.id) ? universeSchema(uniOf(v.id), v.id, OPT.schemaSync?.ytbtst_core || []) : OPT.schemaSync?.[v.id], vals = isYt(v) ? paramValues(sch, overrides) : null; return (v.rules || []).map(x => { const [t, ...rest] = x.split('|'), raw = rest.join('|') || x; return [t, vals ? renderRule(raw, vals) : raw]; }).filter(([, r]) => r !== null); };
const rulesList = v => `<details class="rules"><summary>${isYt(v) ? 'Rules as specified' : 'Rules as written in the book'}</summary><ul>${ruleLines(v).map(([t, r]) => `<li><span class="t">${RTAG[t] || ''}</span>${esc(r)}</li>`).join('')}</ul></details>`;
// YT cards (BTST, ATH): Net return (equity, B&H below) · Max DD (CAGR below) · Win (trades below)
const ytKpis = (s, note = '') => `<div class="kpis yt-kpis"><div><span>Net return</span><b class="${cls(s.total_return ?? s.net_profit)}">${pct(s.total_return ?? s.net_profit, 2, true)}</b><i>${s.bh_total != null ? 'B&amp;H ' + pct(s.bh_total, 2, true) : esc(note)}</i></div><div><span>Max DD</span><b class="neg">${pct(s.max_dd, 2)}</b><i>CAGR ${pct(s.cagr, 2, true)}</i></div><div><span>Win</span><b class="p">${pct(s.win_rate, 2)}</b><i>${fx(s.trades, 0)} trades</i></div></div>`;
const kpiTriplet = r => {
  if (isYt(r.v)) return ytKpis(r.s);
  const M = TABS[S.tab].metric, s = r.s;
  const second = S.tab === 'working' ? `<div><span>Before ${PUB}</span><b>${fx(r.x?.sharpe)}</b><i>${r.x ? (r.x.sharpe > 0 && s.sharpe > 0 ? 'still working' : s.sharpe > 0 ? 'works since book' : r.x.sharpe > 0 ? 'decayed' : 'never worked') : 'Before-book result unavailable'}</i></div>`
    : S.tab === 'trending' ? `<div><span>Sharpe (full)</span><b>${fx(r.x?.sharpe)}</b><i>${fx(r.s.trades, 0)} trades in 12M</i></div>`
    : `<div><span>CAGR</span><b class="${cls(s.cagr)}">${pct(s.cagr, 1, true)}</b><i>B&amp;H ${pct(s.bh_cagr, 1, true)}</i></div>`;
  return `<div class="kpis"><div><span>${esc(M.label)}</span><b class="p">${M.fmt(M.get(r))}</b><i>${S.tab === 'sig' ? 'Sharpe ' + fx(s.sharpe) : 'win ' + pct(s.win_rate, 0)}</i></div>${second}<div><span>Max DD</span><b class="neg">${pct(s.max_dd, 1)}</b><i>${fx(s.trades, 0)} trades</i></div></div>`;
};
const byline = v => isYt(v) ? 'YT Strategies' : `${esc(GURU[v.guru].name)} · ${esc(BOOK[v.book].title)} (${bookYear(BOOK[v.book])}) · ${srcOf(v)}`;

// ---------------------------------------------------------------- MODES
function unavailableBookCard(method){
  const v=method.definition,record=currentLibraryProgress().measured.find(row=>row.method_id===method.key),labels={needs_stock_basket:'Select a stock',needs_data:'Required data missing',source_review:'Not backtested · see reason',no_trades:'No closed trades'},stamp=['needs_data','source_review'].includes(method.status)&&reasonLabel(method.reasons[0])||labels[method.status]||'Backtest unavailable';
  const alternative=record?href('/s/'+encodeURIComponent(record.variant_id),{region:record.market==='us'?'us':'in',tk:record.instrument,basket:record.instrument?.startsWith('basket:')?record.instrument.split(':')[1]:'',...measuredWindow(record),cost:record.execution?.cost||'0',slippage:record.execution?.slippage||'0',rollover:record.execution?.rollover||'0',research_mode:record.execution?.research_mode||'',run_scope:'single'}):'';
  return `<article class="card unavailable-method" data-unavailable-method="${esc(method.key)}"><div class="rank"><span>Unavailable for ${esc(LABEL[S.tk]||S.tk)}</span><span class="stamp">${esc(stamp)}</span></div><h3>${esc(method.name)}</h3><div class="by">${byline(v)}</div><div class="unavailable-reasons">${method.reasons.map(note=>`<p>${esc(note)}</p>`).join('')}</div><p class="book-progress-reason">${method.status==='no_trades'?'This run has no closed positions. Open positions or signals may still exist.':'No performance chart or statistics are shown for this selection.'}</p>${rulesList(v)}<div class="btns"><a class="btn" href="${stratLink(v.id,S.tk)}">View rules &amp; requirements</a>${alternative?`<a class="btn" href="${alternative}">Available on ${esc(LABEL[record.instrument]||record.instrument.split(':').at(-1))}</a>`:''}</div></article>`;
}
async function unavailableForSelectedBook(){
  if(!S.book||R.name!=='guru')return [];
  const defs=ALLCAT.filter(v=>v.book===S.book),tickers=S.tk==='ALL'?TICKERS.map(t=>t.ticker):[S.tk],reasons=[];
  for(const tk of tickers){
    const w=S.tab==='sharpe'&&(S.from||S.to)?{from:S.from||'',to:S.to||''}:TABS[S.tab].win?.(tk)||{from:S.from||'',to:S.to||''};
    const [summary,ready]=await Promise.all([getSum(tk,w.from,w.to),getReadiness(tk,w.from,w.to)]);
    reasons.push(...VaultLibraryDiscovery.authorAvailability([S.book],defs,summary,ready).variantReasons);
  }
  // A ready result on any selected instrument admits the method; never let a later
  // blocked context overwrite it when browsing all loaded indices.
  const merged=new Map();for(const reason of reasons){const old=merged.get(reason.id);if(!old||reason.status==='available'||old.status!=='available'&&reason.status==='no_trades')merged.set(reason.id,reason);}
  const query=S.q.trim().toLowerCase(),matching=defs.filter(v=>(!query||`${v.name} ${v.family_name} ${v.variant||''} ${v.group} ${v.dir} ${BOOK[v.book]?.title||''} ${GURU[v.guru]?.name||''} ${(v.rules||[]).join(' ')}`.toLowerCase().includes(query))&&(!S.style||v.style===S.style)&&(!S.dir||v.dir===S.dir)&&(!S.family||v.family===S.family)&&(!S.grp||ytMainVariant(v.group)===S.grp));
  return unavailableBookMethods(matching,[...merged.values()]);
}
// YT families with main variants (e.g. BTST) open as their own page: family card -> main variants -> sub-variants.
const ytFamilyCard = (r, perVariant) => R.name === 'yt' && !perVariant && !S.family && ytMainVariants(ALLCAT, r.v.family).length > 0;
// green dot from outside: how many sub-variants in `ids` have a reference trade sheet (dot shown on the main-variant / family card)
const refSheetCount = ids => ids.filter(id => ytFamilyReference(id)?.tradesheets?.length).length;
const refGroupDot = (ids, what = 'sub-variant') => { const n = refSheetCount(ids); return n ? `<i class="ref-dot ok" role="img" aria-label="${n} of ${ids.length} ${what}s have a reference trade sheet" title="${n} of ${ids.length} ${what}s have a reference trade sheet"></i>` : ''; };
function ytGroupCards(mv) {
  const fam = S.family, name = familyDisplayName(ALLCAT.find(v => v.family === fam) || {});
  return `<div class="grid">${mv.map((m, i) => `<article class="card yt-main" data-yt-main="${esc(m.name)}"><div class="rank"><span>${refGroupDot(m.ids)}Main variant ${i + 1}</span><span class="stamp">${m.ids.length} sub-variant${m.ids.length === 1 ? '' : 's'}</span></div>
      <h3>${esc(m.name)}</h3><div class="by">${esc(name)} · YT Strategies</div><p class="yt-main-note">${esc(MAIN_VARIANT_NOTES[m.name] || '')}</p>
      <div class="btns"><a class="btn solid" href="${href('/yt', { tk: S.tk, from: S.from, to: S.to, family: fam, grp: m.name, mode: 'variants' })}">View sub-variants</a></div></article>`
  ).join('')}</div>`;
}
// Inside a YT main variant: the sub-variants that run only on the F&O stock universe (no index result to rank).
// ATH and BTST cards: green % = checked against the variant's own tradesheet; yellow % = checked against the nearest version's sheet; nothing = no similar version
const verifyBadge = (id, tk) => { const a = ATH_VERIFY[id] || ADR_VERIFY[id] || btstVerify(id, tk); if (!a) return ''; const own = a[0] === 'own';
  return `<span class="ath-ver ${own ? 'own' : 'near'}" title="${own ? 'Checked against its own tradesheet' : 'Checked against the nearest version\'s tradesheet'}: ${esc(a[2])}">${a[1].toFixed(1)}%</span>`; };
// green name: the sub-variants added from the team's "AFL ATH checking" folder
// ATH sub-variants whose page offers the All F&O basket (FO SCRIPTS C&P) next to NSE 500; the others stay on their reference list until this one is confirmed
const ATH_FO_CHOICE = ['ytath_plus_pct_buy_pctswing_exit_nse500_2'];
// ATH cards: the equity curve and headline numbers of one default run, calculated ahead (scripts/ath_card_results.py), drawn like the BTST cards
function athSpark(a, first, last, w = 300, h = 86) {
  const v = (a || []).map(x => Math.max(x, 1e-4)); if (v.length < 2) return '';
  const L = Math.log, lo = Math.min(...v.map(L)), hi = Math.max(...v.map(L)), n = v.length;
  const X = i => i / (n - 1) * w, Y = x => h - 4 - (L(x) - lo) / ((hi - lo) || 1) * (h - 10);
  const P = v.map((x, i) => `${i ? 'L' : 'M'}${X(i).toFixed(1)},${Y(x).toFixed(1)}`).join('');
  return `<svg viewBox="0 0 ${w} ${h + 14}" width="100%" class="strip" role="img" aria-label="Equity curve of the default run"><path d="${P}L${w},${h}L0,${h}Z" fill="var(--chart)" opacity=".08"/><path d="${P}" fill="none" stroke="var(--chart)" stroke-width="1.6"/><text x="0" y="${h + 12}">${esc(String(first).slice(0, 4))}</text><text x="${w}" y="${h + 12}" text-anchor="end">${esc(String(last).slice(0, 4))}</text></svg>`;
}
// green name: also the strategies marked by the research team (shown first on their main-variant page)
const ATH_MARKED = { ytath_plus_pct_buy_pctswing_exit_nse500_2: 'Marked by the research team: ATHC + % buy, %Swing exit, NSE500/F&O universe, running trade' };
const greenIds = id => ATH_CHECKED_IDS.includes(id) || id in ATH_MARKED;
// red name: the MASTER of a same-result group (the variant that contains almost all the rules of the others): verify these first
const masterName = id => ATH_MASTER_IDS.includes(id) ? ' class="yt-master" title="Master of its group of variants with the same result: it contains almost all the rules of the others, verify this one first"' : '';
// green + master (user 2026-10-31): green name first, then a red MASTER tag after it (masterTag)
const greenMaster = id => ATH_MASTER_IDS.includes(id) && (ATH_CHECKED_IDS.includes(id) || id in ATH_MARKED);
const masterTag = id => greenMaster(id) ? ' <span class="yt-master yt-master-tag" title="Master of its group of variants with the same result: it contains almost all the rules of the others, verify this one first">MASTER</span>' : '';
const checkedName = id => greenMaster(id) ? ` class="yt-checked" title="${esc(ATH_MARKED[id] || 'Added from the team\'s AFL ATH checking folder')}"` : ATH_MASTER_IDS.includes(id) ? masterName(id) : ATH_CHECKED_IDS.includes(id) ? ' class="yt-checked" title="Added from the team\'s AFL ATH checking folder"' : id in ATH_MARKED ? ` class="yt-checked" title="${esc(ATH_MARKED[id])}"` : '';
const greenFirst = defs => [...defs].sort((a, b) => greenIds(b.id) - greenIds(a.id));
function ytUniverseCards(perVariant) {
  if (R.name !== 'yt' || !perVariant || !S.grp || !S.family || !(UNIVERSE || ATH_UNIVERSE)) return '';
  const q = S.q.trim().toLowerCase();
  const defs = greenFirst(ALLCAT.filter(v => v.family === S.family && ytMainVariant(v.group) === S.grp && isUniverseOnly(v.id) && universeRunnable(uniOf(v.id), v.id) && (!q || `${v.name} ${v.variant || ''}`.toLowerCase().includes(q))));
  return defs.map(v => { const last = UNIVERSE_SUMMARY[v.id], pre = ATH_CARD[v.id], link = last?.hash || href('/s/' + encodeURIComponent(v.id), { run_scope: 'universe' });
    return `<article class="card yt-universe" data-yt-universe="${esc(v.id)}"><div class="rank"><span>${refDot(v, S.tk)}${esc(universeText(uniOf(v.id), 'badge', 'F&O stocks'))}</span>${verifyBadge(v.id)}<span class="stamp">${esc(universeText(uniOf(v.id), 'stamp', 'Runs on F&O stocks'))}</span></div>
      <h3${checkedName(v.id)}>${esc(v.variant || v.name)}${masterTag(v.id)}</h3><div class="by"><b style="font-weight:600">${esc(familyDisplayName(v))} · ${esc(S.grp)}</b> · ${esc(universeChoiceLabel(uniOf(v.id)))}</div>
      ${!last && pre ? `${athSpark(pre.sp, pre.first, pre.last)}${ytKpis(pre.stats, `${pre.max_positions} positions · default run`)}<div class="tags"><span class="tag">${esc(pre.universe)} (${esc(pre.symbols)})</span><span class="tag">${esc(pre.max_positions)} positions</span><span class="tag">${esc(v.dir || 'Long')}</span><span class="tag" title="Default settings of the AFL, 0.65 % brokerage per side, ${esc(pre.first)} to ${esc(pre.last)}. Calculated ${esc(pre.date)}; open the strategy to run it again.">calculated ${esc(pre.date)}</span></div>` : ''}
      ${last ? `${ytKpis(last.stats, `${last.max_positions} positions · last run`)}<div class="tags"><span class="tag">${esc(uniOf(v.id)?.universe || 'F&O stocks')} (${esc(uniOf(v.id)?.symbols || '')})</span><span class="tag">${esc(last.max_positions)} positions</span><span class="tag">${esc(v.dir || 'Long')}</span></div>`
        : pre ? '' : `<p class="yt-main-note">${esc(universeText(uniOf(v.id), 'card_note', `A portfolio of the team's rule on every stock of the ${uniOf(v.id).universe} watchlist (${uniOf(v.id).symbols} F&O stock futures, ${uniOf(v.id).first.slice(0, 4)}–${uniOf(v.id).last.slice(0, 4)}). Not run on an index; open it to run the stock-universe backtest.`))}</p>`}
      ${rulesList(v)}<div class="btns"><a class="btn solid" href="${link}">${v.id.startsWith('ytath_') ? 'Full backtest' : esc(universeText(uniOf(v.id), 'run_label', 'Run on F&O stocks'))}</a></div></article>`; }).join('');
}
// YT families none of whose sub-variants can run yet (ATH: waiting for its database) still get their cards:
// family card -> main variants (ytGroupCards) -> sub-variant cards with rules, reference dot and the reason.
function ytPendingCards(perVariant) {
  if (R.name !== 'yt') return '';
  const q = S.q.trim().toLowerCase(), hit = v => !q || `${v.name} ${v.family_name} ${v.variant || ''} ${v.group} ${(v.rules || []).join(' ')}`.toLowerCase().includes(q);
  const yt = ALLCAT.filter(isYt), fams = [...new Set(yt.map(v => v.family))].filter(f => !yt.some(v => v.family === f && v.status === 'live') && ytMainVariants(yt, f).length);
  if (!S.family && !perVariant) return fams.map(f => {
    const defs = yt.filter(v => v.family === f && hit(v)); if (!defs.length) return '';
    const mv = ytMainVariants(defs, f), name = familyDisplayName(defs[0]), why = defs[0].status_note || 'Backtest unavailable';
    const runnable = defs.filter(v => universeRunnable(uniOf(v.id), v.id)).length;
    return `<article class="card yt-pending" data-yt-pending="${esc(f)}"><div class="rank"><span>${refGroupDot(defs.map(v => v.id))}YT Strategies</span><span class="stamp" style="color:#8a6a2f">${runnable ? 'PARTLY TESTABLE' : 'NOT TESTABLE YET'}</span></div>
      <h3>${esc(name)}</h3><div class="by">YT Strategies · ${srcOf(defs[0])}</div><p class="yt-main-note">${mv.length} main variants · ${defs.length} sub-variants${refSheetCount(defs.map(v => v.id)) ? ` · ${refSheetCount(defs.map(v => v.id))} with a reference trade sheet (green dot)` : ''}. ${runnable ? `${runnable} can be backtested on the stock universe (checked against the team's trade sheet); the others wait for their engine.` : `${esc(why)}.`}</p>
      <div class="btns"><a class="btn solid" href="${href('/yt', { tk: S.tk, from: S.from, to: S.to, family: f })}">View variants</a></div></article>`; }).join('');
  if (!S.family || !S.grp || !fams.includes(S.family)) return '';
  return yt.filter(v => v.family === S.family && ytMainVariant(v.group) === S.grp && hit(v) && !universeRunnable(uniOf(v.id), v.id)).map(v => `<article class="card yt-pending" data-yt-pending-variant="${esc(v.id)}"><div class="rank"><span>${refDot(v, S.tk)}${esc(S.grp)}</span><span class="stamp" style="color:#8a6a2f">BACKTEST UNAVAILABLE</span></div>
      <h3${checkedName(v.id)}>${esc(v.variant || v.name)}${masterTag(v.id)}</h3><div class="by"><b style="font-weight:600">${esc(familyDisplayName(v))} · ${esc(S.grp)}</b></div>
      <div class="unavailable-reasons"><p>${esc(v.status_note || 'Backtest unavailable')}</p></div>${rulesList(v)}
      <div class="btns"><a class="btn solid" href="${stratLink(v.id, S.tk)}">Open rules &amp; parameters</a></div></article>`).join('');
}
function viewStrategies(rs, perVariant, missing = []) {
  const uniCards = ytUniverseCards(perVariant) + ytPendingCards(perVariant);
  const missingGrid=missing.length?`<div class="grid unavailable-method-grid">${missing.map(unavailableBookCard).join('')}</div>`:'';
  // strategies = one entry per family (best variant by the current sort); variants = every rule set
  let list = rs;
  if (!perVariant) { const seen = new Map(); for (const r of rs) { const k = r.v.family; if (!seen.has(k)) seen.set(k, { ...r, all: [] }); seen.get(k).all.push(r); } list = [...seen.values()];
    // YT families open on their first variant (the team's own default), not on whichever ranks best; rank order is unchanged.
    if (R.name === 'yt') { const at = r => ALLCAT.findIndex(v => v.id === r.v.id); list = list.map(e => ({ ...e.all.reduce((a, b) => at(b) < at(a) ? b : a), all: e.all })); } }
  const nVar = r => r.all ? new Set(r.all.map(x => x.v.id)).size : 1, nIdx = r => r.all ? new Set(r.all.map(x => x.tk)).size : 1;
  // inside a YT main variant every card is the same family, so the sub-variant name is the title
  const subTitled = r => perVariant && R.name === 'yt' && S.grp && r.v.variant;
  const title = r => esc(subTitled(r) ? r.v.variant : familyDisplayName(r.v)) + gateBadge(r.v);
  const sub = r => subTitled(r) ? `<b style="font-weight:600">${esc(familyDisplayName(r.v))} · ${esc(S.grp)}</b> · ` : perVariant && r.v.variant ? `<b style="font-weight:600">${esc(r.v.variant)}</b> · ` : '';
  if (S.tab === 'awaiting') return S.book&&missing.length?`<div class="grid">${missing.map(unavailableBookCard).join('')}</div>`:awaiting(rs);
  const uniGrid = uniCards ? `<div class="grid yt-universe-grid">${uniCards}</div>` : '';
  if (S.view === 'list') {
    const M = TABS[S.tab].metric, ruleLine = v => { const L = ruleLines(v), e = L.find(([t]) => t === 'E'), x = L.find(([t]) => t === 'X'); return [e && 'Entry: ' + e[1], x && 'Exit: ' + x[1]].filter(Boolean).map(esc).join(' · '); };
    return `<div class="list">${list.map((r, i) => { const vd = verdictOf(r.s); return `<div class="lr"><span class="ln">${refDot(r.v, r.tk)}${i + 1}</span><div class="sp">${spark(r, 110, 40)}</div>
      <div class="lt"><b>${title(r)}</b><small>${sub(r)}${byline(r.v)}${S.tk === 'ALL' ? ' · ' + esc(LABEL[r.tk]) : ''}</small><span class="rl">${ruleLine(r.v)}</span>
        <span class="ltags"><span class="tag">⏱ ${HOLD[holdOf(r.s)]}</span><span class="tag">${esc(r.v.style)}</span><span class="tag">${r.v.dir}</span>${!perVariant && nVar(r) > 1 ? `<span class="tag link" data-fam="${esc(r.v.family)}">${nVar(r)} variants</span>` : ''}</span></div>
      <div class="lk"><span>${esc(M.label)}<b class="p">${M.fmt(M.get(r))}</b></span><span>CAGR<b class="${cls(r.s.cagr)}">${pct(r.s.cagr, 1, true)}</b></span><span>Max DD<b class="neg">${pct(r.s.max_dd, 1)}</b></span><span>Trades<b>${fx(r.s.trades, 0)}</b></span></div>
      <div class="la"><span class="stamp" style="color:${vd.c}">${vd.t}</span>${ytFamilyCard(r, perVariant) ? `<a class="go" href="${href('/yt', { tk: S.tk, from: S.from, to: S.to, family: r.v.family })}">View variants</a>` : `<a class="go" href="${appLink(r)}">Full backtest</a>`}</div></div>`; }).join('')}</div>${uniGrid}${missingGrid}`;
  }
  if (S.view === 'table') {
    const M = TABS[S.tab].metric, th = (k, t, n) => `<th class="${n ? 'n' : ''} ${S.sort === k ? 'on' : ''}" data-sort="${k}">${t}</th>`;
    return `<div class="tblwrap"><table class="tbl"><thead><tr><th class="nosort">#</th>${th('name', 'Strategy')}<th class="nosort">Style</th>${S.tk === 'ALL' ? '<th class="nosort">Index</th>' : ''}<th class="nosort">Hold</th><th class="nosort">Equity</th>${th('primary', esc(M.label), 1)}${S.tab === 'working' ? `<th class="n nosort">Before ${PUB}</th>` : S.tab === 'trending' ? '<th class="n nosort">Sharpe (full)</th>' : ''}${th('cagr', 'CAGR', 1)}${th('maxdd', 'Max DD', 1)}${th('win', 'Win', 1)}${th('trades', 'Trades', 1)}${th('t', 't', 1)}<th class="nosort"></th></tr></thead><tbody>
      ${list.map((r, i) => `<tr><td class="rk">${i + 1}</td><td class="nm"><b>${title(r)}</b><span>${sub(r)}${byline(r.v)}${!perVariant && nVar(r) > 1 ? ` · <a href="#" data-fam="${esc(r.v.family)}">${nVar(r)} variants</a>` : ''}</span></td><td>${esc(r.v.style)}<br><span class="muted">${r.v.dir}</span></td>${S.tk === 'ALL' ? `<td>${esc(LABEL[r.tk])}</td>` : ''}<td>${HOLD[holdOf(r.s)]}</td><td style="width:150px">${spark(r, 150, 34)}</td>
        <td class="n p">${M.fmt(M.get(r))}</td>${S.tab === 'working' || S.tab === 'trending' ? `<td class="n">${fx(r.x?.sharpe)}</td>` : ''}<td class="n ${cls(r.s.cagr)}">${pct(r.s.cagr, 1, true)}</td><td class="n neg">${pct(r.s.max_dd, 1)}</td><td class="n">${pct(r.s.win_rate, 0)}</td><td class="n">${fx(r.s.trades, 0)}</td><td class="n">${fx(r.s.t_stat)}</td><td><a class="go" href="${appLink(r)}">Backtest</a></td></tr>`).join('')}
      </tbody></table></div>${uniGrid}${missingGrid}`;
  }
  return `<div class="grid">${list.map((r, i) => { const vd = verdictOf(r.s); return `<article class="card">
      <div class="rank"><span>${refDot(r.v, r.tk)}#${i + 1}${S.tk === 'ALL' ? ' · ' + esc(LABEL[r.tk]) : ''}</span>${verifyBadge(r.v.id, r.tk)}<span class="stamp" style="color:${vd.c}">${vd.t}</span></div>
      <h3>${title(r)}</h3><div class="by">${sub(r)}${byline(r.v)}</div>
      ${spark(r)}${kpiTriplet(r)}
      <div class="tags"><span class="tag">⏱ ${HOLD[holdOf(r.s)]}</span><span class="tag">${esc(isYt(r.v) ? (btstFuturesSymbol(r.tk) || LABEL[r.tk] || r.tk) : r.v.style)}</span><span class="tag">${r.v.dir}</span>${!perVariant && nVar(r) > 1 ? `<span class="tag link" data-fam="${esc(r.v.family)}">${nVar(r)} variants</span>` : ''}${!perVariant && S.tk === 'ALL' && nIdx(r) > 1 ? `<span class="tag">best of ${nIdx(r)} indices</span>` : ''}</div>
      ${rulesList(r.v)}
      <div class="btns">${ytFamilyCard(r, perVariant) ? `<a class="btn solid" href="${href('/yt', { tk: S.tk, from: S.from, to: S.to, family: r.v.family })}">View variants</a>` : `<a class="btn solid" href="${appLink(r)}">Full backtest</a>`}${isYt(r.v) ? '' : `<button class="btn" data-book="${r.v.book}">Book</button><button class="btn" data-guru="${r.v.guru}">Guru</button>`}</div>
    </article>`; }).join('')}${uniCards}${missing.map(unavailableBookCard).join('')}</div>`;
}
function awaiting(rs) {
  const reasons = r => sourceBlockers(r.v) || r.issues.map(issue => `<p><b>${esc(LABEL[issue.tk])}: ${esc(heldLabel(issue))}</b><br>${esc(issue.note)}</p>`).join('');
  const link = r => `<a href="${stratLink(r.v.id,r.tk,r.region)}">${esc(familyDisplayName(r.v))}</a>`;
  if (S.view === 'table' || S.view === 'list') return `<div class="tblwrap"><table class="tbl"><thead><tr><th class="nosort">#</th><th class="nosort">Strategy / source method</th><th class="nosort">Style</th><th class="nosort">Why results are withheld</th></tr></thead><tbody>${rs.map((r,i)=>`<tr><td class="rk">${i+1}</td><td class="nm"><b>${link(r)}</b><span>${r.v.variant?esc(r.v.variant)+' · ':''}${byline(r.v)}</span></td><td>${esc(r.v.style)}</td><td>${reasons(r)}</td></tr>`).join('')}</tbody></table></div>`;
  return `<div class="grid">${rs.map((r,i)=>`<article class="card"><div class="rank"><span>${refDot(r.v, r.tk)}#${i+1}</span><span class="stamp" style="color:#8a6a2f">${r.v.guru==='ross'?'DATA MISSING · RULES UNRESOLVED':'BACKTEST UNAVAILABLE'}</span></div><h3>${link(r)}</h3><div class="by">${r.v.variant?esc(r.v.variant)+' · ':''}${byline(r.v)}</div><div class="unavailable-reasons">${reasons(r)}</div><div class="tags"><span class="tag">${esc(r.v.style)}</span><span class="tag">${r.v.dir}</span></div>${rulesList(r.v)}<a class="btn" href="${stratLink(r.v.id,r.tk)}">View rules and data requirements</a></article>`).join('')}</div>`;
}
function groupBy(rs, key) { const m = new Map(); for (const r of rs) { const k = key(r); if (!m.has(k)) m.set(k, []); m.get(k).push(r); } return m; }
function summarise(list) {
  const M = TABS[S.tab].metric, fams = groupBy(list, r => r.v.family);
  const bestPerFam = [...fams.values()].map(a => a[0]); // rows already sorted
  return { fams: fams.size, variants: new Set(list.map(r => r.v.id)).size, best: list.find(r => r.s?.trades>=20 && Number.isFinite(r.s?.sharpe)), vals: M ? bestPerFam.map(M.get) : [], sig: bestPerFam.filter(r => r.s && verdictOf(r.s).k === 'sig').length };
}
function viewBooks(rs) {
  const M = TABS[S.tab].metric, g = groupBy(rs, r => r.v.book);
  const live = [...g.entries()].map(([id, list]) => ({ b: BOOK[id], ...summarise(list) }));
  const filteredOnlyByHierarchy = !S.q && !S.style && !S.dir && !S.hold && !S.verdict && S.min === '0' && !S.family;
  const previews = filteredOnlyByHierarchy ? Object.values(BOOK).filter(b => !b.live && (!S.guru || b.guru.id === S.guru) && !S.book) : [];
  if (S.view === 'table') return `<div class="tblwrap"><table class="tbl"><thead><tr><th class="nosort">#</th><th class="nosort">Book</th><th class="nosort">Guru</th><th class="n nosort">Year</th><th class="n nosort">Strategies</th><th class="n nosort">Rule sets</th><th class="nosort">Best strategy</th><th class="n nosort">Best ${M ? esc(M.label) : ''}</th><th class="n nosort">Median</th><th class="n nosort">Significant</th><th class="nosort"></th></tr></thead><tbody>
    ${live.map((x, i) => `<tr><td class="rk">${i + 1}</td><td class="nm"><b>${esc(x.b.title)}</b></td><td>${esc(x.b.guru.name)}</td><td class="n">${bookYear(x.b)}</td><td class="n">${x.fams}</td><td class="n">${x.variants}</td><td>${x.best ? esc(familyDisplayName(x.best.v)) : '–'}</td><td class="n p">${M && x.best?.s ? M.fmt(M.get(x.best)) : '–'}</td><td class="n">${M ? M.fmt(med(x.vals)) : '–'}</td><td class="n">${x.sig}</td><td><button class="go" data-book="${x.b.id}">Strategies</button></td></tr>`).join('')}
    ${previews.map(b => `<tr class="preview"><td class="rk">–</td><td class="nm"><b>${esc(b.title)}</b></td><td>${esc(b.guru.name)}</td><td class="n">${bookYear(b)}</td><td colspan="7">Reading material · no admitted performance</td></tr>`).join('')}</tbody></table></div>`;
  return `<div class="grid">${live.map((x, i) => `<article class="card"><div class="rank"><span>#${i + 1}</span><span>${x.fams} strategies · ${x.variants} rule sets</span></div>
      <div class="cover"><div class="bk" style="--c:${x.b.c}"><span>${esc(x.b.title)}</span><span>${bookYear(x.b)}</span></div><div><h3>${esc(x.b.title)}</h3><div class="by" style="margin:2px 0 0">${esc(x.b.guru.name)} · ${bookYear(x.b)}</div></div></div>
      ${stripPlot(x.vals)}
      <div class="kpis"><div><span>Best ${M ? esc(M.label) : ''}</span><b class="p">${M && x.best?.s ? M.fmt(M.get(x.best)) : '–'}</b><i>${x.best ? esc(familyDisplayName(x.best.v)) : ''}</i></div><div><span>Median</span><b>${M ? M.fmt(med(x.vals)) : '–'}</b><i>across strategies</i></div><div><span>Significant</span><b>${x.sig}</b><i>of ${x.fams}</i></div></div>
      <div class="btns"><button class="btn solid" data-book="${x.b.id}">Open book · ${x.fams} strategies</button></div></article>`).join('')}
    ${previews.length ? `<div class="sep">Reading material · no admitted performance</div>` + previews.map(b => `<article class="card preview"><div class="cover"><div class="bk pv"><span>${esc(b.title)}</span><span>${bookYear(b)}</span></div><div><h3>${esc(b.title)}</h3><div class="by" style="margin:2px 0 0">${esc(b.guru.name)} · ${bookYear(b)}</div></div></div><p style="margin:0;font-size:13px">Reading material. An objective backtest requires complete source rules and the appropriate market data; none is invented.</p></article>`).join('') : ''}</div>`;
}
function viewGurus(rs) {
  const M = TABS[S.tab].metric, g = groupBy(rs, r => r.v.guru);
  const live = [...g.entries()].map(([id, list]) => ({ g: GURU[id], ...summarise(list), books: new Set(list.map(r => r.v.book)).size }));
  const filteredOnlyByHierarchy = !S.q && !S.style && !S.dir && !S.hold && !S.verdict && S.min === '0' && !S.family && !S.book;
  const previews = filteredOnlyByHierarchy ? LIB.filter(x => x.id!=='ross' && !g.has(x.id) && (!S.guru || x.id === S.guru)) : [];
  const face = x => x.photo ? `<div class="ph"><img src="${x.photo}" alt=""></div>` : `<div class="ph">${x.name.split(' ').map(w => w[0]).join('')}</div>`;
  if (S.view === 'table') return `<div class="tblwrap"><table class="tbl"><thead><tr><th class="nosort">#</th><th class="nosort">Guru</th><th class="n nosort">Books</th><th class="n nosort">Converted</th><th class="n nosort">Strategies</th><th class="nosort">Best strategy</th><th class="n nosort">Best ${M ? esc(M.label) : ''}</th><th class="n nosort">Median</th><th class="n nosort">Significant</th><th class="nosort"></th></tr></thead><tbody>
    ${live.map((x, i) => `<tr><td class="rk">${i + 1}</td><td class="nm"><b>${esc(x.g.name)}</b></td><td class="n">${x.g.books.length}</td><td class="n">${x.books}</td><td class="n">${x.fams}</td><td>${x.best ? esc(familyDisplayName(x.best.v)) : '–'}</td><td class="n p">${M && x.best?.s ? M.fmt(M.get(x.best)) : '–'}</td><td class="n">${M ? M.fmt(med(x.vals)) : '–'}</td><td class="n">${x.sig}</td><td><button class="go" data-guru="${x.g.id}">Books</button></td></tr>`).join('')}
    ${previews.map(x => `<tr class="preview"><td class="rk">–</td><td class="nm"><b>${esc(x.name)}</b></td><td class="n">${x.books.length}</td><td class="n">0</td><td colspan="6">Reading material · no admitted performance</td></tr>`).join('')}</tbody></table></div>`;
  return `<div class="grid">${live.map((x, i) => `<article class="card"><div class="rank"><span>#${i + 1}</span><span>${x.g.books.length} books · ${x.books} converted</span></div>
      <div class="cover">${face(x.g)}<div><h3>${esc(x.g.name)}</h3><div class="by" style="margin:2px 0 0">${esc(GURUS_API.find(a => a.id === x.g.id)?.tagline || '')}</div></div></div>
      ${stripPlot(x.vals)}
      <div class="kpis"><div><span>Best ${M ? esc(M.label) : ''}</span><b class="p">${M && x.best?.s ? M.fmt(M.get(x.best)) : '–'}</b><i>${x.best ? esc(familyDisplayName(x.best.v)) : ''}</i></div><div><span>Strategies</span><b>${x.fams}</b><i>${x.variants} rule sets</i></div><div><span>Significant</span><b>${x.sig}</b><i>of ${x.fams}</i></div></div>
      <div class="btns"><button class="btn solid" data-guru="${x.g.id}">Books</button><button class="btn" data-guru-strats="${x.g.id}">All strategies</button></div></article>`).join('')}
    ${previews.length ? `<div class="sep">Reading material · no admitted performance</div>` + previews.map(x => `<article class="card preview"><div class="cover">${face(x)}<div><h3>${esc(x.name)}</h3><div class="by" style="margin:2px 0 0">${x.books.map(b => esc(b.title)).join(' · ')}</div></div></div><p style="margin:0;font-size:13px">Reading library · no admitted performance.</p></article>`).join('') : ''}</div>`;
}

// ---- charts and exit checks (same code as book-worlds.html)
function eqChart(d, w = 520, h = 150) {
  const st = Math.max(1, Math.floor(d.equity.length / 260)), e = d.equity.filter((_, i) => i % st === 0), b = d.bh.filter((_, i) => i % st === 0), dt = d.dates.filter((_, i) => i % st === 0);
  const L = Math.log, all = e.concat(b).map(L), lo = Math.min(...all), hi = Math.max(...all), n = e.length, X = i => 34 + i / (n - 1) * (w - 38), Y = v => h - 14 - (L(v) - lo) / ((hi - lo) || 1) * (h - 22);
  const P = v => v.map((x, i) => `${i ? 'L' : 'M'}${X(i).toFixed(1)},${Y(x).toFixed(1)}`).join('');
  const yrs = [...new Set(dt.map(t => t.slice(0, 4)))].filter((_, k, a) => k % Math.ceil(a.length / 6) === 0);
  return `<svg viewBox="0 0 ${w} ${h}" width="100%"><line x1="34" x2="${w - 4}" y1="${Y(1)}" y2="${Y(1)}" stroke="currentColor" stroke-opacity=".25" stroke-dasharray="3 3"/>
    <text x="0" y="${Y(Math.exp(hi)) + 8}" font-size="9" fill="currentColor" fill-opacity=".6">${Math.exp(hi).toFixed(1)}×</text><text x="0" y="${Y(1) + 3}" font-size="9" fill="currentColor" fill-opacity=".6">1×</text>
    ${yrs.map(y => { const i = dt.findIndex(t => t.startsWith(y)); return `<text x="${X(i)}" y="${h - 2}" font-size="9" fill="currentColor" fill-opacity=".55" text-anchor="middle">${y}</text>`; }).join('')}
    <path d="${P(b)}" fill="none" stroke="#a8a297" stroke-width="1.3"/><path d="${P(e)}" fill="none" stroke="var(--ac)" stroke-width="2"/></svg>`;
}
function ddChart(d, w = 520, h = 60) {
  const st = Math.max(1, Math.floor(d.equity.length / 260)); let pk = 1; const dd = d.equity.map(v => { pk = Math.max(pk, v); return v / pk - 1; }).filter((_, i) => i % st === 0 || i === d.equity.length - 1);
  const lo = Math.min(...dd, -.01), n = dd.length, P = dd.map((x, i) => `${i ? 'L' : 'M'}${(34 + i / (n - 1) * (w - 38)).toFixed(1)},${(2 + x / lo * (h - 6)).toFixed(1)}`).join('');
  return `<svg viewBox="0 0 ${w} ${h}" width="100%"><text x="0" y="${h - 4}" font-size="9" fill="currentColor" fill-opacity=".6">${(lo * 100).toFixed(0)}%</text><path d="${P}L${w - 4},2L34,2Z" fill="#b43a3426"/><path d="${P}" fill="none" stroke="#b43a34" stroke-width="1.1"/></svg>`;
}
function periods(d, len) { const o = {}; let base = 1, key = d.dates[0].slice(0, len); d.dates.forEach((t, i) => { const k = t.slice(0, len); if (k !== key) { o[key] = d.equity[i - 1] / base - 1; base = d.equity[i - 1]; key = k; } }); o[key] = d.equity[d.equity.length - 1] / base - 1; return o; }
function yearBars(d, w = 520, h = 80) {
  const s = periods(d, 4), b = periods({ dates: d.dates, equity: d.bh }, 4), ks = Object.keys(s), mx = Math.max(.01, ...ks.map(k => Math.max(Math.abs(s[k]), Math.abs(b[k])))), bw = (w - 30) / ks.length, Y = v => h / 2 - v / mx * (h / 2 - 4);
  return `<svg viewBox="0 0 ${w} ${h + 12}" width="100%"><line x1="30" x2="${w}" y1="${h / 2}" y2="${h / 2}" stroke="currentColor" stroke-opacity=".3"/>${ks.map((k, i) => { const x = 30 + i * bw; return `<rect x="${x + 1}" y="${Math.min(Y(s[k]), h / 2)}" width="${Math.max(1, bw / 2 - 1)}" height="${Math.max(.5, Math.abs(Y(s[k]) - h / 2))}" fill="${s[k] >= 0 ? 'var(--ac)' : 'var(--red)'}"/><rect x="${x + bw / 2}" y="${Math.min(Y(b[k]), h / 2)}" width="${Math.max(1, bw / 2 - 1)}" height="${Math.max(.5, Math.abs(Y(b[k]) - h / 2))}" fill="#a8a297"/>${i % 5 === 0 ? `<text x="${x + bw / 2}" y="${h + 10}" font-size="8.5" text-anchor="middle" fill="currentColor" fill-opacity=".55">${k}</text>` : ''}`; }).join('')}</svg>`;
}
function heat(d) {
  const m = periods(d, 7), ys = periods(d, 4), cap = Math.max(.01, ...Object.values(m).map(Math.abs)), M = 'JFMAMJJASOND';
  const bg = v => v == null ? 'transparent' : Math.abs(v) < 1e-9 ? 'color-mix(in srgb, currentColor 6%, transparent)' : `color-mix(in srgb, ${v > 0 ? 'var(--ac)' : 'var(--red)'} ${Math.round((Math.min(1, Math.abs(v) / cap) * .75 + .12) * 100)}%, transparent)`;
  return `<table class="heat"><tr><th></th>${[...M].map(x => `<th>${x}</th>`).join('')}<th>Yr</th></tr>${Object.keys(ys).reverse().map(y => `<tr><th>${y.slice(2)}</th>${[...M].map((_, k) => { const v = m[`${y}-${String(k + 1).padStart(2, '0')}`]; return `<td style="background:${bg(v)}" title="${y}-${k + 1}: ${v == null ? '–' : pct(v, 2, true)}">${v == null || Math.abs(v) < 1e-9 ? '' : (v * 100).toFixed(2)}</td>`; }).join('')}<td class="${cls(ys[y])}"><b>${(ys[y] * 100).toFixed(2)}</b></td></tr>`).join('')}</table>`;
}
function hist(rets, w = 520, h = 80) {
  if (!rets.length) return '<div class="cap">No closed trades.</div>';
  const lo = Math.min(...rets), hi = Math.max(...rets), nb = Math.min(30, Math.max(8, Math.ceil(Math.sqrt(rets.length) * 1.4))), bw0 = (hi - lo) / nb || .001, c = Array(nb).fill(0);
  rets.forEach(r => c[Math.min(nb - 1, Math.floor((r - lo) / bw0))]++); const mx = Math.max(...c), bw = (w - 10) / nb;
  return `<svg viewBox="0 0 ${w} ${h + 12}" width="100%">${c.map((v, k) => { const mid = lo + (k + .5) * bw0, bh = v / mx * (h - 4); return `<rect x="${5 + k * bw + 1}" y="${h - bh}" width="${bw - 2}" height="${bh}" fill="${mid >= 0 ? 'var(--ac)' : 'var(--red)'}"><title>${pct(lo + k * bw0)} to ${pct(lo + (k + 1) * bw0)}: ${v}</title></rect>`; }).join('')}<text x="5" y="${h + 10}" font-size="9" fill="currentColor" fill-opacity=".6">${pct(lo, 1)}</text><text x="${w - 5}" y="${h + 10}" font-size="9" text-anchor="end" fill="currentColor" fill-opacity=".6">${pct(hi, 1)}</text></svg>`;
}


function exitCheck(t) { return tradeExitCheck(t); }


// ================================================================ FULL VARIANT 1
// Routes:  #/  landing (legends)  ·  #/guru/<id>  author grid  ·  #/strategies  all gurus  ·  #/s/<id>  one strategy
// Filters live in a top navbar of dropdowns (DaisyUI-style); animation by motion.dev; charts in the Bklit style.
DEF.research_mode=''; DEF.view = 'grid'; DEF.region = 'in'; DEF.from = ''; DEF.to = ''; DEF.cost = '0'; DEF.slippage='0'; DEF.rollover='0'; DEF.basket=''; DEF.run_scope='single'; DEF.only=''; DEF.uni_basket=''; DEF.uni_set=''; DEF.report_mode='fixed'; DEF.membership='current'; DEF.max_positions='10'; DEF.position_choice='10'; DEF.p = ''; DEF.library_scope='trading'; DEF.library_market=''; DEF.library_q=''; DEF.library_view='authors'; DEF.library_sort='default'; DEF.book_category=''; DEF.library_limit='24'; DEF.library_page='1'; DEF.source_page='1'; DEF.book_progress=''; // p = custom strategy settings 'key:value,…' (empty = the book's) // custom backtest period, kept in the link
const getS = () => S;
const MO = window.Motion, CALM = matchMedia('(prefers-reduced-motion: reduce)').matches;
const anim = (el, kf, o) => { if (MO && !CALM && el && (el.length === undefined || el.length)) try { return MO.animate(el, kf, o); } catch (e) {} };
const famBest = rs => [...groupBy(rs, r => r.v.family).values()].map(a => a[0]);
const distinct = (rs, f) => new Set(rs.map(f)).size;
const ds = o => esc(new URLSearchParams(o).toString());
const VERD = { sig: 'Significant (t ≥ 2)', none: 'No edge', thin: 'Thin data (< 30 trades)' };
const MODES = { strategies: 'Strategies', variants: 'Rule variants', books: 'Books', gurus: 'Gurus' };
let R = { name: 'home' };

// ---------------------------------------------------------------- routing (state after "?", so Back/Forward work)
function readRoute() {
  const [path, q = ''] = (location.hash.slice(1) || '/').split('?'), p = path.split('/').filter(Boolean);
  R = p[0] === 'source' && p[1] ? {name:'source',id:decodeURIComponent(p[1]),path:path} : p[0] === 'saved' ? { name: 'saved', path: '/saved' } : p[0] === 'strategies' ? { name: 'all', path: '/strategies' } : p[0] === 'yt' ? { name: 'yt', path: '/yt' } : p[0] === 'guru' && GURU[p[1]] ? { name: 'guru', id: p[1], path: '/guru/' + p[1] }
    : ['s','o'].includes(p[0]) && p[1] ? { name: 'strategy', optim: p[0] === 'o', id: decodeURIComponent(p[1]), path: path } : { name: 'home', path: '/' };
  S = { ...DEF }; for (const [k, v] of new URLSearchParams(q)) if (k in DEF) S[k] = v;
  S.report_mode=S.report_mode==='compound'?'compound':'fixed';
  S.run_scope=normaliseRunScope(S.run_scope,R.name==='strategy',R.id); // BTST: 'universe' = the F&O stock universe (universe-only sub-variants always)
  if(S.run_scope==='universe'&&uniOf(R.id)&&!universeRunnable(uniOf(R.id),R.id))S.run_scope='single';
  if(S.run_scope==='universe'){S.tk='NIFTY 50';S.basket='';} // the universe run uses the NIFTY calendar and benchmark
  S.membership=S.membership==='past'?'past':'current';
  S.max_positions=String(positionLimit(S.max_positions)||10);
  S.position_choice=['10','20','custom'].includes(S.position_choice)?S.position_choice:'10';
  if(!pristineMethod(R.id,PRISTINE_BOOK))S.cost='0'; // Legacy interface retains its existing fee setting.
  if (R.name === 'guru') S.guru = R.id;
  // YT strategies have no author or book: drop those filters and the book-based views.
  if (R.name === 'yt') { S.guru = ''; S.book = ''; if (!['strategies', 'variants'].includes(S.mode)) S.mode = 'strategies'; if (S.tab === 'working') S.tab = 'sharpe'; }
  if (S.book?.startsWith('jr_') && !new URLSearchParams(q).has('tab')) S.tab='awaiting';
  if (S.book?.startsWith('jr_') && !BOOK[S.book]?.year && S.tab==='working') S.tab='awaiting';
  // Use each source-qualified basket only with the market engine that admitted it.
  if(S.region==='us'&&(TICK_US||pristineMethod(R.id,PRISTINE_BOOK))){API=API_US;BASKETS=BASKETS_US;}else{if(!(S.region==='all'&&R.name==='all'&&TICK_US)){S.region='in';}else{S.tk='ALL';S.basket='';}API=API_IN;BASKETS=BASKETS_IN;}
  const pool=S.region==='us'?TICK_ALL_US:TICK_ALL_IN, indices=S.region==='us'?TICK_US:TICK_IN;
  const chosen=R.name==='strategy'?ALLCAT.find(v=>v.id===R.id):null;
  const types=chosen?.instrument_policy?.allowed_types||[];
  const etfs=S.region==='us'?TICK_ALL_US.filter(t=>['SPY','QQQ','MDY','IWM','SMH','DIA'].includes(t.ticker)):[];
  const basket=BASKETS.find(b=>b.id===S.basket);
  const explicit=new URLSearchParams(q),isDetail=R.name==='strategy';
  if(chosen && types.includes('individual_stock') && !types.includes('index') && !types.includes('etf')
    && !explicit.get('tk') && S.run_scope==='single') {
    const preferred=BASKETS.find(b=>b.id===(S.region==='us'?'us_sp500':chosen.instrument_policy.preferred_basket||'nifty50'));
    S.tk=pool.find(t=>preferred?.symbols?.includes(t.ticker))?.ticker||S.tk;
  }
  const stocks=isDetail&&types.includes('individual_stock')?stockChoices(pool,S.tk):[];
  TICKERS=isDetail&&S.run_scope==='universe'?indices.filter(t=>t.ticker==='NIFTY 50'):isDetail&&S.run_scope==='single'
    ? [...instrumentTickers(chosen?.instrument_policy,indices,etfs,chosen?.id,chosen?.book),...stocks]
    : S.basket&&basket?.symbols?.length?pool.filter(t=>basket.symbols.includes(t.ticker))
    : instrumentTickers(['home','guru','all','yt'].includes(R.name)?{allowed_types:['index','etf']}:chosen?.instrument_policy,indices,etfs,chosen?.id,chosen?.book);
  // Keep an explicitly named, admitted source even when its instrument is
  // incompatible. The readiness gate explains the mismatch without replacing it.
  const explicitStock=isDetail&&S.run_scope==='single'&&explicit.get('tk')?.startsWith('basket:')?pool.find(t=>t.ticker===S.tk):null;
  if(explicitStock&&!TICKERS.some(t=>t.ticker===S.tk))TICKERS.push(explicitStock);
  const explicitUnitSelection=isDetail&&unitExposureMethod(chosen?.id,chosen?.book)&&explicit.get('tk')?pool.find(t=>t.ticker===S.tk):null;
  if(explicitUnitSelection&&!TICKERS.some(t=>t.ticker===S.tk))TICKERS.push(explicitUnitSelection); // Preserve actual incompatible URLs for an honest admission hold.
  const explicitPristine=isDetail&&pristineMethod(chosen?.id,chosen?.book)&&explicit.has('tk');
  if(S.tk!=='ALL'&&!TICKERS.some(t=>t.ticker===S.tk)&&!explicitPristine)S.tk=TICKERS[0]?.ticker||S.tk;
  // Preserve explicit basket URLs: the readiness gate explains incompatible instruments.
  // Silent substitution back to an index made basket clicks appear stuck.
  CAT = marketCat();
}
// Market-specific variants (NSE breadth on India, NYSE breadth on the US) are shown only on their own market.
let ALLCAT = [];
let UNIVERSE = null; // BTST F&O stock universe info (guru/yt/universe), null until loaded or when the engine has none
let ATH_UNIVERSE = null; // ATH stock universe info (guru/yt/ath-universe): the sub-variants the Rust ATH engine runs on the NSE 500 C&P watchlist
const uniOf = id => { // the universe a sub-variant runs on (ATH: the info of the engine, with the wording of the sub-variant's own database)
  if (!id || !id.startsWith('ytath_')) return UNIVERSE;
  if (!ATH_UNIVERSE) return null;
  const own = ATH_UNIVERSE.by_id?.[id];
  return own ? { ...ATH_UNIVERSE, ...own } : ATH_UNIVERSE;
};
// stock-universe baskets (engine `baskets`): NSE 500 / Nifty 50 / All F&O, each as Current, Past (C&P minus current) or C&P (default)
const uniBaskets = id => (id?.startsWith('ytath_') ? ATH_UNIVERSE?.baskets?.[ATH_UNIVERSE?.by_id?.[id]?.members_key] : UNIVERSE?.baskets) || [];
const uniDefaultBasket = id => { const U = uniOf(id), bs = uniBaskets(id); return bs.find(b => b.cp.replace(/\.tls$/, '') === U?.universe)?.key || bs[0]?.key || ''; };
const uniPick = id => { const bs = uniBaskets(id), key = S.uni_basket || uniDefaultBasket(id), b = bs.find(x => x.key === key) || null, set = b?.current ? (S.uni_set || 'cp') : 'cp';
  return { b, key, set, isDefault: key === uniDefaultBasket(id) && set === 'cp' }; };
const UNI_SET = { current: 'Current', past: 'Past', cp: 'C&P' };
const uniMembers = id => (id?.startsWith('ytath_') ? ATH_UNIVERSE?.members?.[ATH_UNIVERSE?.by_id?.[id]?.members_key] : UNIVERSE?.members) || [];
const universeScope = v => S.run_scope === 'universe' && R.name === 'strategy' && !!v && universeRunnable(uniOf(v.id), v.id);
// the parameter schema of the page's run: a universe run has its own (engine `schemas`, else core rules + portfolio keys)
const btSchema = v => universeScope(v) ? universeSchema(uniOf(v.id), v.id, OPT.schemaSync?.ytbtst_core || []) : OPT.schemaSync?.[v.id] || [];
const btRef = (v, tk) => universeScope(v) ? ytUniverseReferenceDefaults(v.id) : ytReferenceDefaults(v.id, tk); // BTST and ATH sub-variants
const foreignTo = (v, us) => (v.needs_series || []).some(n => us ? /^NSE_/.test(n) : /^NYSE_/.test(n));
const marketCat = () => libraryMarketVariants(ALLCAT,libraryMarket(S,R.name));
const href = (path, st = {}) => { const q = new URLSearchParams();if(S.research_mode&&!('research_mode' in st))q.set('research_mode',S.research_mode);const catalogue=path==='/'||path==='/strategies'||path==='/yt'||path.startsWith('/guru/')||path.startsWith('/source/'); if (S.region !== 'in' && !('region' in st)) q.set('region', S.region); if(catalogue&&S.library_market&&!('library_market' in st))q.set('library_market',S.library_market); for (const k in st) if (st[k] !== DEF[k] && st[k] != null && !(path.startsWith('/guru/') && k === 'guru') && (k!=='library_market'||catalogue)) q.set(k, st[k]); const s = q.toString(); return '#' + path + (s ? '?' + s : ''); };
const go = (path, st) => { location.hash = href(path, st).slice(1); };
const isYt = v => v.api === 'yt'; // YT strategies: no author or book, own tab
const setState = patch => { const st = { ...S, ...patch }; if ('guru' in patch && !('book' in patch) && st.book && BOOK[st.book]?.guru.id !== st.guru) st.book = ''; go(R.path, st); };
const guruLink = id => href('/guru/' + id, {view:'grid',tk:S.tk,basket:S.basket,from:S.from,to:S.to});
const stratLink = (id, tk, region) => href('/s/' + encodeURIComponent(id), { ...(region ? { region } : S.region === 'all' ? { region: 'in' } : {}), ...(tk && tk !== 'NIFTY 50' && tk !== 'ALL' ? { tk } : {}), from: S.from, to: S.to, cost: S.cost,slippage:S.slippage,rollover:S.rollover,basket:S.basket,run_scope:S.run_scope,report_mode:S.report_mode,membership:S.membership,max_positions:S.max_positions,position_choice:S.position_choice });

// ---------------------------------------------------------------- shared bits
const GUILL = (() => { const paths = [[80, 23, 60, 23], [74, 19, 48, 19], [66, 29, 40, 29]].map(([R_, r, d, turns]) => { let p = ''; for (let k = 0; k <= 2400; k++) { const t = k / 2400 * Math.PI * 2 * turns, x = (R_ - r) * Math.cos(t) + d * Math.cos((R_ - r) / r * t), y = (R_ - r) * Math.sin(t) - d * Math.sin((R_ - r) / r * t); p += `${k ? 'L' : 'M'}${(100 + x * .7).toFixed(2)},${(100 + y * .7).toFixed(2)}`; } return `<path d="${p}"/>`; }); return `<svg viewBox="0 0 200 200">${paths.join('')}</svg>`; })();
// Card image: the author's portrait in black and white (img/gurus/<id>.jpg, sources in img/gurus/SOURCES.md);
// authors without a clearly identified public photo get a monogram tile, never a stand-in picture.
const PORTRAITS = new Set(['appel','bandy','bernstein','bollinger','bwilliams','chande','clayburg','cooper','covel','crabel','demark','elder','guppy','kaufman','landry','lebeau','livermore','lwilliams','minervini','momentum','oneil','primo','quant','radge','raschke','ruggiero','seasonal','sperandeo','swing12','swingstk','tharp','timers','weissman','wilder']);
const initials = n => n.replace(/\(.*?\)/g, '').split(/[\s&,]+/).filter(w => /^[A-Z]/.test(w)).slice(0, 2).map(w => w[0]).join('');
const photoOf = g => g.photo || (PORTRAITS.has(g.id) ? `img/gurus/${g.id}.jpg` : '');
const pframe = g => { const ph = photoOf(g);
  return ph ? `<div class="pframe portrait"><img src="${ph}" alt="${esc(g.name)}" loading="lazy"><span>${initials(g.name)}</span></div>`
            : `<div class="pframe mono"><span>${initials(g.name)}</span></div>`; };
const gTag = g => GURUS_API.find(a => a.id === g.id)?.tagline || g.tagline || '';
let HOME_STATS = {};
window.openGuru = (id, ev) => {
  if (ev) { ev.preventDefault(); ev.stopPropagation(); }
  const g = LIB.find(x => x.id === id); if (!g) return;
  const gs = HOME_STATS[id], entries=authorLibraryRecords(id),entryCounts=libraryEntryCounts(id),books=entries.slice(0,20);
  closeGuru();
  const el = document.createElement('div'); el.className = 'gmodal'; el.setAttribute('role', 'dialog'); el.setAttribute('aria-modal', 'true'); el.setAttribute('aria-label', g.name);
  el.innerHTML = `<div class="gmodal-card"><button type="button" class="gm-x" aria-label="Close" onclick="closeGuru()">×</button>
    <div>${pframe(g)}</div>
    <div><h2>${esc(g.name)}</h2>${gTag(g) ? `<p class="gm-tag">${esc(gTag(g))}</p>` : ''}
      <h4>Books and reading sources · ${entryCounts.total} indexed entries</h4>
      <ul>${books.map(b => `<li><b>${esc(b.title)}</b> <small>(${esc(String(b.year||'Edition unconfirmed'))})</small>${b.kind==='source'?'<span>Reading source · rules and performance reviewed separately</span>':b.book?.blurb?`<span>${esc(b.book.blurb)}</span>`:''}</li>`).join('')}</ul>${entries.length>books.length?`<p>${entries.length-books.length} more entries in this author’s library.</p>`:''}
      <dl class="lc-stats"><div><dt>Books</dt><dd>${entryCounts.total}</dd></div><div><dt>Strategies</dt><dd>${currentLibraryProgress().counts(g.books.map(b=>b.id)).strategies}</dd></div><div><dt>Backtested</dt><dd>${currentLibraryProgress().counts(g.books.map(b=>b.id)).backtested}</dd></div></dl>
      <div class="gm-foot"><a class="dbtn primary" href="${gs?.href||guruLink(g.id)}" onclick="closeGuru()">Open the vault</a><button type="button" class="dbtn ghost" onclick="closeGuru()">Close</button></div></div></div>`;
  el.addEventListener('click', e => { if (e.target === el) closeGuru(); });
  document.body.appendChild(el); document.body.style.overflow = 'hidden'; el.querySelector('.gm-x').focus();
};
window.closeGuru = () => { document.querySelectorAll('.gmodal').forEach(m => m.remove()); document.body.style.overflow = ''; };
document.addEventListener('keydown', e => { if (e.key === 'Escape') closeGuru(); });
async function heroStats(guru, book) {
  const tk = S.tk === 'ALL' || S.market !== 'india' ? TICKERS[0].ticker : S.tk, sum = await getSum(tk,S.from||'',S.to||'');
  const inScope = v => !isYt(v) && (!guru || v.guru === guru) && (!book || v.book === book);
  const st = CAT.filter(v => v.status === 'live' && inScope(v)).map(v => ({ v, s: sum[v.id]?.stats })).filter(x => TRUST.hasResult(sum[x.v.id]));
  const best = st.filter(x => x.s.trades >= 20 && Number.isFinite(x.s.sharpe)).sort((a, b) => b.s.sharpe - a.s.sharpe)[0];
  return { tk, fams: new Set(CAT.filter(inScope).map(v => v.family)).size, tested: new Set(st.map(x => x.v.family)).size, booksTested:new Set(st.map(x=>x.v.book)).size, variants: st.length, sig: st.filter(x => x.s.t_stat >= 2).length, best };
}

function libraryInstrumentHTML(){const indices=S.region==='us'?TICK_US:TICK_IN;return `<label class="library-sort library-context">Test data <select id="library-instrument" aria-label="Backtest data"><optgroup label="Indices">${(indices||[]).map(t=>`<option value="i:${esc(t.ticker)}" ${!S.basket&&S.tk===t.ticker?'selected':''}>${esc(LABEL[t.ticker]||t.ticker)}</option>`).join('')}</optgroup><optgroup label="Stock baskets — independent symbol">${BASKETS.map(b=>`<option value="b:${esc(b.id)}" ${S.basket===b.id?'selected':''} ${!b.symbols?.length?'disabled':''}>${esc(b.name)}</option>`).join('')}</optgroup></select></label>${S.basket?`<label class="library-sort library-symbol">Symbol <select id="library-stock" aria-label="Backtest stock">${(BASKETS.find(b=>b.id===S.basket)?.symbols||[]).map(t=>`<option value="${esc(t)}" ${t===S.tk?'selected':''}>${esc(LABEL[t]||t.split(':').at(-1))}</option>`).join('')}</select></label>`:''}`;}
function librarySearchHTML(){return `<form class="library-search" id="library-search-form" role="search"><svg viewBox="0 0 24 24" class="ic ic-line" aria-hidden="true"><circle cx="10.5" cy="10.5" r="7.5"/><path d="m16 16 5 5"/></svg><label class="sr-only" for="library-query">Search books and authors</label><input id="library-query" type="search" value="${esc(S.library_q)}" placeholder="Search a book, author or title…" autocomplete="off"><button type="submit">Search <span aria-hidden="true">↗</span></button></form>`;}
function libraryFiltersHTML(authorOnly=false){return `<section class="library-controls" aria-label="Library filters">${!authorOnly?`<div class="join library-mode" role="group" aria-label="Browse library">${[['authors','Authors'],['books','Books & reading']].map(([key,label])=>`<button class="join-item ${S.library_view===key?'active':''}" data-library-set="${ds({library_view:key,library_sort:'default',library_limit:'24',library_page:'1',source_page:'1'})}">${label}</button>`).join('')}</div>`:''}<div class="library-categories" role="group" aria-label="Book category">${[['','All books'],...Object.entries(CATEGORY_LABELS)].map(([key,label])=>`<button class="category-filter ${S.book_category===key?'active':''}" data-library-set="${ds({book_category:key,library_limit:'24',library_page:'1',source_page:'1'})}" aria-pressed="${S.book_category===key}">${label}</button>`).join('')}</div>${!authorOnly?`<div class="library-market">${regionToggle()}</div>${libraryInstrumentHTML()}<label class="library-sort">Sort <select id="library-sort"><option value="default" ${S.library_sort==='default'?'selected':''}>Catalogue order</option><option value="name" ${S.library_sort==='name'?'selected':''}>${S.library_view==='books'?'Title':'Author'} A–Z</option><option value="name-desc" ${S.library_sort==='name-desc'?'selected':''}>${S.library_view==='books'?'Title':'Author'} Z–A</option>${S.library_view==='books'?`<option value="author" ${S.library_sort==='author'?'selected':''}>Author A–Z</option><option value="newest" ${S.library_sort==='newest'?'selected':''}>Newest edition</option>`:`<option value="books" ${S.library_sort==='books'?'selected':''}>Most library entries</option><option value="available" ${S.library_sort==='available'?'selected':''}>Available backtests</option>`}</select></label>`:''}</section>`;}
function libraryEmpty(){return '<div class="empty library-empty"><b>No matching books or authors</b>Try another title, author or category.<button class="dbtn sm" data-library-set="library_q=&book_category=&library_limit=24&library_page=1&source_page=1">Clear search and filters</button></div>';}
function libraryPagination(total,page,key,label){
  const pages=Math.ceil(total/24);if(pages<2)return '';
  return `<nav class="library-load" aria-label="${esc(label)} pages"><button class="dbtn sm" data-library-set="${key}=${page-1}" ${page<=1?'disabled':''}>← Previous</button><span>Page ${page} of ${pages} · ${total} entries</span><button class="dbtn sm" data-library-set="${key}=${page+1}" ${page>=pages?'disabled':''}>Next 24 ${esc(label)} →</button></nav>`;
}
function authorResultHTML(authors,card,live,scoped){
  const liveIds=new Set(live.map(g=>g.id)),positions=new Map(LIB.map((g,k)=>[g.id,k])),list=[...authors];
  if(S.library_sort==='name'||S.library_sort==='name-desc')list.sort((a,b)=>a.name.localeCompare(b.name)*(S.library_sort==='name'?1:-1));
  else if(S.library_sort==='books')list.sort((a,b)=>libraryEntryCounts(b.id).total-libraryEntryCounts(a.id).total||a.name.localeCompare(b.name));
  else if(S.library_sort==='available')list.sort((a,b)=>(scoped[b.id]?.tested||0)-(scoped[a.id]?.tested||0)||a.name.localeCompare(b.name));
  else list.sort((a,b)=>Number(liveIds.has(b.id))-Number(liveIds.has(a.id))||positions.get(a.id)-positions.get(b.id));
  const page=Math.max(1,Math.min(Math.ceil(list.length/24)||1,+S.library_page||1)),start=(page-1)*24;
  return `<div class="library-result-count"><h2 class="section-h">${S.library_q?'Search results':'Authors in the library'}</h2><span>${list.length} ${list.length===1?'author / source group':'authors / source groups'}${S.library_q?` matching “${esc(S.library_q)}”`:''}${S.book_category?` · ${esc(CATEGORY_LABELS[S.book_category])}`:''}</span></div>${list.length?`<div class="gallery">${list.slice(start,start+24).map((g,i)=>card(g,start+i,liveIds.has(g.id))).join('')}</div>${libraryPagination(list.length,page,'library_page','authors')}`:libraryEmpty()}`;
}
function libraryBookResults(records){
  const found=VaultLibraryBrowser.search(records,{query:S.library_q,category:S.book_category,sort:S.library_sort}),page=Math.max(1,Math.min(Math.ceil(found.length/24)||1,+S.library_page||1)),start=(page-1)*24;
  return `<div class="library-result-count"><h2 class="section-h">Books &amp; reading sources</h2><span>${found.length} indexed entries${S.library_q?` matching “${esc(S.library_q)}”`:''}</span></div>${found.length?`<div class="library-book-grid">${found.slice(start,start+24).map(libraryBookCard).join('')}</div>${libraryPagination(found.length,page,'library_page','books')}`:libraryEmpty()}`;
}
function libraryBookLink(record){
  if(record.kind!=='catalogue')return href('/source/'+encodeURIComponent(record.id),{guru:record.authorId});
  const owner=record.book?.guru?.id||record.authorId;
  const rows=currentLibraryProgress().measured.filter(row=>row.book_id===record.id);
  const selected=rows.find(row=>row.instrument===S.tk&&(row.market==='us'?'us':'in')===S.region),row=selected||rows[0];
  if(!row)return href('/guru/'+owner,{view:'grid',book:record.id,tk:S.tk,basket:S.basket,from:S.from,to:S.to});
  const region=row.market==='us'?'us':'in',basket=row.instrument.startsWith('basket:')?row.instrument.split(':')[1]:'';
  return href('/guru/'+owner,{view:'grid',book:record.id,region,library_market:region,tk:row.instrument,basket,...measuredWindow(row),cost:row.execution?.cost||'0',slippage:row.execution?.slippage||'0',rollover:row.execution?.rollover||'0',research_mode:row.execution?.research_mode||'',run_scope:'single'});
}
function libraryBookCard(record){const b=record.book||record.source||record,link=libraryBookLink(record),isRule=record.categories.includes('rule-based'),progress=recordProgress(record);return `<article class="library-book" data-library-book="${esc(record.id)}"><span class="library-book-kind">${record.kind==='catalogue'?'Catalogue book entry':esc(b.entry_kind_label||'Reading source · identity review')}</span><span class="book-progress progress-${progress.status}" title="${esc(progress.reason)}">${progress.label}</span><p class="library-book-author">${esc(record.kind==='source'&&record.source?.author_metadata&&record.source.author_metadata!=='?'?record.source.author_metadata+' · '+record.authorName:record.authorName)}</p><h3><a href="${link}">${esc(record.title||'Title not recorded')}</a></h3>${categoryTags(b,record.kind)}<p class="book-progress-reason">${esc(progress.reason)}</p>${record.kind==='catalogue'?`<dl class="bc-stats"><div><dt>Strategies</dt><dd>${progress.strategies}</dd></div><div><dt>Backtested</dt><dd>${progress.backtested}</dd></div></dl>`:''}<p class="library-book-summary">${esc(record.kind==='catalogue'?readingSummary(record.author||GURU[record.authorId],b):b.summary||'Original reading material indexed from the source vault. Content and objective rules still need review.')}</p><div class="library-book-foot"><span>${record.year||'Edition year unconfirmed'}</span><a href="${link}">${isRule&&b.live?'Open book & rules':'Open reading page'} <span aria-hidden="true">↗</span></a></div></article>`;}
function readingBookPage(g,b,source=null){if(!g||!b)return '<div class="empty"><b>Reading entry unavailable</b>This source is not in the current index.</div>';const title=b.title||'Reading source',sources=source?[source]:bookSources(g,b),links=sourcePdfLinks(sources),credit=b.author_credit||source?.source_identity?.credit||source?.author_metadata||(source?(b.source_author&&b.source_author!=='?'?`Recorded source author: ${b.source_author}`:'Individual author unconfirmed'):g.name);return `<nav class="crumbs"><a href="#/">Guru’s</a><span>/</span>${GURU[g.id]?`<a href="${guruLink(g.id)}">${esc(g.name)}</a>`:`<span>${esc(g.name)}</span>`}<span>/</span><span>${esc(title)}</span></nav><section class="box reading-page" data-reading-page="${esc(b.id)}"><div class="reading-book-cover" aria-hidden="true"><span>${esc(credit)}</span><strong>${esc(title)}</strong><small>${source?(source.source_identity?.year||'Source edition unconfirmed'):bookYear(b)}</small></div><div class="reading-book-copy"><span class="olabel">Reading library</span><h1>${esc(title)}</h1><p class="reading-credit">${esc(credit)} · ${source?(source.source_identity?.year||'Edition year unconfirmed'):bookYear(b)}</p>${categoryTags(b,source?'source':'catalogue')}<p class="lede">${esc(source?b.summary||'This reading source is indexed from the vault. A detailed content summary and mechanical rule review are still pending.':readingSummary(g,b))}</p><div class="reading-status"><strong>Read the source</strong><p>${g.id==='pristine'?(b.id==='p_tpm1'&&pristineReviewedSource()?esc(pristineSourceNote):'This Pristine reading source has no separately admitted complete trading protocol. Selected TPM1 branches do not certify other sources.'):'This page is for reading and source review. Topic classification does not establish a backtested trading system.'} No performance figures are attributed to this reading entry.</p></div>${b.publisher_source?`<p><a class="olink" href="${esc(b.publisher_source)}" target="_blank" rel="noopener">Publisher’s book record</a></p>`:''}${links||'<p class="rnote">Data missing: a verified local reading copy is not available for this entry.</p>'}</div></section>${readingReviewHTML(source||bookSources(g,b).find(x=>x.review))}${source?`<section class="box"><div class="box-h">Source record</div><p>${esc(source.count_basis||'An indexed source-title candidate, not a certified count of distinct authored books.')}</p><ul>${(source.files||[]).map(f=>`<li>${esc(f.filename)}</li>`).join('')}</ul></section>`:''}`;}
function sourceReadingPage(){const record=libraryRecords().find(r=>r.kind==='source'&&r.id===R.id),indexed=SOURCE_ROW_BY_ID.get(R.id);return record?readingBookPage(record.author||{id:record.authorId,name:record.authorName},record.source,record.source):indexed?readingBookPage(indexed.author,indexed.source,indexed.source):'<div class="empty"><b>Reading source unavailable</b>This source is not in the current index.</div>';}

// ---------------------------------------------------------------- 1. landing
let PV = true;
window.togglePV = () => { PV = !PV; render(); };
async function viewHome() {
  const h = await heroStats(), readiness=await getReadiness(h.tk), allRecords=libraryRecords(), visibleAuthors=VaultLibraryBrowser.matchingAuthors(LIB,allRecords,S.library_q,S.book_category), live = LIB.filter(g => g.books.some(b => b.live)), pre = LIB.filter(g => !g.books.some(b => b.live));
  const implementedFamilies=new Set(CAT.filter(v=>v.status==='live' && v.guru!=='ross' && !isYt(v)).map(v=>v.family)).size;
  const scoped=Object.fromEntries(await Promise.all(live.map(async g=>[g.id,await heroStats(g.id)])));
  HOME_STATS = scoped;
  const awaitedSummary=await getSum(h.tk),cardStates=Object.fromEntries(LIB.map(g=>[g.id,VaultLibraryDiscovery.authorAvailability(g.books.map(b=>b.id),CAT,awaitedSummary,readiness)]));
  pristineAuthorStatus(cardStates.pristine);
  const card = (g,k,isLive) => {
    const gs=isLive?scoped[g.id]:null,state=cardStates[g.id],entryCounts=libraryEntryCounts(g.id);
    const matching=VaultLibraryBrowser.search(authorLibraryRecords(g.id),{query:S.library_q,category:S.book_category}),preview=matching.length?matching.slice(0,2):(g.books.some(b=>b.live)?g.books.filter(b=>b.live):g.books).slice(0,2);
    return `<a href="${href('/guru/'+g.id,{view:'grid',book_category:S.book_category,tk:S.tk,basket:S.basket})}" class="legend-card live" data-library-author="${g.id}" data-author-availability="${state.status}">
      <div class="lc-top"><span class="no">${String(k+1).padStart(2,'0')} / ${isLive?'Strategy research':'Reading library'}</span><span class="keyhole" aria-hidden="true"></span></div>
      <div class="lc-body">${pframe(g)}<div><h3>${esc(g.name)}</h3><div class="lbook">${esc(preview.map(b=>b.title).join(' · '))}${matching.length>2?` · +${matching.length-2} more`:''}</div></div></div>
      <p class="lc-tag">${esc(gTag(g))}</p><button type="button" class="lc-more" onclick="openGuru('${g.id}', event)">About this author</button>
      <dl class="lc-stats"><div><dt title="${entryCounts.catalogue} catalogue entries and ${entryCounts.reading} additional reading sources. Articles, editions and course material are identified in the source record; this is not a count of distinct published books.">Library entries</dt><dd data-library-entry-count="${g.id}">${entryCounts.total}</dd></div><div><dt>Strategies</dt><dd>${gs?.fams??new Set(CAT.filter(v=>v.guru===g.id).map(v=>v.family)).size}</dd></div><div><dt title="Admitted results on the currently selected instrument; not source certification">Backtests available</dt><dd>${state.availableFamilies}</dd></div></dl>
      <p class="lc-coverage availability-copy" title="${esc(state.text)}">${esc(state.text)}</p>
      <div class="lc-foot">${gs?.best?`<span class="lc-sharpe" title="Highest modeled Sharpe on ${esc(LABEL[gs.tk])}: ${esc(familyDisplayName(gs.best.v))}. Calculated result; original book performance is not certified.">Sharpe <strong>${fx(gs.best.s.sharpe)}</strong></span>`:`<span class="lc-status status-${state.status}">${esc(state.title)}</span>`}<b class="lc-open">Open the vault <span aria-hidden="true">↗</span></b></div></a>`;
  };
  return `<section class="lhero"><div class="guilloche" aria-hidden="true">${GUILL}</div><h1>Guru's</h1><p class="lede">Trading books, their strategy rules, and research on India and US markets. Backtests appear only where the required data and calculation support are available.</p>
      ${librarySearchHTML()}<div class="hero-cta"><a class="dbtn primary" href="${href('/strategies',{tk:S.tk,basket:S.basket})}">Browse all strategies</a><a class="dbtn" href="data/research-share-book-inventory.xlsx" download>Download book inventory</a><a class="dbtn ghost" href="${guruLink('connors')}">Larry Connors</a></div></section>
    <dl class="lstrip"><div><dt>Authors / source groups</dt><dd data-count="${LIB.length}">${LIB.length}</dd></div><div><dt>Library entries</dt><dd data-count="${allRecords.length}">${allRecords.length}</dd></div><div><dt>Backtests on ${esc(LABEL[h.tk]||h.tk)}</dt><dd data-count="${h.tested}">${h.tested}</dd></div><div><dt>${S.basket?'Basket symbols':'Indices'}</dt><dd data-count="${TICKERS.length}">${TICKERS.length}</dd></div></dl>
    ${libraryFiltersHTML()}
    <div id="library-results" aria-live="polite">${S.library_view==='books'?libraryBookResults(allRecords):authorResultHTML(visibleAuthors,card,live,scoped)}</div>
    <p class="rnote">Library entries include books, source editions, articles and course materials; file counts and confirmed published-book counts remain separate. Topic categories help you choose what to read. Rule-based classification does not certify complete rules, original book results or portfolio execution. Backtest availability is for the selected instrument.</p>`;
}

// ---------------------------------------------------------------- 2. filter navbar (dropdowns with live counts)
async function counts(k) {
  if (S.market !== 'india' || S.tab === 'awaiting') return {};
  if (k === 'tk') { const c = {}; for (const [t, l] of groupBy(await rows({ ...S, tk: 'ALL' }), r => r.tk)) c[t] = distinct(l, r => r.v.family); return c; }
  if (k === 'min') { const b = await rows({ ...S, min: '0' }), c = {}; for (const n of ['10', '30', '100']) c[n] = distinct(b.filter(r => r.s.trades >= +n), r => r.v.family); return c; }
  const f = { guru: r => r.v.guru, book: r => r.v.book, style: r => r.v.style, dir: r => r.v.dir, hold: r => holdOf(r.s), verdict: r => verdictOf(r.s).k }[k]; if (!f) return {};
  const c = {}; famBest(await rows({ ...S, [k]: '', ...(k === 'guru' && R.name !== 'guru' ? { book: '' } : {}) })).forEach(r => { const v = f(r); c[v] = (c[v] || 0) + 1; }); return c;
}
const FILTERS = {
  mode: { t: 'Browse', noAll: true, vals: () => Object.entries(MODES).filter(([k]) => R.name !== 'yt' || ['strategies', 'variants'].includes(k)) },
  tk: { t: 'Market', noAll: true, cur: () => S.market !== 'india' ? '@' + S.market : S.tk, vals: () => [...TICKERS.map(t => [t.ticker, LABEL[t.ticker]]), ['ALL', isUS() ? 'All US indices' : 'All Indian indices'], ['@global', 'Global · soon'], ['@stocks', 'Single stocks · soon']], set: v => v.startsWith('@') ? { market: v.slice(1) } : { market: 'india', tk: v } },
  tab: { t: 'Rank by', noAll: true, vals: () => Object.entries(TABS).filter(([k])=>PUB || k!=='working').map(([k, x]) => [k, x.name]), set: v => ({ tab: v, sort: 'primary' }) },
  guru: { t: 'Guru', vals: () => LIB.map(g => [g.id, g.name+(g.id==='ross'?' · Source research':'')]), set: v => ({ guru: v, book: '' }) },
  book: { t: 'Book', vals: () => Object.values(BOOK).filter(b => !S.guru || b.guru.id === S.guru).map(b => [b.id, b.title + (b.live ? '' : ' · soon')]) },
  style: { t: 'Style', vals: () => [...new Set(CAT.map(v => v.style))].map(x => [x, x]) },
  dir: { t: 'Direction', vals: () => [['Long', 'Long'], ['Short', 'Short']] },
  hold: { t: 'Holding', vals: () => Object.entries(HOLD) },
  verdict: { t: 'Verdict', vals: () => Object.entries(VERD) },
  min: { t: 'Trades', zero: '0', vals: () => [['10', '≥ 10 trades'], ['30', '≥ 30 trades'], ['100', '≥ 100 trades']] },
};
const curOf = k => FILTERS[k].cur ? FILTERS[k].cur() : S[k];
const isSet = k => { const c = curOf(k), F = FILTERS[k]; return !F.noAll && c && c !== (F.zero || ''); };
const setFor = (k, v) => { const F = FILTERS[k]; if (!F.noAll && curOf(k) === v) return { [k]: F.zero || '' }; return F.set ? F.set(v) : { [k]: v }; };
function dropdown(k, cnt) {
  const F = FILTERS[k], c = curOf(k), on = F.vals().find(([v]) => v === c), set = isSet(k) || F.noAll;
  const items = (F.noAll ? '' : `<li><a class="${isSet(k) ? '' : 'active'}" data-set="${ds({ [k]: F.zero || '' })}"><span>All</span></a></li>`) +
    F.vals().map(([v, t]) => { const n = cnt?.[v]; return `<li><a class="${c === v ? 'active' : ''} ${cnt && !n && !v.startsWith('@') && v !== 'ALL' && Object.keys(cnt).length ? 'muted' : ''}" data-set="${ds(setFor(k, v))}"><span>${esc(t)}</span>${n != null ? `<span class="badge">${n}</span>` : ''}</a></li>`; }).join('');
  return `<details class="dd ${isSet(k) ? 'set' : ''}"><summary class="dd-btn"><span class="dd-l">${F.t}</span>${set && on ? `<b>${esc(on[1])}</b>` : ''}<i class="chev"></i></summary><ul class="menu">${items}</ul></details>`;
}
async function filterBar() {
  const keys = ['mode', 'tk', 'tab', ...(R.name === 'guru' || R.name === 'yt' ? [] : ['guru']), ...(R.name === 'yt' ? [] : ['book']), 'style', 'dir', 'hold', 'verdict', 'min'];
  const cnt = {}; for (const k of keys) if (!['mode', 'tab'].includes(k)) cnt[k] = await counts(k);
  const chips = ['guru', 'book', 'style', 'dir', 'hold', 'verdict', 'min'].filter(k => isSet(k) && !(R.name === 'guru' && k === 'guru')).map(k => { const F = FILTERS[k], c = curOf(k); return `<span class="chip">${F.t}: ${esc(F.vals().find(([v]) => v === c)?.[1] || c)}<button data-set="${ds({ [k]: F.zero || '' })}" aria-label="Remove">×</button></span>`; });
  if (S.family) chips.push(`<span class="chip">Strategy: ${esc((CAT.find(v => v.family === S.family)?familyDisplayName(CAT.find(v => v.family === S.family)):''))}<button data-set="${ds({ family: '', mode: 'strategies' })}" aria-label="Remove">×</button></span>`);
  if (S.q) chips.push(`<span class="chip">“${esc(S.q)}”<button data-set="${ds({ q: '' })}" aria-label="Remove">×</button></span>`);
  return `<div class="fbar"><div class="fbar-in">${keys.map(k => dropdown(k, cnt[k])).join('')}<label class="dsearch"><input id="q" type="search" value="${esc(S.q)}" placeholder="Search name, rule, book…"></label></div>
    ${chips.length ? `<div class="chips">${chips.join('')}<button class="link" data-set="${ds({ q: '', ...(R.name === 'guru' ? {} : { guru: '' }), book: '', style: '', dir: '', hold: '', verdict: '', min: '0', family: '' })}">Clear all</button></div>` : ''}</div>`;
}
const regionToggle = (withAll = false) => `<div class="join region" title="India: Nifty 50, Bank Nifty, Midcap 100 (NSE daily close) · US: S&amp;P 500, Nasdaq 100, Dow Jones (daily close)${withAll ? ' · All: both markets together' : ''}">${[...(withAll && TICK_US ? [['all', 'All']] : []), ['in', 'India'], ['us', 'US']].map(([k, t]) => `<button class="join-item ${S.region === k ? 'active' : ''} ${k === 'us' && !TICK_US ? 'off' : ''}" data-set="${ds({ region: k,basket:'', tk: k === 'all' ? 'ALL' : '', from: '', to: '', market: 'india' })}" ${k === 'us' && !TICK_US ? 'disabled title="US engine (port 8091) is not running"' : ''}>${t}</button>`).join('')}</div>`;
function resultsBar(rs) {
  const M = TABS[S.tab].metric;
  return `<div class="rbar" data-results-controls>
    ${regionToggle(R.name === 'all')}<label class="dselect"><select data-key="sort" aria-label="Sort">${Object.entries(SORTS).map(([k, t]) => `<option value="${k}" ${S.sort === k ? 'selected' : ''}>Sort: ${k === 'primary' ? (M?.label || t) : t}</option>`).join('')}</select></label>
    <div class="join">${[['grid', 'Grid'], ['table', 'Table'], ['list', 'List']].map(([k, t]) => `<button class="join-item ${S.view === k ? 'active' : ''}" data-set="${ds({ view: k })}"><i class="ico ico-${k}"></i>${t}</button>`).join('')}</div></div>`;
}
function resultsHtml(rs, missing = []) {
  if (S.market !== 'india') return `<div class="empty"><b>${S.market === 'global' ? 'Global markets' : 'Single stocks'}: no data loaded yet</b>The same gurus, books and strategies appear here once that data is in the Rust backend.</div>`;
  if (R.name === 'yt' && S.family && !S.grp) { const mv = ytMainVariants(ALLCAT, S.family); if (mv.length) return ytGroupCards(mv); }
  const pv = S.book && !BOOK[S.book].live ? BOOK[S.book].title : S.guru && !GURU[S.guru].books.some(b => b.live) ? GURU[S.guru].name : '';
  if (pv && !rs.length) return `<div class="empty"><b>${esc(pv)}: not converted yet</b>Its strategies, stats and charts appear here once its rules are coded in Rust.</div>`;
  if (!rs.length && missing.length) return viewStrategies([],S.mode==='variants',missing);
  if (!rs.length && R.name === 'yt' && S.tab !== 'awaiting') { const pend = ytUniverseCards(S.mode === 'variants') + ytPendingCards(S.mode === 'variants'); if (pend) return `<div class="grid yt-universe-grid">${pend}</div>`; }
  if (!rs.length && (S.mode === 'strategies' || S.mode === 'variants')) return `<div class="empty"><b>Nothing matches</b>Loosen a filter or press Clear all.</div>`;
  if (S.tab === 'awaiting') return S.book&&missing.length?`<div class="grid">${missing.map(unavailableBookCard).join('')}</div>`:awaiting(rs);
  return S.mode === 'gurus' ? viewGurus(rs) : S.mode === 'books' ? viewBooks(rs) : viewStrategies(rs, S.mode === 'variants',missing);
}

// ---------------------------------------------------------------- 3. author page and all-strategies page
async function viewList() {
  if(R.name==='guru'&&S.book&&BOOK[S.book]&&!BOOK[S.book].live)return readingBookPage(GURU[R.id],BOOK[S.book]);
  // Author shelves use their book statistics, never the ranked strategy rows.
  const rs = R.name==='guru'&&!S.book ? [] : await rows();
  let head;
  if (R.name === 'guru') {
    const g = GURU[R.id], api = GURUS_API.find(a => a.id === g.id), b = S.book && BOOK[S.book];
    head = `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span>${b ? `<a href="${guruLink(g.id)}">${esc(g.name)}</a><span>/</span><span>${esc(b.title)}</span>` : `<span>${esc(g.name)}</span>`}</nav>
      <section class="ghero"><div class="guilloche" aria-hidden="true">${GUILL}</div><div class="ghero-copy"><h1>${esc(b ? b.title : g.name)}</h1><p class="lede">${esc(b ? `${g.name} · ${bookYear(b)}` : api?.tagline || g.tagline || 'Reading library · objective strategy review pending')}</p>${b ? `<p class="gbook">${esc(g.id==='pristine'&&b.id==='p_tpm1'?readingSummary(g,b):(b.blurb || BOOK_BLURB[b.id] || ''))}</p>` : authorProfileHtml(g.id)}</div><figure class="guru-portrait"><div class="portrait ${g.id==='ross'?'portrait-ross':''}">${pframe(g)}</div>${g.photoSource?`<figcaption class="photo-credit">Photo: <a href="${esc(g.photoSource)}" target="_blank" rel="noopener noreferrer">Trading Educators</a></figcaption>`:''}</figure></section>
      <dl class="lstrip library-summary" data-author-counts><div><dt>Books</dt><dd>${b?1:libraryEntryCounts(g.id).total}</dd></div><div><dt>Strategies</dt><dd>${currentLibraryProgress().counts(b?[b.id]:g.books.map(x=>x.id)).strategies}</dd></div><div><dt>Backtested</dt><dd>${currentLibraryProgress().counts(b?[b.id]:g.books.map(x=>x.id)).backtested}</dd></div></dl>${b?`<p class="library-count-note" data-book-result-context>Backtested counts strategies with a result on any instrument, India and US together (same as the book card). All counts are strategies, never variants.</p>`:''}`;
  } else if (R.name === 'yt' && S.family) { const fname = familyDisplayName(ALLCAT.find(v => v.family === S.family) || {}), fl = href('/yt', { tk: S.tk, from: S.from, to: S.to, family: S.family });
    head = `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span><a href="${href('/yt', { tk: S.tk, from: S.from, to: S.to })}">YT Strategies</a><span>/</span>${S.grp ? `<a href="${fl}">${esc(fname)}</a><span>/</span><span>${esc(S.grp)}</span>` : `<span>${esc(fname)}</span>`}</nav><section class="ahero"><h1>${esc(S.grp || fname)}</h1><p class="lede">${esc(S.grp ? (MAIN_VARIANT_NOTES[S.grp] || '') : `${fname}: its main variants. Open one to see its sub-variants, each a separate rule set from the team's AFLs.`)}</p></section>`;
  } else if (R.name === 'yt') head = `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span><span>YT Strategies</span></nav><section class="ahero"><h1>YT Strategies</h1><p class="lede">Rule sets the team sourced outside the book library, backtested on the selected India or US instrument. These are simulated results; no published figures exist for them.</p></section>`;
  else head = `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span><span>All strategies</span></nav><section class="ahero"><h1>All strategies</h1><p class="lede">Explore the catalogued book rules on the selected India or US instrument. Available backtests and unavailable strategies are listed separately; book figures are reference claims.</p></section>`;
  if(S.from||S.to)head+=`<p class="rnote" data-list-period><b>Selected results period:</b> ${esc(S.from||'First available bar')} to ${esc(S.to||'Latest available bar')}. Top Sharpe results and linked backtests use this window.</p>`;
  if(R.name==='guru' && R.id==='chande' && S.book) head+=experimentNotice({guru:'chande'},null);
  if(R.name==='guru' && S.book) head+=readingReviewHTML(bookSources(GURU[R.id],BOOK[S.book]).find(x=>x.review));
  if(R.name==='guru'&&S.book&&!BOOK[S.book]?.live)return readingBookPage(GURU[R.id],BOOK[S.book]);
  if (R.name === 'guru' && !S.book) return `${head}${summaryNoticeHTML(listSummaryIssues())}${R.id==='ross'?rossResearchNotice():R.id==='chande'?experimentNotice({guru:'chande'},null):''}${libraryFiltersHTML(true)}${await bookShelf(GURU[R.id])}`;
  const missing=await unavailableForSelectedBook();
  return `${head}${await filterBar()}${summaryNoticeHTML(listSummaryIssues())}${resultsBar(rs)}<div id="results">${resultsHtml(rs,missing)}</div>`;
}
// A guru's books as covers; opening one shows its strategies.
function rossResearchNotice() {
  return `<section class="box" data-ross-review><div class="box-h">Source research — no implemented backtests <small>GPT’s source review · full corpus review in progress</small></div><p><b>Data missing:</b> actual traded futures history, contract identities and roll treatment, tick/point values and execution costs. Some methods also require intraday prices or volume. The loaded cash-index bars do not supply these inputs.</p><p><b>Rules unresolved:</b> entries require context or judgment, and exits, stops or sizing are not fully specified. More data alone does not produce a complete book-faithful strategy. Each method lists its own requirements below.</p><p>${esc(window.JoeRossEvidence.scope)}</p><p>${esc(window.JoeRossEvidence.finding)}</p><p>The original bounded review covered 125 PDF copies with 59 distinct files. The current source inventory below also includes later imports; neither file count establishes a count of distinct books. Sweet Chariot's originally supplied chapter is damaged. No verified Joe Ross US or Indian performance is published; open a reviewed method for its page references and precise blockers.</p></section>`;
}
function sourceBookShelf(g) {
  const audit=BOOK_COVERAGE.authors[g.id];if(!audit)return '';
  const extra=VaultLibraryBrowser.search(authorLibraryRecords(g.id).filter(r=>r.kind==='source'),{category:S.book_category,sort:S.library_sort});
  const counts=audit.counts||{},entryCounts=libraryEntryCounts(g.id),page=Math.max(1,Math.min(Math.ceil(extra.length/24)||1,+S.source_page||1)),start=(page-1)*24;
  return `<section class="box" data-book-coverage><div class="box-h">Books and reading sources <small>${esc(g.name)}</small></div><p><strong>${entryCounts.total} library entries</strong> · ${entryCounts.catalogue} catalogue entries · ${entryCounts.reading} additional reading sources. ${counts.source_title_candidates??audit.source_titles.length} source-title candidates · ${counts.local_pdf_files??0} local PDF files. Articles, course materials, editions and split chapters are identified separately; these counts do not establish distinct published books.</p><p>${esc(audit.note||'')}</p><p><a class="dbtn sm" href="data/book-coverage-by-author.csv" download>Download author coverage CSV</a>${g.id==='pristine'?'<a class="dbtn sm" href="data/pristine-reading-index.csv" download>Download related-source inventory</a>':''}</p><details open><summary>Reading sources (${extra.length})</summary><div class="shelf">${extra.slice(start,start+24).map(record=>{const b=record.source,link=href('/source/'+encodeURIComponent(b.id),{guru:g.id,tk:S.tk,basket:S.basket,from:S.from,to:S.to}),originals=sourcePdfLinks([b]);return `<article class="bookcard" data-reading-source="${esc(b.id)}"><div class="bc-body"><span class="olabel">${esc(b.entry_kind_label||'Reading source')} · ${esc(b.review?.status_label||'Source review pending')}</span><h3><a href="${link}">${esc(b.title)}</a></h3><p class="bc-blurb">${esc(b.summary||'This original reading source is indexed from the vault. Content and mechanical trading rules still need review; no backtest is attributed to it.')}</p>${categoryTags(b,'source')}<details><summary>Source record</summary><p>${esc(b.count_basis||'An indexed source entry; distinct bibliographic identity remains separate.')}</p>${b.author_metadata?`<p>Recorded author: ${esc(b.author_metadata)}</p>`:''}<p>${esc((b.years_reported||[]).join(', ')||'Edition year not established')}</p><ul>${(b.files||[]).map(f=>`<li>${esc(f.filename)}</li>`).join('')}</ul></details>${originals||'<p class="category-pending">Original reading copy pending in the local cache.</p>'}<a class="dbtn sm" href="${link}">Open reading page</a></div></article>`;}).join('')}</div>${libraryPagination(extra.length,page,'source_page','sources')}</details></section>`;
}
function authorBookCard(record){
  const b=record.book||record.source||record,link=libraryBookLink(record),progress=recordProgress(record),title=record.title||'Title not recorded';
  const backtested=record.kind==='catalogue'?progress.backtested||0:0; // India and US together, so book cards add up to the author total
  const year=record.year||'Edition unconfirmed',cloth=b.c||CLOTH[3],originals=record.kind==='catalogue'?bookSources(record.author||GURU[record.authorId],b):[b];
  return `<article class="bookcard" data-library-book="${esc(record.id)}"><a class="book-cover-link" href="${link}" aria-label="Open ${esc(title)}"><div class="cover3d ${b.live?'':'pv'}" style="--c:${esc(cloth)}"><div class="c3-spine"></div><div class="c3-face"><span class="c3-author">${esc(record.authorName)}</span><span class="c3-title">${esc(title)}</span><span class="c3-year">${esc(String(year))}</span></div></div></a>
    <div class="bc-body"><span class="olabel">${record.kind==='catalogue'?'Book / course':esc(b.entry_kind_label||'Reading source · identity review')} · ${esc(String(year))}</span><h3><a href="${link}">${esc(title)}</a></h3><span class="book-progress progress-${progress.status}" title="${esc(progress.reason)}">${progress.label}</span><p class="bc-blurb">${esc(record.kind==='catalogue'?readingSummary(record.author||GURU[record.authorId],b):b.summary||'Original reading source; content and mechanical rule review are pending.')}</p>${categoryTags(b,record.kind)}
    ${record.kind==='catalogue'?`<dl class="bc-stats"><div><dt>Strategies</dt><dd>${progress.strategies}</dd></div><div><dt>Backtested</dt><dd>${backtested}</dd></div></dl><p class="book-progress-reason">Backtested counts India and US results together.</p>`:''}<p class="book-progress-reason">${esc(progress.reason)}</p>${sourcePdfLinks(originals)}<a class="dbtn ${backtested?'primary ':''}sm" href="${link}">${record.kind==='catalogue'&&b.live?'Open book & rules':'Open reading page'}</a></div></article>`;
}
async function bookShelf(g) {
  const records=VaultLibraryBrowser.search(authorLibraryRecords(g.id),{category:S.book_category,query:S.library_q,sort:S.library_sort});
  const progress=currentLibraryProgress(),totals={backtested:0,in_progress:0,not_backtested:0};
  for(const record of records)totals[recordProgress(record).status]++;
  const filtered=records.filter(record=>!S.book_progress||recordProgress(record).status===S.book_progress);
  const page=Math.max(1,Math.min(Math.ceil(filtered.length/24)||1,+S.source_page||1)),start=(page-1)*24;
  return `<div class="shelfbar"><h2 class="section-h">Books</h2><a class="olink" href="${href('/strategies',{guru:g.id})}">Browse strategies</a></div>
    <p class="library-count-note">All indexed titles are included below. Editions, articles and courses retain their source labels. In progress means queued or incomplete review, not that every title is being processed now.</p>
    <div class="book-status-filters" role="group" aria-label="Book status">${[['','All books'],['backtested','Backtested'],['in_progress','In progress'],['not_backtested','Not Backtested']].map(([key,label])=>`<button class="category-filter ${S.book_progress===key?'active':''}" aria-pressed="${S.book_progress===key}" data-library-set="${ds({book_progress:key,source_page:'1'})}">${label} <span>${key?totals[key]:records.length}</span></button>`).join('')}</div>
    ${filtered.length?`<div class="shelf">${filtered.slice(start,start+24).map(authorBookCard).join('')}</div>${libraryPagination(filtered.length,page,'source_page','books')}`:'<div class="empty"><b>No books in this status</b>Choose All books to see the full collection.</div>'}`;
}

const BOOK_BLURB = { stc9: 'TradingMarkets Swing Trading College IX (2010) · 14 sessions: the 4 Keys, R3, TPS, 5x5x5, H10 and the market-timing rules (RSI, DMI, %b, VIX, end of month).' };

// ---------------------------------------------------------------- 4. one strategy (the main app's strategy panel, nothing else)
const RUNS = {}, OHLC = {}, FULL_OHLC={},PRISTINE_READY={};
const fullOhlc=(tk,api=API)=>{const key=api+'|'+tk;return FULL_OHLC[key]||(FULL_OHLC[key]=fetch(api+'/ohlc?'+new URLSearchParams({ticker:tk}),{signal:AbortSignal.timeout(30000)}).then(async r=>{if(!r.ok)throw Error('Full chart history unavailable');return r.json();}).catch(error=>{delete FULL_OHLC[key];throw error;}));};
const ohlc = (tk, tf = 'D') => OHLC[tk + tf] || (OHLC[tk + tf] = fetch(`data/ohlc-${tk.replace(/\s/g, '')}${tf === 'D' ? '' : '-' + tf}.json`).then(r => r.ok ? r.json() : null).catch(() => null));
let TF = 'D';
const runData = (id, tk, from = '', to = '', cost = '0', p = '', api = API,execution=VaultUI.costState(S)) => { const k = [api, id, tk, from, to, cost, p,execution.slippage,execution.rollover,...(pristineMethod(id,PRISTINE_BOOK)?[execution.research_mode||'']:[])].join('|');
  return RUNS[k] || (RUNS[k] = (async () => { const t0 = performance.now();
    try { const r = await fetch(enginePath(id,'run',api)+'?' + new URLSearchParams({ id, ticker: tk, from, to, cost, ...execution, ...Object.fromEntries(Object.entries(decodeP(p)).map(([k, v]) => ['p.' + k, v])) })); if (!r.ok) { delete RUNS[k]; return { error: await r.text() }; }
      const j = await r.json(); if(!pristineMethod(id,PRISTINE_BOOK))j._ms = performance.now() - t0; return j; } catch (e) { delete RUNS[k]; return { error: e.message }; } })()); };
// the period bar: dates, quick presets, cost, and a Run button (the Rust API recomputes the whole backtest for that window)
let BASKETS=[],BASKETS_IN=[],BASKETS_US=[],TICK_ALL_IN=[];
function basketChoices(){const past=S.membership==='past';return S.region==='us'?[['us_sp500','S&P 500'],['us_nasdaq100','Nasdaq 100'],['us_dow30','Dow Jones'],['us_russell2000','Russell 2000']].map(([id,name])=>[id+(past?'_past':''),name]):[['nse500','NSE 500'],['nifty50','Nifty 50'],['midcap100','Midcap100'],['allfo','All F&O']];}
function marketControls(v,tk){
 const allIndices=S.region==='us'?TICK_US:TICK_IN,unitMethod=unitExposureMethod(v.id,v.book),indices=unitMethod?(allIndices||[]).filter(t=>t.ticker===(S.region==='us'?'SPX':'NIFTY 50')):allIndices,controls=instrumentControls(v.instrument_policy,v.id,v.book),uni=isYt(v)&&universeRunnable(uniOf(v.id),v.id),uniOnly=uni&&isUniverseOnly(v.id),onUni=universeScope(v),portfolio=S.run_scope==='basket'||onUni,sourceAccountOnly=uni||v.instrument_policy?.execution_scope==='single_symbol_source_account'||unitMethod;
 const etfs=S.region==='us'&&controls.etf?TICK_ALL_US.filter(t=>['SPY','QQQ','MDY','IWM','SMH','DIA'].includes(t.ticker)):[];
 const stocks=stockChoices(S.region==='us'?TICK_ALL_US:TICK_ALL_IN,tk);
 const singleState=ticker=>({...S,run_scope:'single',basket:'',tk:ticker,from:'',to:'',only:'',uni_basket:'',uni_set:'',...(btstReference(v.id)?{p:'',slippage:'0'}:{})}); // BTST: the new index opens with its own reference settings
 const noSeries=t=>isYt(v)&&v.needs_series?.length&&t.coverage&&!t.coverage[v.needs_series[0]]; // YT series coverage is reported per ticker (BTST has no Midcap 100 series); other books are gated by readiness
 const fut=!!btstReference(v.id),tkName=t=>fut&&btstFuturesSymbol(t.ticker)||LABEL[t.ticker]||t.ticker; // BTST: the button names the futures series traded
 const indexButtons=(indices||[]).map(t=>uniOnly?`<button type="button" class="join-item" disabled title="${esc(familyDisplayName(v))} ${esc(v.variant||v.name)} runs on the F&O stock universe only">${esc(tkName(t))}</button>`:controls.index&&noSeries(t)?`<button type="button" class="join-item" disabled title="No ${esc(familyDisplayName(v))} data for this index">${esc(tkName(t))}</button>`:controls.index?`<a class="join-item ${!portfolio&&t.ticker===tk?'active':''}" href="${href(R.path,singleState(t.ticker))}">${esc(tkName(t))}</a>`:`<button type="button" class="join-item" disabled title="${esc(controls.note)}">${esc(tkName(t))}</button>`).join('');
 const basketButtons=basketChoices().map(([key,name])=>{const entry=BASKETS.find(b=>b.id===key),missing=!entry?.symbols?.length;return `<button type="button" class="join-item ${portfolio&&S.basket===key?'active':''}" data-basket="${key}" ${!controls.basket||missing?`disabled title="${esc(!controls.basket?controls.note:'No admitted constituent prices are available for this basket.')}"`:''}>${esc(name)}</button>`;}).join('');
 const stockPicker=controls.stock&&!uni?`<label class="dfield stock-picker"><span class="swl">Stock</span><select id="strategy-stock" aria-label="Stock"><option value="" ${portfolio||!tk.startsWith('basket:')?'selected':''}>Select a stock</option>${stocks.map(t=>`<option value="${esc(t.ticker)}" ${!portfolio&&t.ticker===tk?'selected':''}>${esc(t.symbol||t.ticker.split(':').at(-1))}</option>`).join('')}</select></label>`:'';
 const pastTooltip=S.region==='us'?'Includes current and former constituents.\nDate-specific membership remains unverified.':'Historical constituent lists were found.\nPrice admission and dated membership are under review.';
 return `<div class="sw research-controls" data-market-controls><div class="swg"><span class="swl">Market</span>${regionToggle()}</div><div class="swg"><span class="swl">${fut?'Index (futures)':'Index'}</span><div class="join sm">${indexButtons}</div></div>${uni?(()=>{const U=uniOf(v.id),mem=uniMembers(v.id),uniState={...S,run_scope:'universe',basket:'',tk:'NIFTY 50',from:'',to:'',p:'',slippage:'0',only:'',uni_basket:'',uni_set:''};
  // STOCK: one stock of the universe (the same rules on that stock alone); BASKET: the whole watchlist (C&P = current and past members)
  return `<label class="dfield stock-picker" data-yt-stock><span class="swl">Stock</span><select id="yt-stock" aria-label="Stock" ${mem.length?'':'disabled title="The engine did not list the stocks of this universe"'}><option value="">Select a stock</option>${mem.map(m=>`<option value="${esc(m)}" ${onUni&&S.only===m?'selected':''}>${esc(m)}</option>`).join('')}</select></label>
  <div class="basket-selector" data-universe-choice><div class="basket-heading"><span class="swl">Basket</span><div class="join sm membership-toggle" role="group" aria-label="Basket membership">${['current','past','cp'].map(k=>{const pk=uniPick(v.id),n=pk.b?(k==='cp'?pk.b.n_cp:k==='current'?pk.b.n_current:pk.b.n_past):0,tip={current:`Current members only (${n}): survivors, results look better`,past:`Former members only (${n}): stocks that left the list, results look worse`,cp:`Current and past members (${n}): the complete list, the team's reference`}[k];
    return `<button type="button" class="join-item ${onUni&&pk.set===k?'active':''}" data-uni-set="${k}" aria-pressed="${onUni&&pk.set===k}" title="${esc(tip)}" ${!pk.b||(k!=='cp'&&!pk.b.current)?'disabled':''}>${esc(UNI_SET[k])}</button>`;}).join('')}</div></div><div class="join sm">${[['nse500','NSE 500'],['allfo','All F&O']].filter(([key])=>uniDefaultBasket(v.id)!=='nse500'||key==='nse500'||ATH_FO_CHOICE.includes(v.id)).map(([key,label])=>{const b=uniBaskets(v.id).find(x=>x.key===key),pk=uniPick(v.id);
    return b?`<button type="button" class="join-item ${onUni&&!S.only&&pk.key===key?'active':''}" data-uni-basket="${key}" title="${esc(b.cp.replace(/\.tls$/,''))} (${b.n_cp}) · current ${b.n_current}${key===uniDefaultBasket(v.id)?' · the reference list of this sub-variant':''}">${esc(label)}</button>`:`<button type="button" class="join-item" disabled title="Not in this database">${esc(label)}</button>`;}).join('')}</div></div>`;})():''}${stockPicker}${etfs.length?`<div class="swg"><span class="swl">ETF</span><div class="join sm">${etfs.map(t=>`<a class="join-item ${!portfolio&&t.ticker===tk?'active':''}" href="${href(R.path,singleState(t.ticker))}">${esc(t.ticker)}</a>`).join('')}</div></div>`:''}${sourceAccountOnly?'':`<div class="basket-selector" data-selector-panel><div class="basket-heading"><span class="swl">Basket</span><div class="join sm membership-toggle" role="group" aria-label="Basket membership"><button type="button" class="join-item ${S.membership==='current'?'active':''}" data-membership="current" aria-pressed="${S.membership==='current'}" title="Source-listed current constituents; historical tests can have survivorship bias.">Current</button><button type="button" class="join-item ${S.membership==='past'?'active':''}" data-membership="past" aria-pressed="${S.membership==='past'}" title="${esc(pastTooltip)}">Past</button></div></div><div class="join sm">${basketButtons}</div></div>`}<div class="control-run"><button class="dbtn primary sm" ${isYt(v)&&btSchema(v).length?'type="button" data-bt-open':'type="submit" form="period"'}>Run backtest</button>${(v.status==='live'&&!portfolio)||v.id.startsWith('ytath_')?`<a class="dbtn sm back-btn" id="open-optimization" href="${href('/'+(R.optim?'s':'o')+'/'+encodeURIComponent(v.id),{...S})}">${R.optim?'Back to backtest':'Optimization'}</a>`:''}</div></div>${controls.note&&!uni?`<p class="instrument-note" data-instrument-note>${esc(controls.note)}</p>`:''}`;
}
function varyingControls(v,tk) {
 const stock=instrumentControls(v.instrument_policy,v.id,v.book).basket&&!(isYt(v)&&universeRunnable(uniOf(v.id),v.id)),presets=universeScope(v)?[]:YT_PRESETS[v.id]||[],cur=decodeP(S.p);
 return `<div class="sw variant-basket">${presets.length?`<div class="swg yt-presets"><span class="swl">Preset</span><div class="join sm">${presets.map((x,k)=>`<button type="button" class="join-item ${presetActive(cur,x)&&(!x.ticker||x.ticker===tk)?'active':''}" data-preset="${k}" title="${esc(x.note)}">${esc(x.label)}</button>`).join('')}</div></div>`:''}${stock?`<div class="swg position-controls"><span class="swl">Positions</span><div class="join sm" role="group" aria-label="Maximum simultaneous positions">${['10','20','custom'].map(n=>`<button type="button" class="join-item ${S.position_choice===n?'active':''}" data-position-choice="${n}" aria-pressed="${S.position_choice===n}">${n==='custom'?'Custom':n+' open'}</button>`).join('')}</div>${S.position_choice==='custom'?`<form id="position-limit" class="custom-position"><label>Max open <input name="max_positions" type="number" min="1" max="100" step="1" required value="${esc(S.max_positions)}"></label><button type="submit" class="dbtn sm">Apply</button><span role="status" id="position-error"></span></form>`:''}<small>${S.run_scope==='basket'?`1/${esc(S.max_positions)} per position · unused slots in cash`:'Applies when a basket is selected'}</small></div>`:''}</div>`;
}
function selectedSpan(tk){
 const basket=BASKETS.find(b=>b.id===S.basket),span=S.run_scope==='basket'&&basket?basketSpan((S.region==='us'?TICK_ALL_US:TICK_ALL_IN).filter(t=>basket.symbols?.includes(t.ticker))):null;
 if(span){const calendar=(S.region==='us'?TICK_US:TICK_IN)?.find(t=>t.ticker===(S.region==='us'?'SPX':'NIFTY 50'));if(calendar?.first&&calendar?.last)return {first:span.first>calendar.first?span.first:calendar.first,last:span.last<calendar.last?span.last:calendar.last};}
 return span||TICKERS.find(x=>x.ticker===tk)||TICKERS[0]||{first:'',last:''};
}
// Where the team's own reference AFL and trade sheet for a variant are (inside the team's family folder).
// Dot: green = reference trade sheet; yellow = optimization workbook / highest-return setting; red = neither.
function refDot(v, tk) {
  const r = ytFamilyReference(v?.id); if (!r) return '';
  const m = (R.name === 'strategy' ? universeScope(v) : isUniverseOnly(v.id)) ? universeTradesheet(r, R.name === 'strategy' ? decodeP(S.p) : {}) : mainTradesheet(r, tk, R.name === 'strategy' ? decodeP(S.p) : {});
  if (m) return `<i class="ref-dot ok" role="img" aria-label="Reference trade sheet available" title="Reference trade sheet: ${esc(m.file.split('/').at(-1))} · ${esc(m.sheet)}"></i>`;
  if (r.best) return `<i class="ref-dot best" role="img" aria-label="${esc(BEST_SETTING)}" title="${esc(BEST_SETTING)}: ${esc(r.best.file.split('/').at(-1))} · sheet ${esc(r.best.sheet.trim())} · ${esc(r.best.row)}"></i>`;
  if (r.hasOptimization || r.summary) return `<i class="ref-dot optim" role="img" aria-label="Optimization workbook available" title="Optimization workbook available in the ATH folder${r.summary ? `: ${esc(r.summary.file.split('/').at(-1))}` : ''}"></i>`;
  return `<i class="ref-dot no" role="img" aria-label="${esc(NO_TRADESHEET)}" title="${esc(NO_TRADESHEET)}${r.summary ? ' · team results cannot be run in the Vault yet' : ''}"></i>`;
}
// ---- "Run backtest" parameter window for YT strategies (AmiBroker Parameters style; param-dialog.ts) ----
// BTST: the AFL's own Param() rows in AFL order and labels (afl-params.ts), opening on the reference sheet's settings.
// Every row carries its baseline (data-bt-base) for "changed" marking and Reset; rows the engine cannot set are listed apart.
// BTST: plain labels and choices (param-dialog.ts PLAIN_*); other YT strategies keep their schema labels.
const btOpts = (p, plain) => (plain && plainOptions(p)) || paramOptions(p) || timeOptions(p);
const btFmt = (p, x, plain = false) => timeOptions(p) ? hhmm(x) : (btOpts(p, plain)?.find(o => o.value === x)?.label ?? String(+(+x).toFixed(4)));
function btControl(p, val, base, o = {}) {
  const id = `bt-${p.key}`, label = o.label || paramTitle(p);
  const opts = o.options || btOpts(p, o.plain);
  const ctl = opts
    ? `<select id="${id}" name="${esc(p.key)}">${opts.map(x => `<option value="${x.value}" ${x.value === val ? 'selected' : ''}>${esc(x.label)}</option>`).join('')}</select>`
    : `<input id="${id}" type="number" name="${esc(p.key)}" ${p.min >= 0 ? 'min="0"' : ''} step="any" value="${val}" inputmode="decimal" data-aflrange="${p.min}:${p.max}:${p.step}">`;
  const baseLabel = opts?.find(x => x.value === base)?.label ?? btFmt(p, base, o.plain);
  return `<div class="bt-row${val !== base ? ' changed' : ''}" data-bt-key="${esc(p.key)}" data-bt-base="${base}"><label for="${id}">${esc(label)}</label><div class="bt-ctl">${ctl}</div><small class="bt-def">default ${esc(baseLabel)}${opts ? '' : ` · AFL range ${p.min}–${p.max}, any value runs`}</small></div>`;
}
// the instrument named in the window's subtitle: the futures symbol a BTST run trades, the universe, else the index
function btInstrumentShort(v, tk, src) {
  if (universeScope(v)) return `${universeText(uniOf(v.id), 'badge', 'F&O stocks')} (${uniOf(v.id)?.symbols ?? ''})`;
  return btstInstrument(v.id, tk, src)?.symbol || LABEL[tk] || tk;
}
function btDialogHtml(v, tk) {
  const onUni = universeScope(v), btst = !!ytFamilyReference(v.id), schema = btSchema(v), f = document.getElementById('period'), ref = btRef(v, tk), afl = onUni ? universeAflRows(AFL_PARAMS[v.id]) : AFL_PARAMS[v.id];
  const base = paramValues(schema, ref?.params || {}), cur = { ...base, ...decodeP(S.p) }, sk = Object.fromEntries(schema.map(p => [p.key, p]));
  const from = f?.elements.from?.value || '', to = f?.elements.to?.value || '', slip = f?.elements.slippage?.value || S.slippage || '0', roll = f?.elements.rollover?.value || S.rollover || '0';
  const cost = +(S.slippage || 0), costBase = ref ? ref.cost : 0;
  const row = p => btControl(p, clampParam(p, cur[p.key]), base[p.key], {
    plain: btst,
    label: btst ? PLAIN_LABELS[p.key] : undefined,
    // This ATH variant's engine codes are 1=Daily, 0=Weekly, 2=Monthly.
    // Keep the displayed order intuitive without sending BTST's 1/2/3 codes.
    options: v.id === 'ytath_close_entry_on_daily_chart_exit_below' && p.key === 'flt_tf'
      ? [{value: 1, label: 'Daily'}, {value: 0, label: 'Weekly'}, {value: 2, label: 'Monthly'}]
      : undefined,
  });
  const dateRows = `<div class="bt-row"><label for="bt-from">From</label><div class="bt-ctl"><input id="bt-from" type="date" name="from" value="${esc(from)}" required></div></div>
        <div class="bt-row"><label for="bt-to">To</label><div class="bt-ctl"><input id="bt-to" type="date" name="to" value="${esc(to)}" required></div></div>`;
  const costRows = btst
    ? `<div class="bt-row${cost !== costBase ? ' changed' : ''}" data-bt-key="@cost" data-bt-base="${costBase / 100}"><label for="bt-cost">Brokerage (% a side)</label><div class="bt-ctl"><input id="bt-cost" type="number" name="@cost" min="0" max="1" step="0.01" value="${cost / 100}"></div><small class="bt-def">default ${costBase / 100}</small></div>` // sent as the Vault's slippage (bps a side, taken off returns; trade prices unchanged)
    : `<div class="bt-row"><label for="bt-slip">Slippage (bps a side)</label><div class="bt-ctl"><input id="bt-slip" type="number" name="slippage" min="0" step="0.1" value="${esc(slip)}"></div></div><div class="bt-row"><label for="bt-roll">Rollover (bps per roll)</label><div class="bt-ctl"><input id="bt-roll" type="number" name="rollover" min="0" step="0.1" value="${esc(roll)}"></div></div>`;
  // one data source only (user decision 2026-10-26): the main IEOD_FO database as stored; shown, not selectable
  const srcRow = sk.src ? `<div class="bt-row" data-bt-key="src" data-bt-base="1"><label>Data source</label><div class="bt-ctl"><input type="hidden" name="src" value="1"><span class="bt-fixed">IEOD_FO database (shared, as stored)</span></div><small class="bt-def">2007-01-02 to 2023-02-07</small></div>` : '';
  const keys = strategyKeys(schema, btst ? (afl || []).map(r => r.key) : []).filter(k => k !== 'sizing');
  // position sizing as two cards (like the Net Profit choice of the toolbar's Strategy settings)
  const sz = sk.sizing ? clampParam(sk.sizing, cur.sizing) : null, szBase = sk.sizing ? base.sizing : null;
  const card = (val, title, note) => `<label class="bt-card${sz === val ? ' selected' : ''}"><input type="radio" name="sizing" value="${val}" ${sz === val ? 'checked' : ''}><span><strong>${title}</strong><small>${note}</small></span></label>`;
  const sizing = sk.sizing ? `<fieldset class="bt-sec bt-sizing"><legend>Position sizing</legend><div class="bt-row wide bt-cards${sz !== szBase ? ' changed' : ''}" data-bt-key="sizing" data-bt-base="${szBase}">${card(2, 'Fixed quantity', 'The same amount in every trade; profits are not reinvested.')}${card(1, 'Compounding', 'Each trade is a share of current equity; profits are reinvested.')}</div></fieldset>` : '';
  const body = `${sizing}<fieldset class="bt-sec"><legend>Period and costs</legend>${dateRows}${costRows}${srcRow}</fieldset><fieldset class="bt-sec"><legend>Strategy settings</legend>${keys.map(k => row(sk[k])).join('')}</fieldset>`;
  return `<dialog id="bt-dialog" class="bt-dialog" aria-labelledby="bt-title"><form id="bt-form" method="dialog">
    <header class="bt-head"><div><h2 id="bt-title">Strategy settings</h2><p class="bt-sub">${esc(v.variant || v.name)} · ${esc(v.dir || 'Long')} · ${esc(btInstrumentShort(v, tk, cur.src))}</p></div><button type="button" class="bt-x" data-bt-close aria-label="Close">×</button></header>
    <div class="bt-body">${body}</div>
    <p class="bt-err" id="bt-err" role="alert"></p>
    <footer class="bt-foot"><button type="button" class="dbtn sm" data-bt-reset>Reset to defaults</button><span></span><button type="button" class="dbtn sm" data-bt-close>Cancel</button><button type="submit" class="dbtn primary sm">Apply</button></footer>
    <p class="bt-hint">Apply keeps these settings; <b>Run backtest</b> runs them.</p>
  </form></dialog>`;
}
function btOpen() {
  if (!STRAT) return; document.getElementById('bt-dialog')?.remove();
  document.body.insertAdjacentHTML('beforeend', btDialogHtml(STRAT.v, STRAT.tk));
  const d = document.getElementById('bt-dialog'); btInactive(d); d.showModal ? d.showModal() : d.setAttribute('open', '');
}
function btInactive(d) { const vals = {}; d.querySelectorAll('.bt-row [name]').forEach(el => { if (el.name && el.type !== 'date' && !(el.type === 'radio' && !el.checked)) vals[el.name] = +el.value; }); const off = inactiveParams(vals); d.querySelectorAll('[data-bt-key]').forEach(r => r.classList.toggle('inactive', off.has(r.dataset.btKey))); }
function btMarkChanged(row) { row?.querySelectorAll('.bt-card').forEach(c => c.classList.toggle('selected', !!c.querySelector('input')?.checked)); const el = row?.querySelector('[name]:checked') || row?.querySelector('[name]'); if (el && row.dataset.btBase !== undefined) row.classList.toggle('changed', Math.abs(+el.value - +row.dataset.btBase) > 1e-9); }
// after a run from the window: bring the results into view and say what ran
function btAnnounce() {
  let n = 0; const t = setInterval(() => { const m = document.querySelector('.metrics'); if (m || ++n > 120) { clearInterval(t); if (!m) return; m.scrollIntoView({behavior: 'smooth', block: 'center'}); m.classList.add('bt-flash'); setTimeout(() => m.classList.remove('bt-flash'), 2400); } }, 250);
}
document.addEventListener('click', e => {
  if (e.target.closest('[data-bt-open]')) { e.preventDefault(); return btRun(); }
  const d = document.getElementById('bt-dialog'); if (!d) return;
  if (e.target.closest('[data-bt-close]') || e.target === d) { d.close?.(); d.remove(); return; }
  if (e.target.closest('[data-bt-reset]')) { d.querySelectorAll('[data-bt-key][data-bt-base]').forEach(row => { row.querySelectorAll('[name], [data-bt-range]').forEach(el => { if (el.type === 'radio') el.checked = el.value === row.dataset.btBase; else el.value = row.dataset.btBase; }); btMarkChanged(row); }); btInactive(d); }
});
document.addEventListener('input', e => {
  const d = e.target.closest('#bt-dialog'); if (!d) return;
  const k = e.target.dataset.btRange || e.target.name, row = e.target.closest('.bt-row');
  if (e.target.dataset.btRange) { const n = d.querySelector(`input[type=number][name="${k}"]`); if (n) n.value = e.target.value; }
  else if (e.target.type === 'number') { const r = d.querySelector(`[data-bt-range="${k}"]`); if (r) r.value = e.target.value; }
  btMarkChanged(row); btInactive(d);
});
document.addEventListener('submit', e => {
  if (e.target.id !== 'bt-form' || !STRAT) return; e.preventDefault();
  const fd = new FormData(e.target), schema = btSchema(STRAT.v), from = String(fd.get('from') || ''), to = String(fd.get('to') || '');
  if (!from || !to || from >= to) { document.getElementById('bt-err').textContent = 'From must be before To.'; return; }
  const vals = {}; for (const p of schema) if (fd.has(p.key)) vals[p.key] = +fd.get(p.key);
  // the URL keeps every changed value plus the reference sheet's own settings, so the reference is not re-applied over them
  const ref = btRef(STRAT.v, STRAT.tk), p = changedParams(schema, vals);
  for (const k of Object.keys(ref?.params || {})) if (k in vals) { const d = schema.find(x => x.key === k); p[k] = d ? clampParam(d, vals[k]) : vals[k]; }
  const bps = fd.has('@cost') ? Math.max(0, Math.round(+fd.get('@cost') * 10000) / 100) : null;
  const t = selectedSpan(S.tk), win = coverageWindow(STRAT.v, S.tk), slip = bps !== null ? bps : Math.max(0, +fd.get('slippage') || 0), slippage = slip === 0 ? '0.0' : String(slip);
  const f2 = from <= t.first || (win && from === win.from) ? '' : from, t2 = to >= t.last || (win && to === win.to) ? '' : to;
  const dlg = document.getElementById('bt-dialog'); dlg.close?.(); dlg.remove();
  // a zero cost is written as 0.0: the plain default '0' would be dropped from the URL and the reference 0.03% applied again.
  // The Net Profit display follows the sizing (fixed quantity: rupee result; compounding: compounded returns).
  const sizing = 'sizing' in vals ? { report_mode: vals.sizing === 2 ? 'fixed' : 'compound' } : {};
  BT_PENDING = { path: R.path, state: { ...S, p: encodeP(p), from: f2, to: t2, slippage, ...sizing, ...(fd.has('rollover') ? { rollover: String(Math.max(0, +fd.get('rollover') || 0)) } : {}) } };
  btPendingMark();
});
// settings applied in the Strategy settings popup wait for "Run backtest"
let BT_PENDING = null;
function btPendingMark() {
  const b = document.querySelector('[data-bt-open]'); if (!b) return;
  b.classList.toggle('bt-pending', !!BT_PENDING);
  let n = document.getElementById('bt-pending-note');
  if (BT_PENDING && !n) { b.insertAdjacentHTML('afterend', '<small id="bt-pending-note" class="bt-pending-note" role="status">Settings changed — press Run backtest</small>'); }
  else if (!BT_PENDING && n) n.remove();
}
function btRun() {
  const job = BT_PENDING && BT_PENDING.path === R.path ? BT_PENDING : null; BT_PENDING = null;
  if (job && href(job.path, job.state) !== location.hash) go(job.path, job.state); else render();
  btAnnounce();
}
addEventListener('hashchange', () => { if (BT_PENDING && BT_PENDING.path !== R.path) BT_PENDING = null; });
// the toolbar's settings icon opens the parameter popup on pages that have one
document.addEventListener('click', e => {
  const t = e.target.closest('.strategy-settings-trigger'); if (!t || !STRAT || R.name !== 'strategy' || !btSchema(STRAT.v)?.length || !isYt(STRAT.v)) return;
  e.preventDefault(); e.stopPropagation(); btOpen();
}, true);
function btstInstrumentNote(v, tk) {
  if (universeScope(v)) return `<p class="instrument-note btst-instrument" data-btst-instrument><b>${esc(universeTradedLine(decodeP(S.p).src || 1, uniOf(v.id), STRAT?.d?.instrument?.database || undefined))}</b>. A portfolio of up to ${esc(STRAT?.d?.max_positions ?? decodeP(S.p).max_positions ?? 10)} positions; ${esc(universeText(uniOf(v.id), 'calendar_note', 'NIFTY futures supply the calendar and the buy & hold comparison.'))}</p>`;
  const ins = btstInstrument(v?.id, tk, decodeP(S.p).src || 1); if (!ins) return '';
  return `<p class="instrument-note btst-instrument" data-btst-instrument><b>Traded: ${esc(ins.symbol)}</b> · ${esc(ins.source)}. The index button only selects the series.</p>`;
}
function referenceBox(v, tk) {
  const r = ytFamilyReference(v?.id); if (!r) return '';
  const file = f => `<code class="ref-path">${esc(ytRefPath(v?.id, f.file))}</code><span class="ref-sub">sheet <b>${esc(f.sheet)}</b> · ${esc(f.detail)}</span>`;
  const main = universeScope(v) ? universeTradesheet(r, decodeP(S.p)) : mainTradesheet(r, tk, decodeP(S.p));
  const sheet = main ? file(main) : r.best ? bestSettingBlock(v, tk, r.best)
    : `<b class="ref-none">${esc(NO_TRADESHEET)}</b>${r.summary ? `<span class="ref-sub">Team results only (no trade list):</span>${file(r.summary)}` : ''}`;
  return `<section class="box ref-box" data-btst-reference><div class="box-h">Reference <small>the team's own files, inside the ${esc(ytFamilyFolder(v?.id))} folder</small></div>
    <dl class="ref-list"><div><dt>Reference AFL</dt><dd><code class="ref-path">${esc(ytRefPath(v?.id, r.afl))}</code><span class="ref-sub">${esc(r.aflNote)}</span></dd></div>
    <div><dt>${!main && r.best ? 'Compared with' : 'Reference trade sheet'}</dt><dd>${sheet}${(() => { if (!universeScope(v)) return ''; const pk = uniPick(v.id), def = uniBaskets(v.id).find(b => b.key === uniDefaultBasket(v.id)); if (pk.isDefault && !S.only) return '';
      const now = S.only ? `the stock ${S.only} alone` : `${pk.b?.label || ''} · ${UNI_SET[pk.set]}`; return `<span class="ref-sub ref-none" data-ref-basket-note>This run uses ${esc(now)}; the reference sheet was made on ${esc((def?.cp || '').replace(/\.tls$/, ''))} (C&amp;P). Compare with it only on that basket.</span>`; })()}</dd></div></dl></section>`;
}
// no trade sheet: the team's highest-return row (results workbook), where it is, its settings and numbers, and one link that
// sets the whole run (instrument, settings, brokerage, period); a tick when the page already runs it
function bestSettingBlock(v, tk, best) {
  const uni = universeScope(v), vals = paramValues(btSchema(v), decodeP(S.p)), t = best.team, n = x => (+x).toLocaleString('en-IN', {maximumFractionDigits: 2});
  const on = bestActive(best, {universe: uni, ticker: tk, params: vals, cost: +(S.slippage || 0), from: S.from || '', to: S.to || ''});
  const where = best.scope === 'universe' ? `F&O stocks (${UNIVERSE?.universe || 'FO STOCKS C&P'})` : btstFuturesSymbol(best.ticker) || LABEL[best.ticker] || best.ticker;
  const state = {...S, run_scope: best.scope === 'universe' ? 'universe' : 'single', basket: '', tk: best.scope === 'universe' ? 'NIFTY 50' : best.ticker, from: best.from || '', to: best.to, p: encodeP(best.params), slippage: String(best.cost), report_mode: 'compound'};
  return `<b class="ref-best">${esc(BEST_SETTING)}</b><code class="ref-path">${esc(ytRefPath(v?.id, best.file))}</code>
    <span class="ref-sub">sheet <b>${esc(best.sheet.trim())}</b>${best.table ? ` · table ${esc(best.table)}` : ''} · <b>${esc(best.row)}</b> · ${esc(best.cells)}</span>
    <span class="ref-sub">Setting: ${esc(best.settings)}; period ${esc(best.from || 'first data day')} to ${esc(best.to)}.</span>
    <span class="ref-sub">Team result: Net ${n(t.net)}% · CAR ${n(t.car)}% · max drawdown ${n(t.mdd)}% · ${n(t.trades)} trades · ${n(t.win)}% winners · avg ${n(t.avg)}% per trade</span>
    ${on ? `<span class="ref-best-on" role="status" data-best-active>✓ This page runs the best setting (${esc(where)})</span>`
      : `<span class="ref-best-off"><a class="dbtn sm" data-best-setting href="${href(R.path, state)}">Run with the best setting</a><small>sets ${esc(where)}, the settings, brokerage ${n(best.cost / 100)}% and the period</small></span>`}`;
}
// The dates in which every series a strategy requires has data, when narrower than the instrument's history (e.g. BTST futures bars).
function coverageWindow(v, tk) {
  if (universeScope(v) && uniOf(v.id)?.first && uniOf(v.id)?.last) return { from: uniOf(v.id).first, to: uniOf(v.id).last };
  const needs = v?.id?.startsWith('ytbtst_') && decodeP(S.p).src === 2 ? ['BTST_F5M.C1525'] : v?.needs_series;
  return strategyDataWindow(needs, [...TICK_ALL_IN, ...(TICK_ALL_US || [])].find(x => x.ticker === tk)?.coverage, selectedSpan(tk));
}
function periodPanel(tk, d, issue = null) {
  // A blank period means all the data the strategy has, so such a strategy's default period is its data coverage.
  const t = selectedSpan(tk), first = t.first, last = t.last, win = coverageWindow(STRAT?.v || ALLCAT.find(v => v.id === R.id), tk), f = S.from || (win ? win.from : first), to = S.to || (win ? win.to : last);
  // All / Before book / After book (the book's publication year splits the data like in-sample and out-of-sample)
  const noBefore = +first.slice(0, 4) > PUB - 1, noAfter = +last.slice(0, 4) < PUB;
  const tog = [['All', '', '', `${first.slice(0, 4)}–${last.slice(0, 4)}`, false], ['Before book', '', `${PUB - 1}-12-31`, noBefore ? 'no data' : `to ${PUB - 1}`, noBefore], ['After book', `${PUB}-01-01`, '', noAfter ? 'no data' : `${PUB}–${last.slice(0, 4)}`, noAfter]].filter((_,i)=>PUB || i===0);
  // Such a strategy cannot run on the instrument's whole history, so its coverage replaces "All" and is selected by default.
  if (win) tog.splice(0, tog.length, ['Data coverage', '', '', `${win.from.slice(0, 4)}–${win.to.slice(0, 4)}`, false]);
  const on = ([, a, b]) => (S.from || '') === a && (S.to || '') === b;
  return `<p class="print-only">Backtest: ${esc(f)} to ${esc(to)} · Slippage: ${esc(S.slippage||0)} bps per side · Rollover: ${esc(S.rollover||0)} bps per actual roll · Basket: ${esc(S.basket||'Single instrument')}</p><form class="period" id="period"><span class="swl">Backtest period</span>
    <div class="join sm ptog" role="group" aria-label="Period">${tog.map(x => `<button type="button" class="join-item ${on(x) ? 'active' : ''} ${x[4] ? 'off' : ''}" data-period="${x[1]}|${x[2]}" aria-pressed="${on(x)}" ${x[4] ? 'disabled title="No data before the book was published"' : ''}>${x[0]}<small>${x[3]}</small></button>`).join('')}</div>
    <label class="dfield">From <input type="date" name="from" value="${f}" min="${first}" max="${last}" required></label>
    <label class="dfield">To <input type="date" name="to" value="${to}" min="${first}" max="${last}" required></label>
    <label class="dfield">Slippage <input type="number" name="slippage" value="${esc(S.slippage)}" min="0" max="500" step="0.1"> bps a side</label><label class="dfield">Rollover <input type="number" name="rollover" value="${esc(S.rollover)}" min="0" max="500" step="0.1"> bps per roll</label>
    <span class="perr" id="perr"></span></form>${STRAT?varyingControls(STRAT.v,tk):R.id&&CAT.find(v=>v.id===R.id)?varyingControls(CAT.find(v=>v.id===R.id),tk):''}`;
}
const apiError = (e, tk) => `${e} (${LABEL[tk]})`;
const verdictText = (s) => `<b>${fx(s.trades,0)}</b> closed trades in ${fx(s.years,1)} years, in the market ${pct(s.exposure,0)} of the time. It compounded <b class="${cls(s.cagr)}">${pct(s.cagr,2,true)}</b> a year against <b>${pct(s.bh_cagr,2,true)}</b> for buy &amp; hold, with a worst drawdown of <b class="neg">${pct(s.max_dd,1)}</b> (buy &amp; hold ${pct(s.bh_max_dd,1)}). ${s.trades<30?'Too few trades to judge.':s.t_stat>=2?`The average trade is statistically significant (t = ${fx(s.t_stat)}).`:`The average trade is not statistically different from zero (t = ${fx(s.t_stat)}).`}`;
function withheldStrategy(v,tk,crumbs,switches,issue) {
  RULE_VISUAL=makeRuleVisualModel({strategy:v,symbol:LABEL[tk]||tk,status:issue.status,blocked:issue.note,parameters:{...bookDefaults(v.id),...decodeP(S.p)}});
  const label=S.run_scope==='basket'?BASKETS.find(b=>b.id===S.basket)?.name||S.basket:LABEL[tk]||tk;
  const span=selectedSpan(tk),coverage=BASKETS.find(b=>b.id===S.basket)?.price_coverage;
  const recovery=S.run_scope==='basket'&&S.membership==='current'&&instrumentControls(v.instrument_policy,v.id,v.book).basket&&issue.status==='needs_data'
    ?priceRecoveryPeriod(coverage,S.from||span.first,S.to||span.last):null;
  const recoveryHTML=recovery?`<p data-price-recovery>Daily price coverage is available from ${esc(recovery.from)} to ${esc(recovery.to)}. Some requested dates have no supplied quotation; the selected dates have not been changed. <button type="button" class="dbtn sm" data-period="${recovery.from}|${recovery.to}">Run available price period</button></p>`:'';
  return `${crumbs}<section class="box" data-backtest-state="${esc(issue.status)}"><div class="st-head"><h1 class="st">${esc(familyDisplayName(v))}</h1>${VaultUI.toolbar()}</div><div class="tags2"><span class="badge outline">${esc(v.dir)}</span><span>${esc(label)}</span><span class="badge gapb">${esc(heldLabel(issue))}</span></div>${switches}${periodPanel(tk,null,issue)}${sourceBlockers(v)}<div class="empty gapbox"><b>${esc(heldLabel(issue))}</b>${esc(issue.note)}<p>No performance figures or charts are displayed for this run. ${isYt(v)?'The rules below':'The book information and rules below'} are reference material, not evidence of a completed backtest.</p>${recoveryHTML}</div></section><section class="box"><div class="box-h">${v.guru==='ross'?'Source rules and unresolved requirements':isYt(v)?'Rules and implementation notes':'Book rules and implementation notes'}<span data-react-slot="visual-rules"></span></div>${rulesPanel(v)}</section>${referenceBox(v, tk)}${gptSection(v,tk,null,issue)}`;
}

// ---- BTST on the F&O stock universe (guru/yt/universe-run): portfolio of the team's AFL rules over FO STOCKS C&P ----
const UNIVERSE_RUNS = new Map(), UNIVERSE_SUMMARY = {}; // results of this session (4 kept), last summary per sub-variant for the cards
const UNIVERSE_BUSY = 'Another stock-universe run is in progress, try again in a few seconds.';
async function fetchUniverse(query) {
  const url = `${API_IN}/guru/yt/${new URLSearchParams(query).get('id')?.startsWith('ytath_') ? 'ath-run' : 'universe-run'}?${query}`;
  for (let attempt = 0; ; attempt++) {
    const response = await fetch(url, { signal: AbortSignal.timeout(125000) });
    if (response.status === 429) { if (attempt) throw Error(UNIVERSE_BUSY); await new Promise(ok => setTimeout(ok, 4000)); continue; }
    if (!response.ok) throw Error((await response.text()) || `HTTP ${response.status}`);
    return response.json();
  }
}
// universe trade sheet: every trade reachable — symbol filter, year, pages of 500 (newest first); downloads hold all
const UNI_SHEET = { symbol: '', year: '', page: 1 };
function universeTradeSheet(d) {
  const all = universeTradeRows(d.trades), pg = tradePage(all.slice().reverse(), { ...UNI_SHEET, size: 500 });
  const cell = (v, i) => v === null ? '—' : UNIVERSE_PERCENT_COLUMNS.includes(i) ? pct(v, 2) : i === 8 ? fx(v, 0) : i === 10 ? (+v).toFixed(4) : typeof v === 'number' ? v.toFixed(2) : esc(v);
  const count = `${pg.total ? `${fx(pg.first, 0)}–${fx(pg.last, 0)} of ${fx(pg.total, 0)}` : 'No trades'}${UNI_SHEET.symbol || UNI_SHEET.year ? ` (filtered from ${fx(all.length, 0)})` : ''} · page ${pg.page} of ${pg.pages}`;
  const pager = `<span class="uni-pages"><button type="button" class="dbtn sm" data-uni-page="${pg.page - 1}" ${pg.page <= 1 ? 'disabled' : ''}>‹ Newer</button><span class="uni-count">${count}</span><button type="button" class="dbtn sm" data-uni-page="${pg.page + 1}" ${pg.page >= pg.pages ? 'disabled' : ''}>Older ›</button></span>`;
  // the Year choice and the pager sit in the toolbar next to the Symbol filter; a second pager below the table box
  const top = `<label class="dfield">Year <select id="universe-year"><option value="">All years</option>${pg.years.map(y => `<option ${y === UNI_SHEET.year ? 'selected' : ''}>${y}</option>`).join('')}</select></label>${pager}`;
  const table = `<table class="tbl tl" id="tradesheet"><thead><tr>${UNIVERSE_TRADE_HEADERS.map(k => `<th>${esc(k)}</th>`).join('')}</tr></thead><tbody>${pg.rows.map(r => `<tr>${r.map((v, i) => `<td class="${[4, 7, 8, 9, 10, 11].includes(i) ? 'n' : ''}${i === 11 ? (v < 0 ? ' neg' : ' pos') : ''}">${cell(v, i)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
  const bottom = `${pg.pages > 1 ? pager : ''}<p class="rnote" data-universe-shown>All ${fx(all.length, 0)} trades are in the Excel and CSV downloads.</p>`;
  return { top, table, bottom };
}
const uniRedraw = () => { if (STRAT?.d?.kind !== 'universe') return; const t = universeTradeSheet(STRAT.d);
  for (const [id, html] of [['universe-nav', t.top], ['universe-trades', t.table], ['universe-nav-bottom', t.bottom]]) { const el = document.getElementById(id); if (el) el.innerHTML = html; } };
document.addEventListener('input', e => { if (STRAT?.d?.kind !== 'universe') return;
  if (e.target.id === 'universe-symbol') { UNI_SHEET.symbol = e.target.value; UNI_SHEET.page = 1; uniRedraw(); }
  else if (e.target.id === 'universe-year') { UNI_SHEET.year = e.target.value; UNI_SHEET.page = 1; uniRedraw(); } });
document.addEventListener('click', e => { const b = e.target.closest('[data-uni-page]'); if (!b || STRAT?.d?.kind !== 'universe') return; UNI_SHEET.page = +b.dataset.uniPage; uniRedraw(); document.getElementById('universe-nav')?.closest('.uni-toolbar')?.scrollIntoView({ block: 'start' }); });
// ---- ATH stock-universe parameter optimization (guru/yt/ath-optimize): the engine runs the backtest once per combination of the ticked settings ----

// ---- the lab-style result of an ATH grid for the 4 verified variants (same look and files as ADR): Now / #1-#3, downloads, sortable list
const ATH_LAB = {};
// S = the portfolio / backtest settings (positions, start bar, sizing, ranking), listed under Setup: on the optimization sheet's Rules
const athEngineOpt = (label = '') => /^\s*(ENGINE OPTION|\(engine option|ASSUMPTION)/i.test(label); // the engine's own choices, not AFL Params
// E entry, F filters (stock and market), X exit, S portfolio / backtest set-up (positions, sizing, ranking, reshuffle, engine options);
// the label decides where the key alone cannot ("ST EXIT Period" is an exit setting, "Buying Buffer %" an entry one, "RANK FOR ENTRY" a ranking one)
const athGroup = (k, label = '') =>
  athEngineOpt(label) || /^(max_positions|tstart|pct|equity|sizing|alloc|eqallo|deadstock|rank\w*|resf|exitrank|backtest|backtestmarket|bkmkt|entryday|janentryday|buysize|sellsize|comm1|mcxsymbl)$/.test(k) || /rank/i.test(label) ? 'S'
  : /\bexit\b/i.test(label) ? 'X'
  : /\bentry\b|buying buffer|buy buffer|buy-side/i.test(label) || /^(noofloss|losspct)$/.test(k) ? 'E'
  : /^(exit|hardstop|hardsl|hsl|tsl|buffer|sell|pb|minprofit|profit|pt_pct|sl_pct|slpct|trail|atr_sl|atr_pt|atrtrail|days_sl|ndays|holdingdays|typeofexit|swing|sharesselling|prcnt|ver$)/.test(k) || /_(exit|sell)/.test(k) ? 'X'
  : /^(bpd|bfactor|stflt|st_flt|flt|filter|aux|penny|turn|mkt|sflt|iflt|istflt|prflt|circuit|bigprice|volflt|avg_turnover|avgturnover|vix|avg_vix|datahole|ver1$)/.test(k) ? 'F' : 'E';
function athLabHTML(v, res) {
  const keys = Object.keys(res.grid), ps = athOptParams(btSchema(v)), fixed = {...Object.fromEntries(ps.map(d => [d.key, d.def])), ...decodeP(S.p)};
  const labels = keys.map(k => ps.find(d => d.key === k)?.label || k);
  const idx = res.rows.map((_, i) => i).filter(i => !res.rows[i].default), now = res.rows.findIndex(r => r.default);
  const ares = {id: v.id, dims: keys.map(k => ({key: k, values: res.grid[k]})), fixed, rows: idx.map(i => keys.map(k => res.rows[i].p[k])), sheet: idx.map(i => res.sheet[i]), now_sheet: res.sheet[now]};
  const lab = labRows(ares, 'Long', fixed), ci = AM_COLUMNS.indexOf('CAR/MDD'), ti = AM_COLUMNS.indexOf('# Trades');
  // the three best on CAR/MDD among the combinations that traded at least once a year (25 trades over the 22 years)
  const top = lab.rows.filter(r => r.am[ti] >= 25 && Number.isFinite(r.am[ci])).sort((a, b) => b.am[ci] - a.am[ci]).slice(0, 3).map(r => r.k);
  const sort = ATH_LAB[v.id]?.sort || {col: labels.length + 2, dir: -1};
  ATH_LAB[v.id] = {res, ares, keys, labels, lab, top, sort, fixed, params: idx.map(i => res.rows[i].p)};
  return `<div class="lab-results">${resultsHTML(ares, labels, lab, top, 'CAR', sort, false)}</div>`;
}
async function athLabRun(v, k, bars = false) {
  const A = ATH_LAB[v.id], uq = universeQuery(v.id, {from: S.from, to: S.to, slippage: S.slippage, params: {...A.fixed, ...A.params[k]}});
  if (bars) uq.set('bars', '1');                                          // each trade's own daily bars, for the team tradesheet's All Index Explore and MAE
  { const pk = uniPick(v.id); if (pk.b && !pk.isDefault) { uq.set('watchlist', pk.set === 'current' ? pk.b.current : pk.b.cp); if (pk.set === 'past') uq.set('exclude_watchlist', pk.b.current); } }
  const d = await fetchUniverse(uq.toString()); if (d?.status !== 'ready') throw new Error(d?.note || d?.error || 'The run is not available.');
  return d;
}
const athLabReport = d => buildReport(athRunOf(d), null, {capital: d.model_initial_equity || 1000000, side: ((Number(d.costs?.fees_bps) || 0) + (Number(d.costs?.slippage_bps) || 0)) / 10000});
const athShort = l => l.replace(/\s*[(;].*$/, '').trim() || l;             // the sheet's header cell is narrow: "Hard Stoploss % (0 = off; ...)" -> "Hard Stoploss %"
const athLabName = (A, k) => A.keys.map((key, i) => `${key} ${A.lab.rows[k].settings[i]}`).join(' · ');
// the research team's Excel tradesheet for one ATH combination, the same workbook as ADR's "Download TS" (ts-workbook.js): every trade of the basket with its own symbol,
// All Index Explore holds, per trade, the bar that sets its MAE (maeBars), costs are the run's own (fees + slippage per side, both sides) and stay editable in C1:E1
async function athLabTradesheet(v, k) {
  const A = ATH_LAB[v.id], d = await athLabRun(v, k, true), params = {...A.fixed, ...A.params[k]};
  const fmt = x => String(x).slice(0, 10), dmy = x => fmt(x).split('-').reverse().join('-'), name = athLabName(A, k);
  const lines = c => ruleLines(v, params).filter(r => r[0] === c).map(r => r[1]);
  const cost = d.costs || {}, side = (Number(cost.fees_bps) || 0) + (Number(cost.slippage_bps) || 0), dates = d.dates || [];
  const rows = (d.trades || []).filter(t => Number.isFinite(t.entry_px) && Number.isFinite(t.exit_px))
    .map(t => ({dir: /short/.test(t.reason || '') ? 'Short' : 'Long', entry: fmt(t.entry), entryPx: t.entry_px, exit: fmt(t.exit), exitPx: t.exit_px, sym: t.symbol, bars: (t.bars || []).map(b => [fmt(b[0]), b[1], b[2], b[3], b[4]])}))
    .sort((a, b) => (a.entry < b.entry ? -1 : a.entry > b.entry ? 1 : a.sym < b.sym ? -1 : a.sym > b.sym ? 1 : 0));
  if (!rows.length) throw Error('This combination has no trades.');
  const bars = maeBars(rows);                                              // one bar per trade (the one that sets its MAE) keeps the file small; the MAE formulas give the same result
  await loadXlsx(); if (!window.XLSX?.utils) throw Error('The Excel library did not load.');
  const tab = fileSafe(name).replace(/'/g, '').slice(0, 31);
  const book = buildTsWorkbook(XLSX, {ticker: '', tabName: tab, trades: rows, bars, mae: rows.map(t => maeOf(t, t.bars)),
    rules: {timeframe: 'Daily', index: d.universe || STRAT.tk, period: `${dmy(dates[0])} to ${dmy(dates.at(-1))}`, position: rows.some(t => t.dir === 'Short') ? 'Short' : 'Long', filter: lines('F').join('; ') || 'Without filter', setup: '—', entry: lines('E').join('; '), exit: lines('X').join('; '), aflPath: ''},
    costs: {impact: 2 * side / 10000, rollLong: 0, rollShort: 0}});
  saveBytes(await tsBytes(XLSX, book), `TS_${fileSafe(name)}.xlsx`);
}
async function athLabAll(v, btn) {
  const A = ATH_LAB[v.id], n = A.lab.rows.length, one = $('#adr-ts-mode')?.value === 'one', tk = STRAT.tk, label0 = btn.textContent;
  const files = [], sheets = [], sum = [['No.', 'Combination', 'Net Profit %', 'CAR', 'MaxSysDD', 'CAR/MDD', '# trades', '# winners', '# losers', 'Avg. profit/loss %', 'Sharpe', 'Buy & hold CAGR']];
  btn.disabled = true;
  try {
    for (let k = 0; k < n; k++) {
      btn.textContent = `Running ${k + 1} of ${n}…`;
      const d = await athLabRun(v, k), rep = athLabReport(d), s = rep.system, a = rep.all, nn = String(k + 1).padStart(2, '0'), name = athLabName(A, k);
      sum.push([k + 1, name, 100 * s.net_profit, 100 * s.cagr, 100 * s.max_sys_dd, s.car_mdd, a.trades, a.winners.count, a.losers.count, 100 * a.avg, s.sharpe_trades, rep.buy_hold ? 100 * rep.buy_hold.cagr : null]);
      const sheet = {name: one ? `${nn} ${name}`.slice(0, 31) : 'Tradesheet', layout: 'table', header: TRADE_COLUMNS, rows: tradeRows(rep, '')};
      if (one) sheets.push(sheet); else files.push([`${nn} ${fileSafe(name)}`.slice(0, 90) + '.xlsx', buildLabXlsx([sheet])]);
    }
    btn.textContent = 'Building the workbook…';
    const summary = {name: 'Summary', layout: 'table', header: sum[0], rows: sum.slice(1)};
    if (one) saveBytes(buildLabXlsx([summary, ...sheets]), `tradesheets ${tk}.xlsx`);
    else { files.unshift(['00 Summary.xlsx', buildLabXlsx([summary])]); saveBytes(zipStored(files), `tradesheets ${tk}.zip`, 'application/zip'); }
  } finally { btn.disabled = false; btn.textContent = label0; }
}
function athLabSheet(v) {
  const A = ATH_LAB[v.id], tk = uniOf(v.id)?.universe || STRAT.tk, dmy = x => String(x).slice(0, 10).split('-').reverse().join('-'), st = athOptState(v), short = A.labels.map(athShort);
  const lines = {E: [], F: [], X: [], S: []};
  A.keys.forEach((k, i) => { const vals = A.res.grid[k], r = st.rng[k]; lines[athGroup(k, A.labels[i])].push(rangeLine(short[i], vals[0], vals.at(-1), r ? +r.step : (vals[1] - vals[0]) || 1)); });
  // each section as the lab writes it: the rule text (optimized settings at their From value), then " · " and the setting ranges
  const rules = ruleLines(v, {...A.fixed, ...Object.fromEntries(A.keys.map(k => [k, A.res.grid[k][0]]))});
  const sec = g => [rules.filter(r => r[0] === g).map(r => r[1]).join('; '), ...lines[g]].filter(Boolean).join(' · ');
  const bps = Number(A.res.slippage ?? S.slippage ?? 0), from = S.from || '2000-01-03', to = S.to || '2022-03-31';
  const bytes = buildOptimizationXlsx({labels: short, rows: A.lab.rows.map(r => ({no: r.no, am: r.am, settings: r.settings})), rules: {timeframe: 'Daily', index: tk, period: `${dmy(from)} to ${dmy(to)}`, brokerage: bps > 0 ? `${bps} bps per side` : 0, openPosition: A.fixed.max_positions ?? 1,
    aflPath: 'Strategy Vault · YT Strategies', strategy: v.name || v.id, position: 'Long', filter: sec('F'), setup: sec('S'), entry: [sec('E')].filter(Boolean), exit: [sec('X')].filter(Boolean)}});
  saveBytes(bytes, optimsFileName(tk, 'Long', ''));
}
function athLabClick(e) {
  const v = STRAT?.v, A = v && ATH_LAB[v.id]; if (!A) return false;
  const t = e.target;
  const stats = t.closest('[data-stats]'); if (stats) { go('/s/' + encodeURIComponent(v.id), {...S, p: encodeP({...A.fixed, ...A.params[+stats.dataset.stats]}), run_scope: 'universe'}); return true; }
  const ts = t.closest('[data-tsrow]'); if (ts) { const l = ts.textContent; ts.textContent = '…'; athLabTradesheet(v, +ts.dataset.tsrow).then(() => { ts.textContent = l; }, err => { console.error('Tradesheet download failed', err); ts.title = 'Failed: ' + (err?.message || err); ts.textContent = 'Failed'; }); return true; }
  const th = t.closest('th[data-sort]'); if (th) { const c = +th.dataset.sort; A.sort = A.sort && A.sort.col === c ? {...A.sort, dir: -A.sort.dir} : {col: c, dir: c < A.labels.length ? 1 : -1}; $('#adr-list').innerHTML = listHTML(A.labels, A.lab, A.sort); return true; }
  if (t.closest('[data-showall]')) { A.sort = {...(A.sort || {col: -1, dir: 1}), all: true}; $('#adr-list').innerHTML = listHTML(A.labels, A.lab, A.sort); return true; }
  if (t.closest('#adr-opt-sheet')) { athLabSheet(v); return true; }
  const all = t.closest('#adr-ts-all'); if (all) { athLabAll(v, all).catch(err => { all.textContent = 'Failed: ' + err.message; }); return true; }
  return false;
}

const ATH_OPT = {}; // per sub-variant: { on: {key: bool}, rng: {key: {from,to,step}}, sort, res, msg, busy }
const athOptState = v => ATH_OPT[v.id] || (ATH_OPT[v.id] = { on: {}, rng: {}, sort: 'cagr', res: null, msg: '', busy: false });
function athOptGrids(v, st) {
  const out = {}, fixed = decodeP(S.p); for (const d of athOptParams(btSchema(v))) { if (!st.on[d.key]) continue; const r = st.rng[d.key] || defaultRange({ ...d, def: fixed[d.key] ?? d.def }); out[d.key] = rangeValues(+r.from, +r.to, +r.step, d); } return out;
}
function athOptBody(v) {
  const st = athOptState(v), ps = athOptParams(btSchema(v)), fixed = decodeP(S.p), grids = athOptGrids(v, st), problem = gridProblem(grids), n = Object.keys(grids).length ? comboCount(grids) : 0;
  const viewer = window.VAULT_APP && window.VAULT_USER?.role === 'viewer';
  const row = d => { const r = st.rng[d.key] || defaultRange({ ...d, def: fixed[d.key] ?? d.def }), inp = f => `<input type="number" data-f="${f}" value="${esc(r[f])}" step="any">`;
    return `<tr data-k="${esc(d.key)}" class="${st.on[d.key] ? '' : 'offrow'}" title="Default ${esc(fixed[d.key] ?? d.def)} · AFL range ${esc(d.min)} – ${esc(d.max)}; any value runs"><td><input type="checkbox" data-f="on" ${st.on[d.key] ? 'checked' : ''} aria-label="Vary ${esc(d.label || d.key)}"></td><td>${esc(d.label || d.key)}</td><td>${inp('from')}</td><td>${inp('to')}</td><td>${inp('step')}</td><td class="n muted" data-cnt>${st.on[d.key] ? (grids[d.key]?.length ?? 0) : 'fixed'}</td></tr>`; };
  const form = `<section class="box${isAth4(v.id) ? ' adr-opt' : ''}" id="ath-optim"><div class="box-h">Optimization <small>Tick the settings to vary. Unticked settings stay at the default value (or the value set on the backtest page). Runs on the period selected above.</small></div>
    ${optGrid(ps, row, 'ath-opt-grid')}
    <div class="obar"><span class="ocount" id="ath-opt-count">${n ? `${fx(n, 0)} combinations plus the current settings · about ${fx(Math.max(1, n + 1) * 1.5, 0)}–${fx((n + 1) * 4, 0)} seconds. ` : ''}${esc(problem)}</span><button class="dbtn primary sm" type="button" id="ath-opt-run" ${problem || st.busy || viewer ? 'disabled' : ''} ${viewer ? 'title="Researcher access is required to optimize"' : ''}>${st.busy ? 'Running…' : 'Run optimization'}</button><span class="grow"></span><span class="otried muted">${optTried()}</span></div></section>`;
  let result = '';
  if (st.busy) result = `<section class="box" id="ath-opt-res">${bookLoader('Rust is running the stock-universe backtests…')}</section>`;
  else if (st.msg) result = `<section class="box" id="ath-opt-res"><div class="empty gapbox"><b>Optimization not available</b>${esc(st.msg)}</div></section>`;
  else if (st.res && st.res.sheet && isAth4(v.id)) result = `<section class="box adr-opt" id="ath-opt-res">${athLabHTML(v, st.res)}</section>`;
  else if (st.res) {
    const res = st.res, keys = Object.keys(res.grid), sorted = athSortRows(res.rows, st.sort);
    const link = r => href('/s/' + encodeURIComponent(v.id), { ...S, p: encodeP({ ...fixed, ...r.p }), run_scope: 'universe' });
    const sortBtn = (k, t) => `<button type="button" class="join-item ${st.sort === k ? 'active' : ''}" data-ath-sort="${k}">${t}</button>`;
    result = `<section class="box" id="ath-opt-res"><div class="box-h">Results <span class="legend"><small>${fx(res.rows.length, 0)} runs in ${fx(res.seconds, 0)} s · ${esc(S.from || 'start of data')} to ${esc(S.to || 'end of data')}</small><span class="join sm">${sortBtn('cagr', 'CAGR')}${sortBtn('sharpe', 'Sharpe')}${sortBtn('car_mdd', 'CAR/MDD')}</span></span></div>
      <div class="tlog"><table class="tbl" id="ath-opt-table"><thead><tr><th>#</th>${keys.map(k => `<th>${esc(ps.find(d => d.key === k)?.label || k)}</th>`).join('')}<th>CAGR</th><th>Sharpe</th><th>Max DD</th><th>CAR/MDD</th><th>Trades</th><th>Win rate</th><th></th></tr></thead><tbody>${sorted.map((r, i) => { const m = r.stats || {}, cur = r.default;
        return `<tr ${cur ? 'class="hl" style="background:var(--hover,rgba(120,120,255,.12));font-weight:600"' : ''}><td>${i + 1}</td>${keys.map(k => `<td class="n">${esc(cur ? (fixed[k] ?? ps.find(d => d.key === k)?.def ?? '') : r.p[k])}</td>`).join('')}<td class="n ${cls(m.cagr)}">${pct(m.cagr, 2)}</td><td class="n">${fx(m.sharpe)}</td><td class="n neg">${pct(m.max_dd, 2)}</td><td class="n">${fx(carMdd(m))}</td><td class="n">${fx(m.trades, 0)}</td><td class="n">${pct(m.win_rate, 1)}</td><td>${cur ? 'current settings · ' : ''}<a href="${link(r)}">Open backtest</a></td></tr>`; }).join('')}</tbody></table></div>
      <p class="rnote">The highlighted row is the setting now in use. Parameter selection is research, not proof of future performance; the best row of a grid is partly luck. Each row is a complete portfolio backtest of the team's AFL rules.</p></section>`;
  }
  return form + result;
}
function athOptDraw() { const el = $('#ath-opt-body'); if (el && STRAT?.ath) { el.innerHTML = athOptBody(STRAT.v); optFit(); } }
async function athOptRun() {
  const v = STRAT.v, st = athOptState(v), grids = athOptGrids(v, st); if (st.busy || gridProblem(grids)) return;
  st.busy = true; st.msg = ''; st.res = null; athOptDraw();
  const oq = athOptimizeQuery(v.id, { from: S.from, to: S.to, slippage: S.slippage, params: decodeP(S.p), grids });
  if (isAth4(v.id)) oq.set('sheet', '1');
  { const pk = uniPick(v.id); if (pk.b && !pk.isDefault) { oq.set('watchlist', pk.set === 'current' ? pk.b.current : pk.b.cp); if (pk.set === 'past') oq.set('exclude_watchlist', pk.b.current); } } // the basket chosen on the backtest page
  const q = oq.toString();
  try {
    let d;
    for (let attempt = 0; ; attempt++) {
      const r = await fetch(`${API_IN}/guru/yt/ath-optimize?${q}`, { signal: AbortSignal.timeout(150000) });
      if (r.status === 429) { if (attempt) throw Error(UNIVERSE_BUSY); await new Promise(ok => setTimeout(ok, 4000)); continue; }
      if (!r.ok) throw Error((await r.text()) || `HTTP ${r.status}`);
      d = await r.json(); break;
    }
    if (!validAthResult(d)) throw Error('The engine returned no complete optimization result.');
    d.slippage = S.slippage; st.res = d; optTriedAdd(d.rows.length);                // the brokerage this optimization ran with, for its sheet's Rules
  } catch (e) { st.msg = e.name === 'TimeoutError' ? 'The optimization reached its time limit. Tick fewer settings or use a bigger step.' : (e.message || 'The optimization failed.'); }
  st.busy = false; athOptDraw();
}
function athOptInit() {
  const root = $('#ath-opt-body'); if (!root || !STRAT?.ath) return;
  const st = athOptState(STRAT.v);
  optFit();
  root.addEventListener('input', e => { const tr = e.target.closest('tr[data-k]'); if (!tr) return; const f = e.target.dataset.f, k = tr.dataset.k;
    if (f === 'on') { st.on[k] = e.target.checked; athOptDraw(); return; }
    const d = athOptParams(btSchema(STRAT.v)).find(x => x.key === k); st.rng[k] = { ...(st.rng[k] || defaultRange({ ...d, def: decodeP(S.p)[k] ?? d.def })), [f]: e.target.value };
    const grids = athOptGrids(STRAT.v, st), problem = gridProblem(grids), n = Object.keys(grids).length ? comboCount(grids) : 0, cnt = $('#ath-opt-count'), btn = $('#ath-opt-run');
    if (cnt) cnt.textContent = (n ? `${n} combinations plus the current settings. ` : '') + problem; if (btn) btn.disabled = !!problem || st.busy || (window.VAULT_APP && window.VAULT_USER?.role === 'viewer');
    const cell = tr.querySelector('td:last-child'); if (cell) cell.textContent = st.on[k] ? grids[k]?.length ?? 0 : 'fixed'; });
  root.addEventListener('click', e => { if (athLabClick(e)) return; if (e.target.closest('#ath-opt-run')) athOptRun(); const b = e.target.closest('[data-ath-sort]'); if (b) { st.sort = b.dataset.athSort; athOptDraw(); } });
}
function athOptimPage(v, tk, crumbs, switches, selection) {
  if (!athOptParams(btSchema(v)).length) return withheldStrategy(v, tk, crumbs, switches, { status: 'not_testable', note: 'This rule has no settings to vary.' });
  STRAT = { v, tk, region: 'in', ath: true };
  return `${crumbs}<section class="box"><div class="st-head"><h1 class="st">Optimization · ${esc(familyDisplayName(v))}</h1><a class="dbtn sm back-btn" href="${href('/s/' + encodeURIComponent(v.id), { ...S })}">${VaultUI.ICON.back} Back to strategy</a></div><p class="rnote">${esc(universeChoiceLabel(uniOf(v.id)))} · ${esc(v.name)}. Each combination is a complete stock-universe portfolio backtest; the engine runs them one after another.</p>${switches}${periodPanel(tk, null)}${v.variant ? `<p class="vname">Variant: <b>${esc(v.variant)}</b></p>` : ''}</section><div id="ath-opt-body">${athOptBody(v)}</div>`;
}
async function viewUniverse(v, crumbs, switches, selection, generation) {
  const tk = 'NIFTY 50';
  if (R.optim && !v.id.startsWith('ytath_')) return withheldStrategy(v, tk, crumbs, switches, { status: 'not_testable', note: 'Parameter optimization is not available for stock-universe runs. Open the backtest page and use Run backtest to change the settings.' });
  const U = uniOf(v.id);
  if (!U) return withheldStrategy(v, tk, crumbs, switches, { status: 'needs_data', note: v.id.startsWith('ytath_') ? 'The ATH stock universe is not loaded in the engine.' : 'The F&O stock universe is not loaded in the engine.' });
  if (R.optim) return athOptimPage(v, tk, crumbs, switches, selection);
  const params = decodeP(selection.p), uq = universeQuery(v.id, { from: selection.from, to: selection.to, slippage: selection.slippage, params });
  if (S.only) uq.set('only', S.only);
  { const pk = uniPick(v.id); if (pk.b && !pk.isDefault) { uq.set('watchlist', pk.set === 'current' ? pk.b.current : pk.b.cp); if (pk.set === 'past') uq.set('exclude_watchlist', pk.b.current); } }
  const query = uq.toString();
  let d = UNIVERSE_RUNS.get(query);
  try {
    if (!d) { d = await fetchUniverse(query); if (d?.status === 'ready') { UNIVERSE_RUNS.set(query, d); while (UNIVERSE_RUNS.size > 4) UNIVERSE_RUNS.delete(UNIVERSE_RUNS.keys().next().value); } }
  } catch (error) { if (generation !== busy) return ''; return withheldStrategy(v, tk, crumbs, switches, { status: 'unavailable', note: error.name === 'TimeoutError' ? 'The stock-universe run reached its time limit. No partial result is displayed.' : error.message }); }
  if (generation !== busy) return '';
  if (d?.status !== 'ready' || !d.stats || !d.dates?.length) return withheldStrategy(v, tk, crumbs, switches, { status: d?.status || 'unavailable', note: d?.note || d?.error || 'No complete stock-universe result is available.' });
  // buy & hold may miss a date: carry the last value so the charts stay continuous
  let last = 1; d.bh = (d.bh || []).map(x => (Number.isFinite(x) ? (last = x) : last));
  const bName = uniBench(d), bText = bName ? `${esc(bName)} buy &amp; hold` : 'No buy &amp; hold (no index for this basket)';
  const s = d.stats, p = d.portfolio || {}, ps = positionStats(d.positions), src = d.src ?? (params.src || 1);
  if (!S.only && uniPick(v.id).isDefault) UNIVERSE_SUMMARY[v.id] = { stats: s, src, max_positions: d.max_positions, pct: d.pct, hash: location.hash }; // a one-stock run is not the basket's result
  const tiles = [['Net Profit', s.net_profit ?? s.total_return, x => pct(x, 2, true), 'Portfolio equity return, net', cls(s.net_profit ?? s.total_return)], ['CAGR', s.cagr, x => pct(x, 2, true), 'Compounded a year', cls(s.cagr)], ['No. of trades', d.trades.length, x => fx(x, 0), (n => n ? `Accepted stock trades, ${fx(n, 0)} still open` : 'Accepted stock trades')(d.trades.filter(t => t.open).length)], ['Win Rate', s.win_rate, x => pct(x, 1), 'Closed trades'], ['Avg trade', s.avg_trade, x => pct(x, 3, true), `Per closed trade, net · profit factor ${fx(s.profit_factor)}`, cls(s.avg_trade)], ['MaxDD', s.max_dd, x => pct(x, 2), 'Portfolio equity decline', 'neg'], ['Sharpe', s.sharpe, x => fx(x), 'Portfolio daily returns · rf = 0']];
 STRAT = { d, v, tk: d.universe || U.universe, region: 'in', tiles, px: null, display: { value: s.net_profit, reportMode: 'Compounded portfolio equity', description: 'Stock-universe portfolio', unavailableReason: null }, indicatorParameters: params, runQuery: query };
  RULE_VISUAL = makeRuleVisualModel({ strategy: { ...v, family_name: familyDisplayName(v) }, parameters: { ...paramValues(btSchema(v), {}), ...params }, symbol: universeChoiceLabel(U) });
  const benchmarkPrices = await ohlc(tk).catch(() => null); if (generation !== busy) return '';
  STRAT.benchPx = benchmarkPrices || (isAth4(v.id) ? await fullOhlc(tk).catch(() => null) : null); if (generation !== busy) return '';
  const sizing = d.pct ? `${fx(d.pct, 2)}% of equity per position` : `100 / ${d.max_positions}% of equity per position`;
  const basis = `<div class="portfolio-basis" data-universe-basis><b>${esc(d.max_positions)} simultaneous positions · ${esc(sizing)}${d.pad ? ' · padded and aligned to NIFTY' : ''}.</b> ${esc(p.sizing || '')}.<br>${esc(p.ranking || '')}. ${esc(p.timing || '')}.<br>${fx(p.candidates, 0)} signals · ${fx(p.accepted, 0)} accepted · ${fx(p.rejected_capacity, 0)} over the position limit · ${fx(p.rejected_cash, 0)} without cash · ${fx(p.last_day_signals_dropped, 0)} on a symbol's last data day (never entered)${ps ? ` · on average ${fx(ps.avg, 1)} open positions (most ${fx(ps.max, 0)}), invested on ${pct(ps.invested, 0)} of days` : ''}${Number.isFinite(p.seconds) ? ` · computed in ${fx(p.seconds, 1)} s` : ''}.</div>`;
  const period = `<p class="rnote" data-results-provenance><b>Computed stock-universe portfolio · ${esc(universeChoiceLabel(U))}.</b> Period ${esc(d.dates[0])} to ${esc(d.dates.at(-1))} (data coverage ${esc(U.first)} to ${esc(U.last)}). Brokerage ${fx(+(d.costs?.total_side_bps ?? selection.slippage ?? 0) / 100, 2)}% a side, taken off returns; trade prices unchanged. ${esc((d.notices || []).join(' '))}</p>`;
  return `${crumbs}<section class="box" data-universe-result><div class="st-head"><h1 class="st">${esc(familyDisplayName(v))}</h1>${VaultUI.toolbar()}</div>${switches}${btstInstrumentNote(v, tk)}${periodPanel(tk, d)}${v.variant ? `<p class="vname">Variant: <b>${esc(v.variant)}</b></p>` : ''}${period}
    <div class="metrics">${tiles.map(([k, x, f, i, c], n) => `<div class="metric"><span>${k}</span><b class="${c || ''}" data-tile="${n}">${f(x)}</b><i>${i}</i></div>`).join('')}</div>${basis}</section>
  <section class="box"><div class="box-h">Rules <span data-react-slot="visual-rules"></span><small>implemented interpretation of the team-supplied AFL rules, run on every stock of the watchlist</small></div>${rulesPanel(v)}</section>
  ${referenceBox(v, tk)}
  <section class="box"><div class="box-h">Portfolio equity <span class="legend"><span><i class="sw-line" style="background:var(--blue)"></i>Portfolio</span><label class="dcheck"><input type="checkbox" id="cmp"${bName ? '' : ' disabled'}> <i class="sw-line" style="background:var(--bh)"></i>${bName ? `Compare with ${bText}` : bText}</label><label class="dcheck"><input type="checkbox" id="logs" checked> Log scale</label></span></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-eq">Download PNG</button><button class="dbtn sm" data-report-export="equity">Download Excel</button></div><div class="lwc" id="c-eq"></div><div id="equity-navigator" class="equity-navigator"><div class="year-shortcuts"><button class="dbtn sm" data-equity-year="all">All years</button>${[...new Set(d.dates.map(x => x.slice(0, 4)))].map(y => `<button class="dbtn sm" data-equity-year="${y}">${y}</button>`).join('')}</div><label>Start <input type="range" data-range="start" min="0" aria-label="Equity range start"></label><label>End <input type="range" data-range="end" min="0" aria-label="Equity range end"></label><output></output></div><div class="cap">Cash plus open positions marked each day · portfolio ${d.equity.at(-1).toFixed(2)}× (gross ${(d.gross_equity?.at(-1) ?? NaN).toFixed(2)}×) · NIFTY futures buy &amp; hold ${d.bh.at(-1).toFixed(2)}×</div></section>
  <div class="two"><section class="box"><div class="box-h">Drawdown <small>worst ${pct(s.max_dd, 1)}</small></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-dd">Download PNG</button></div><div class="bkc" id="c-dd"></div></section>
    <section class="box"><div class="box-h">Calendar-year returns <span class="legend"><span><i class="sw-box" style="background:var(--blue)"></i>Portfolio</span><span><i class="sw-box" style="background:var(--bh)"></i>${bText}</span></span></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-yr">Download PNG</button></div><div class="bkc" id="c-yr"></div></section></div>
  ${d.positions?.length ? `<section class="box"><div class="box-h">Open positions <small>at each day's close · limit ${esc(d.max_positions)}</small></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-pos">Download PNG</button></div><div class="bkc" id="c-pos"></div></section>` : ''}
  ${adrReportHTML(d)}${VaultUI.benchmarkHTML(d, tk, benchmarkPrices)}${VaultUI.detailHTML(d)}
  <section class="box"><div class="box-h">Monthly returns (%) <span class="legend"><small>${Object.keys(periods(d, 4))[0]} – ${Object.keys(periods(d, 4)).slice(-1)[0]} · darker = larger</small><span class="join sm"><button class="join-item ${HEAT_DESC ? '' : 'active'}" data-heat="asc">Oldest first</button><button class="join-item ${HEAT_DESC ? 'active' : ''}" data-heat="desc">Newest first</button></span></span></div><div class="chart-tools"><label class="dfield">Colors <select id="heat-palette"><option value="classic">Green / red</option><option value="ocean">Blue / orange</option><option value="violet">Teal / magenta</option><option value="mono">Single hue</option></select></label><button class="dbtn sm" data-report-export="monthly">Download Excel</button>${isAdr()?'<button class="dbtn sm" id="adr-yoy-xlsx" title="Year x month grid in percent units, with Total and Avg rows">Download YOY Excel</button>':''}</div><div class="heat-scroll"><div id="heatbox">${heatTable(d)}</div></div><p class="rnote">Calendar returns of the portfolio equity including open-position marks. Avg/Total is the arithmetic mean of displayed calendar-year returns.</p></section>
  <section class="box" data-tradesheet data-universe-tradesheet><div class="box-h">Tradesheet <span class="legend"><small>${fx(d.trades.length, 0)} trades · ${esc(d.trades.length ? `${String(d.trades[0].entry).slice(0, 4)}–${String(d.trades.at(-1).entry).slice(0, 4)}` : '')} · newest first</small><button class="dbtn sm" id="trades-xlsx">Download Excel</button><button class="dbtn sm" id="csv">Download CSV</button></span></div><p class="rnote">One row per stock trade: entry and exit bar (date and time), fill prices as stored in the database, whole shares, weight = position value / portfolio equity at entry, score = the AFL's PositionScore (higher first), return net of brokerage.</p>${(() => { Object.assign(UNI_SHEET, { symbol: '', year: '', page: 1 }); const t = universeTradeSheet(d);
    return `<div class="uni-toolbar"><label class="dfield">Symbol <input type="search" id="universe-symbol" placeholder="Filter, e.g. RELIANCE" autocomplete="off"></label><span class="uni-nav" id="universe-nav">${t.top}</span></div><div class="tlog" id="universe-trades">${t.table}</div><div class="uni-toolbar uni-foot" id="universe-nav-bottom">${t.bottom}</div>`; })()}</section>`;
}

const BASKET_RUNS=new Map();
async function viewBasket(v,tk,crumbs,switches,selection,api,generation){
  if(R.optim)return withheldStrategy(v,tk,crumbs,switches,{status:'not_testable',note:'Basket parameter optimization is not yet admitted. Open the basket backtest page or choose an individual Stock for existing parameter research.'});
  if(!instrumentControls(v.instrument_policy,v.id,v.book).basket)return withheldStrategy(v,tk,crumbs,switches,{status:'needs_data',note:instrumentControls(v.instrument_policy,v.id,v.book).note||'Basket execution is unavailable for this source method.'});
  const inventory=BASKETS.find(b=>b.id===selection.basket),counts=inventory?.source_counts;
  if(counts&&Number.isInteger(counts.expected)&&counts.expected>inventory.symbols.length)return withheldStrategy(v,tk,crumbs,switches,{status:'needs_data',note:`This source list has ${counts.expected} constituents but only ${inventory.symbols.length} admitted price series. The complete basket is withheld; missing stocks are not silently excluded. ${inventory.note||''}`});
  const query=new URLSearchParams({id:v.id,basket:selection.basket,from:selection.from,to:selection.to,cost:'0',...VaultUI.costState(selection),max_positions:selection.max_positions,membership:selection.membership,...Object.fromEntries(Object.entries(decodeP(selection.p)).map(([k,value])=>['p.'+k,value]))});
  const key=api+'|'+query.toString();let d=BASKET_RUNS.get(key);
  try{
    if(!d){const response=await fetch(enginePath(v.id,'basket-run',api)+'?'+query,{signal:AbortSignal.timeout(95000)});if(!response.ok)throw Error(await response.text());d=await response.json();if(d.status==='ready'){BASKET_RUNS.set(key,d);while(BASKET_RUNS.size>4)BASKET_RUNS.delete(BASKET_RUNS.keys().next().value);}}
  }catch(error){if(generation!==busy)return '';return withheldStrategy(v,tk,crumbs,switches,{status:'unavailable',note:error.name==='TimeoutError'?'Basket request reached its time limit. No partial result is displayed; retry with a shorter covered period.':error.message});}
  if(generation!==busy)return '';
  if(d?.status!=='ready'||!d.stats||!d.dates?.length)return withheldStrategy(v,tk,crumbs,switches,{status:d?.status||'unavailable',note:d?.blocked||d?.note||'No complete admitted basket result is available.'});
  const ruleSchema=await optSchema(api);if(generation!==busy)return '';
  const ruleDefaults=Object.fromEntries((Array.isArray(ruleSchema[v.id])?ruleSchema[v.id]:[]).filter(row=>typeof row.key==='string'&&Number.isFinite(row.def)).map(row=>[row.key,row.def]));
  const s=d.stats,p=d.portfolio||{},name=BASKETS.find(b=>b.id===selection.basket)?.name||selection.basket;
  const tiles=[['Net Profit',s.net_profit,x=>pct(x,2,true),'Closed allocated return',cls(s.net_profit)],['Avg net pnl',s.avg_net_pnl,x=>pct(x,2,true),'Per closed stock trade',cls(s.avg_net_pnl)],['No. of trades',s.trades,x=>fx(x,0),'Accepted closed positions'],['Win Rate',s.win_rate,x=>pct(x,1),'Closed trades'],['Expectancy',s.expectancy_ratio,x=>fx(x),'Per unit of average loss'],['MaxDD',s.max_dd,x=>pct(x,2),'Portfolio equity decline','neg'],['Sharpe',s.sharpe,x=>fx(x),'Portfolio daily returns · rf = 0']];
  STRAT={d,v,tk:'Basket · '+name,region:selection.region,tiles,px:null,display:{value:s.net_profit,reportMode:'Equal-slot closed portfolio contribution',description:d.accounting_basis,unavailableReason:null}};
  RULE_VISUAL=makeRuleVisualModel({strategy:v,run:d,parameters:{...ruleDefaults,...decodeP(selection.p)},symbol:name,basketPriceApi:api,sourceTickers:selection.region==='us'?TICK_ALL_US:TICK_ALL_IN});
  const coverage=`<details class="box" data-basket-coverage><summary>Constituent coverage · ${esc(p.source_members)} source members</summary><div class="tlog"><table class="tbl"><thead><tr><th>Symbol</th><th>Source first</th><th>Source last</th><th>Status</th><th>Coverage notes</th></tr></thead><tbody>${(d.coverage||[]).map(c=>`<tr><td>${esc(c.symbol)}</td><td>${esc(c.first)}</td><td>${esc(c.last)}</td><td>${esc(c.status)}</td><td>${esc(c.note)}</td></tr>`).join('')}</tbody></table></div></details>`;
  const benchmark=d.benchmark_symbol||(selection.region==='us'?'SPX':'NIFTY 50');
  const benchmarkPrices=await ohlc(benchmark).catch(()=>null);if(generation!==busy)return '';
  return `${crumbs}<section class="box" data-basket-result><div class="st-head"><h1 class="st">${esc(familyDisplayName(v))}</h1>${VaultUI.toolbar()}</div>${gateNotice(v,tk)}${switches}${periodPanel(tk,d)}${optimBanner(v)}<p class="rnote" data-results-provenance><b>Computed basket application · ${esc(name)} · ${selection.region==='us'?'US':'India'}.</b> ${calculatedPeriodHTML(d)}${esc((d.notices||[]).join(' '))}</p><div class="metrics">${tiles.map(([k,x,f,i,c],n)=>`<div class="metric"><span>${k}</span><b class="${c||''}" data-tile="${n}">${f(x)}</b><i>${i}</i></div>`).join('')}</div><p class="rnote" data-return-definition><b>Basket Net Profit = sum of accepted closed-trade returns × their initial-equity slot allocations.</b> Avg net pnl is the source-unit average per stock trade. Avg × trade count gives an unallocated return sum, not the basket return.</p><div class="portfolio-basis"><b>${esc(selection.max_positions)} simultaneous positions · 1/${esc(selection.max_positions)} per full position.</b> ${esc(p.allocation_basis)}<br>${esc(p.ranking)}. ${esc(p.signal_mode)}.<br>${esc(p.timing_basis)}<br>${esc(p.accepted_positions)} accepted positions · ${esc(p.rejected_capacity)} capacity rejections · ${esc(p.rejected_cash)} cash rejections. ${esc(p.gross_basis)}</div></section>
  <section class="box"><div class="box-h">Rules <span data-react-slot="visual-rules"></span><small>Source stock rule with a declared equal-slot portfolio application</small></div>${rulesPanel(v)}</section>${coverage}
  <section class="box"><div class="box-h">Basket equity <span class="legend"><span><i class="sw-line" style="background:var(--blue)"></i>Portfolio</span><label class="dcheck"><input type="checkbox" id="cmp">Compare with ${esc(LABEL[benchmark]||benchmark)} buy &amp; hold</label><label class="dcheck"><input type="checkbox" id="logs" checked>Log scale</label></span></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-eq">Download PNG</button><button class="dbtn sm" data-report-export="equity">Download Excel</button></div><div class="lwc" id="c-eq"></div><div id="equity-navigator" class="equity-navigator"><div class="year-shortcuts"><button class="dbtn sm" data-equity-year="all">All years</button>${[...new Set(d.dates.map(date=>date.slice(0,4)))].map(year=>`<button class="dbtn sm" data-equity-year="${year}">${year}</button>`).join('')}</div><label>Start <input type="range" data-range="start" min="0" aria-label="Equity range start"></label><label>End <input type="range" data-range="end" min="0" aria-label="Equity range end"></label><output></output></div><div class="cap">Cash plus actual simultaneous position marks · ${d.equity.at(-1).toFixed(2)}× final normalized equity, including open marks</div></section>
  <div class="two"><section class="box"><div class="box-h">Drawdown <small>worst ${pct(s.max_dd,1)}</small></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-dd">Download PNG</button></div><div class="bkc" id="c-dd"></div></section><section class="box"><div class="box-h">Calendar-year returns</div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-yr">Download PNG</button></div><div class="bkc" id="c-yr"></div></section></div>
  ${VaultUI.benchmarkHTML(d,benchmark,benchmarkPrices)}${VaultUI.detailHTML(d)}<section class="box"><div class="box-h">Monthly returns (%)</div><div class="chart-tools"><button class="dbtn sm" data-report-export="monthly">Download Excel</button></div><div id="heatbox">${heatTable(d)}</div><p class="rnote">Calendar returns use the basket equity including held-position marks. Avg/Total is the arithmetic mean of displayed calendar-year returns.</p></section>${VaultUI.tradeHTML(d,STRAT.tk)}`;
}

function rulesPanel(v) {
  // YT rule templates show the page's current parameter values (rule-template.ts); guarded lines may drop out
  const sec = { F: [], E: [], A: [], X: [] }; ruleLines(v, isYt(v) ? decodeP(S.p) : {}).forEach(([t, r]) => (sec[t] || sec.E).push(r));
  const act = r => /^(Buy|Short|Sell|Cover|Next day|No other signal|Once|After|Stop)/.test(r);
  const col = (t, sub, c, items) => items.length ? `<div class="rcol"><h4><i style="background:${c}"></i>${t}</h4><p class="rsub">${sub}</p><ul>${items.map(r => `<li class="${act(r) ? 'act' : ''}">${esc(r)}</li>`).join('')}</ul></div>` : '';
  return `<div class="rgrid">${col('Conditions', 'All must be true (trend, liquidity, volatility filters)', 'var(--ink)', sec.F)}${col('Entry', v.guru==='ross'?'Source entry conditions; unresolved choices remain explicit':'When all of these happen on a bar’s close', 'var(--blue)', sec.E)}${col('Scale-in', 'While the trade is open', '#8b93a1', sec.A)}${col('Exit', v.guru==='ross'?'Source management; a complete automatic exit is not assumed':isYt(v)&&v.id.startsWith('ytmr_')?'The exit type is a toggle; the optional stops apply alongside it':sec.X.length > 1 ? 'Whichever happens first' : 'How the trade is closed', 'var(--red)', sec.X)}</div>${v.notes ? `<p class="rnote">${esc(v.notes)}</p>` : ''}`;
}
async function viewPristineStrategy(v,tk,crumbs,selection,runApi,generation,optimization){
 const stocks=TICK_ALL_US.flatMap(row=>pristineInstrumentContext(row.ticker,'us',[row])?[{ticker:row.ticker,symbol:row.symbol}]:[]);
 const snapshot={data:null,view:null,definition:v,selection:{...selection,tk},stocks,availability:null,optimization:!!optimization},shell=crumbs+VaultUI.toolbar();
 const hold=note=>{snapshot.availability={status:'not_testable',note};STRAT={v,tk,region:selection.region,pristine:snapshot};return shell;};
 if(selection.region!=='us')return hold('This selected research mode requires an actual US/USD native stock. Choose US and a supplied stock; no index or INR substitution is made.');
 if(selection.run_scope==='basket')return hold('Full basket results remain unavailable until the separate coordinator, membership/coverage and model/presenter admission is complete.');
 if(selection.research_mode!==PRISTINE_RESEARCH_MODE)return hold('Choose the supplied-series-unit research basis explicitly before running. Historical-dollar equivalence is not certified.');
 if(selection.p)return hold('These source branches have fixed rules and no parameters. Unknown settings are not silently replaced.');
 if(!pristineInstrumentContext(tk,selection.region,TICK_ALL_US))return hold('Select one uniquely qualified native US/USD individual stock. ETFs, indices and ambiguous sources are not admitted.');
 if(optimization)return hold('Fixed source rules; no parameter grid or optimizer search is available. Open the ordinary strategy report to run the selected stock.');
 try{pristineCost(selection.cost);pristineCost(selection.slippage);pristineCost(selection.rollover);}catch(error){return hold(error.message);}
 const query=new URLSearchParams({id:v.id,ticker:tk,from:selection.from||'',to:selection.to||'',cost:selection.cost??'0',...pristineExecution(selection),research_mode:selection.research_mode});
 try{
  const readinessUrl=enginePath(v.id,'readiness',runApi)+'?'+query;
  const readiness=await(PRISTINE_READY[readinessUrl]||(PRISTINE_READY[readinessUrl]=fetch(readinessUrl).then(async response=>{if(!response.ok)throw Error(await response.text());return response.json();}).catch(error=>{delete PRISTINE_READY[readinessUrl];throw error;})));
  if(generation!==busy)return '';
  const ready=Array.isArray(readiness?.strategies)?readiness.strategies.find(row=>row.id===v.id):null;
  if(!ready||ready.status!=='ready')return hold(ready?.note||'The source/data/accounting readiness check is unavailable for this exact request.');
  const data=await runData(v.id,tk,selection.from,selection.to,selection.cost,selection.p,runApi,{...pristineExecution(selection),research_mode:selection.research_mode});
  if(generation!==busy)return '';
  if(!data||data.error)return hold(apiError(data?.error||'No response from the Pristine engine',tk));
  const view=validatedPristineReport(data,pristineAdmission(v,tk,selection));
  snapshot.data=data;snapshot.view=view;snapshot.availability={status:'ready',note:ready.note||''};
  STRAT={v,tk,region:selection.region,pristine:snapshot};return shell;
 }catch(error){if(generation!==busy)return '';return hold(error.message||'The selected Pristine response could not be validated.');}
}

async function viewStrategy() {
  const generation=busy,selection={...S},runApi=API,optimization=R.optim;
  let v = CAT.find(x => x.id === R.id);
  if(!v&&pristineMethod(R.id,PRISTINE_BOOK))v=ALLCAT.find(x=>x.id===R.id);
  if (!v) { const other = ALLCAT.find(x => x.id === R.id), sib = other && CAT.find(x => x.family === other.family && x.status === other.status) || other && CAT.find(x => x.family === other.family);
    if (sib) { setTimeout(() => go('/s/' + encodeURIComponent(sib.id), S), 0); return bookLoader(`Switching to the ${S.region === 'us' ? 'NYSE' : 'NSE'} version…`); } } // same rule, this market's data
  if (v) bookCtx(v.book);
  if (v && isYt(v)) { try { await optSchema(runApi); } catch (e) { /* rules then show their templates' defaults */ } }
  if (v && !R.optim) { const ref = btRef(v, S.tk === 'ALL' ? 'NIFTY 50' : S.tk); if (ref) { const raw = new URLSearchParams(location.hash.split('?')[1] || '');
    let pp = raw.has('p') ? S.p : encodeP(ref.params);
    { const q = decodeP(pp); if ('src' in q && q.src !== 1) { delete q.src; pp = encodeP(q); } } // only the main database is offered
    const sizing = decodeP(pp).sizing === 2 ? 2 : 1;
    // the Net Profit display follows the position sizing: compounding -> compounded returns, fixed quantity -> rupee result
    const want = { ...S, p: pp, slippage: raw.has('slippage') ? S.slippage : String(ref.cost), report_mode: raw.has('report_mode') ? S.report_mode : sizing === 2 ? 'fixed' : 'compound' }, target = href(R.path, want); // the Vault's per-side cost setting is 'slippage' (fees are fixed at 0 outside Pristine)
    if (target !== location.hash) { location.replace(target); return bookLoader('Loading the reference settings…'); } } }
  if (!v) return `<nav class="crumbs"><a href="#/">Guru's</a></nav><div class="empty"><b>This strategy is not in the vault</b>The link may be old.</div>`;
  const g = GURU[v.guru], b = BOOK[v.book], fam = CAT.filter(x => x.family === v.family), tk = S.tk === 'ALL'&&!pristineMethod(v.id,v.book) ? TICKERS[0].ticker : S.tk;
  const ytMain = ytMainVariant(v.group), crumbs = isYt(v) ? `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span><a href="${href('/yt',{tk:S.tk,from:S.from,to:S.to})}">YT Strategies</a><span>/</span>${ytMain ? `<a href="${href('/yt',{tk:S.tk,from:S.from,to:S.to,family:v.family})}">${esc(familyDisplayName(v))}</a><span>/</span><a href="${href('/yt',{tk:S.tk,from:S.from,to:S.to,family:v.family,grp:ytMain,mode:'variants'})}">${esc(ytMain)}</a><span>/</span><span>${esc(v.name)}</span>` : `<span>${esc(familyDisplayName(v))}</span>`}<a class="back" href="${LIST_HASH || href('/yt')}">Back to strategies</a></nav>` : `<nav class="crumbs"><a href="#/">Guru's</a><span>/</span><a href="${guruLink(g.id)}">${esc(g.name)}</a><span>/</span><a href="${href('/guru/' + g.id, {view:'grid',book:b.id,tk:S.tk,basket:S.basket,from:S.from,to:S.to})}">${esc(b.title)}</a><span>/</span><span>${esc(familyDisplayName(v))}</span><a class="back" href="${LIST_HASH || guruLink(g.id)}">Back to strategies</a></nav>`;
  if(pristineMethod(v.id,v.book))return viewPristineStrategy(v,tk,crumbs,selection,runApi,generation,optimization);
  const switches = marketControls(v,tk);
  if(universeScope(v))return viewUniverse(v,crumbs,switches,selection,generation);
  const selectedBasket=BASKETS.find(b=>b.id===S.basket);
  if(S.run_scope==='basket'&&S.basket&&(!['ready','partial'].includes(selectedBasket?.price_status)||!selectedBasket.symbols.includes(tk)))return withheldStrategy(v,tk,crumbs,switches,{status:'needs_data',note:selectedBasket?.note||'Data missing: verified constituent prices and universe coverage for this basket are not available. Index prices cannot substitute for basket constituents.'});
  if(selection.run_scope==='basket')return viewBasket(v,tk,crumbs,switches,selection,runApi,generation);
  const availability=(await getReadiness(tk,selection.from,selection.to,v.id,selection.p))[v.id] || {status:'unavailable',note:'The data requirements could not be verified.'};
  if(generation!==busy)return '';
  if(availability.status!=='ready') return withheldStrategy(v,tk,crumbs,switches,availability);
  const schemaRequest=optSchema(),chartRequest=optimization?Promise.resolve(null):fullOhlc(tk,runApi).catch(()=>null);
  const d=await runData(v.id,tk,selection.from,selection.to,selection.cost,selection.p,runApi,VaultUI.costState(selection));
  if(generation!==busy)return '';
  if (!d || d.error) return withheldStrategy(v,tk,crumbs,switches,{status:'unavailable',note:apiError(d?.error || 'No response from the calculation engine',tk)});
  await schemaRequest;
  if(generation!==busy)return '';
  let account,unit;const parameters={...bookDefaults(v.id),...decodeP(selection.p)};try{unit=unitExposureMethod(v.id,v.book)||Object.hasOwn(d,'unit_exposure_report')?validatedUnitExposureReport(d,unitAdmission(v,tk,parameters,selection.region,selection)):null;account=sourceAccount(d,v.id,v.book,tk,sourceAdmission(v,tk,parameters,selection.region));}catch(error){return withheldStrategy(v,tk,crumbs,switches,{status:'calculation_unavailable',note:error.message});}
  const s = d.stats, cl = v.claim, vd = verdictOf(s), closed = d.trades.filter(t => !t.open), chk = closed.map(exitCheck).filter(check=>check?.available), pass = chk.filter(x => x.ok).length;
  const display=returnDisplay(d,selection.report_mode,account,unit);
  const tiles = [[account||unit?display.label:'Net Profit',display.value,x=>pct(x,2,true),display.reportMode,cls(display.value)],['Avg net pnl',s.avg_net_pnl??s.avg_trade,x=>pct(x,2,true),account?'Weighted per closed trade':'Per closed trade',cls(s.avg_trade)],['No. of trades',s.trades,x=>fx(x,0),'Closed positions'],['Win Rate',s.win_rate,x=>pct(x,1),'Closed trades'],['Expectancy',s.expectancy_ratio,x=>fx(x),'Per unit of average loss'],['MaxDD',s.max_dd,x=>pct(x,2),account?sourceAccountCopy(account).decline:'Largest equity decline','neg'],['Sharpe',s.sharpe,x=>fx(x),account?'Account daily returns · rf = 0':'Daily returns · rf = 0']];
  const px=optimization?null:(await chartRequest)||d.ohlc||await ohlc(tk);
  if(generation!==busy)return '';
  STRAT = { d, v, tk,region:selection.region, tiles, px, display, account,unit, sourceView:account?sourceAccountView(account):null, indicatorParameters:parameters };
  RULE_VISUAL=makeRuleVisualModel({strategy:{...v,family_name:familyDisplayName(v)},run:d,prices:px,ticker:tk,parameters:STRAT.indicatorParameters,symbol:LABEL[tk]||tk});
  STRAT.indicators=buildStrategyIndicators({id:v.id,book:v.book,ticker:tk,prices:px,fullHistory:px?.full_history===true,parameters:STRAT.indicatorParameters,sourceReport:d});
  if(R.optim) return `${crumbs}<section class="box"><div class="st-head"><h1 class="st">Optimization · ${esc(familyDisplayName(v))}</h1><a class="dbtn sm back-btn" href="${href('/s/'+encodeURIComponent(v.id),{...S})}">${VaultUI.ICON.back} Back to strategy</a></div><p class="rnote">${esc(LABEL[tk])} · ${esc(v.name)}. ${unit?'Only the fixed DEMA16/8 and raw-Buy clock1 configuration is reviewed. Alternative settings and optimizer searches do not inherit this admission.':account?sourceAccountCopy(account).bandy?'Only the fixed reviewed method and allocation are available. Other parameter settings require their own review; closed-display modes cannot resize the cash account.':'Only the fixed reviewed source/account configuration is available. Other indicator, stop and risk settings remain held; closed-display modes cannot resize the account.':'Parameter searches are research; selected settings and before/after displays do not establish an untouched holdout. Net profit sums closed-trade net returns; CAGR and risk measures use the separate compounded model.'}</p>${gateNotice(v,tk)}${experimentNotice(v,d)}${switches}${periodPanel(tk,d)}${optimBanner(v)}${account?'<div data-react-slot="source-account-notice"></div>':unit?'<div data-react-slot="unit-exposure-notice"></div>':''}</section>${unit?'<section class="box"><div class="box-h">Fixed reviewed index configuration</div><p class="rnote">DEMA first length16 / second length8 / Bars since raw Buy1. Only this source-reviewed configuration is admitted; changed parameters require separate review. No optimization or book-performance claim is made.</p></section>':account?sourceAccountCopy(account).bandy?`<section class="box"><div class="box-h">Fixed reviewed cash-account configuration</div><p class="rnote">${esc(v.name)}. ${esc(sourceAccountCopy(account).sizing)} The account allocation stays fixed; new configurations require their own review.</p></section>`:'<section class="box"><div class="box-h">Fixed protected configuration</div><p class="rnote">EMA13 / MACD12-26-9 / weekly EMA26 / SafeZone10-2 / source2%-6% risk. The account controller stays fixed; new configurations require their own review.</p></section>':await optimSection(v,tk,d,true)}`;
  return `${crumbs}
    <section class="box"><div class="st-head"><h1 class="st">${esc(familyDisplayName(v))}</h1>${VaultUI.toolbar()}</div>
      ${gateNotice(v,tk)}${experimentNotice(v,d)}${switches}${btstInstrumentNote(v,tk)}${periodPanel(tk, d)}${optimBanner(v)}${account?'<div data-react-slot="source-account-notice"></div>':unit?'<div data-react-slot="unit-exposure-notice"></div>':''}${v.variant ? `<p class="vname">Variant: <b>${esc(v.variant)}</b></p>` : ''}
      <div class="metrics">${tiles.map(([k, x, f, i, c], n) => `<div class="metric"><span>${k}</span><b class="${c || ''}" data-tile="${n}">${f(x)}</b><i>${i}</i></div>`).join('')}</div>
      </section>
    <section class="box"><div class="box-h">Rules <span data-react-slot="visual-rules"></span><small>${isYt(v)?'implemented interpretation of the team-supplied AFL rules':`${v.guru==='chande'?'Source interpretation and declared adaptation':'implemented interpretation of the'} ${v.book === 'stc9' ? 'course' : 'book'}, ${srcOf(v)}`}</small></div>${rulesPanel(v)}</section>
    ${referenceBox(v, tk)}
    ${isYt(v)?'':`<section class="box" data-book-reference><div class="box-h">Book-reported figures <small>reference only · separate from the computed backtest</small></div>${TRUST.bookReference(cl)}</section>`}
    <section class="box"><div class="box-h">${unit?'Canonical unit-exposure model':account?sourceAccountCopy(account).accountEquity:'Compounded return model'} <span class="legend"><span><i class="sw-line" style="background:var(--blue)"></i>${account?'Account':'Strategy'}</span><label class="dcheck"><input type="checkbox" id="cmp"> <i class="sw-line" style="background:var(--bh)"></i>Compare with buy &amp; hold</label><label class="dcheck"><input type="checkbox" id="logs" checked> Log scale</label></span></div>
      <div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-eq">Download PNG</button><button class="dbtn sm" data-report-export="equity">Download Excel</button></div><div class="lwc" id="c-eq"></div><div id="equity-navigator" class="equity-navigator"><div class="year-shortcuts"><button class="dbtn sm" data-equity-year="all">All years</button>${[...new Set(d.dates.map(x=>x.slice(0,4)))].map(y=>`<button class="dbtn sm" data-equity-year="${y}">${y}</button>`).join('')}</div><label>Start <input type="range" data-range="start" min="0" aria-label="Equity range start"></label><label>End <input type="range" data-range="end" min="0" aria-label="Equity range end"></label><output></output></div><div class="cap">${account?'Actual source-account return '+pct(s.total_return,2,true)+' · including open marks · account':'Strategy'} ${d.equity[d.equity.length - 1].toFixed(2)}× · buy &amp; hold ${d.bh[d.bh.length - 1].toFixed(2)}× · drag to zoom, hover for values</div></section>
    <section class="box"><div class="box-h">Price &amp; signals · ${esc(LABEL[tk])} <span class="legend"><span><i class="mk up"></i>Buy / cover</span><span><i class="mk ex"></i>Sell / short</span><span><i class="zk win"></i>Winning trade</span><span><i class="zk loss"></i>${unit?'Non-winning closed trade':'Losing trade'}</span><span><i class="zk open"></i>Open</span><span class="muted">shaded from entry day to exit day · zoom in for each return</span>
        <span class="join sm">${[["D", "Daily"], ["W", "Weekly"], ["M", "Monthly"], ...(isIntradayStrategy(v.needs_series) ? [["I", "5 min"]] : [])].map(([k, t]) => `<button class="join-item ${TF === k ? "active" : ""}" data-tf="${k}">${t}</button>`).join("")}</span><span class="join sm"><button class="join-item" id="px3m" style="display:none">Last 3 months</button><button class="join-item active" id="px3">Last 3 years</button><button class="join-item" id="px1">Last year</button><button class="join-item" id="pxall">Show all</button></span></span></div>
      <div class="pxread" id="pxread">Hover the chart: open, high, low, close and the trade you were in.</div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-px">Download PNG</button></div><div class="lwc px" id="c-px"></div>${indicatorControlsHTML(STRAT.indicators,true)}</section>
    <div class="two"><section class="box"><div class="box-h">Drawdown <small>worst ${pct(s.max_dd, 1)} · ${fx(s.max_dd_days, 0)} ${unit?'observed sessions':'days'} under water</small></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-dd">Download PNG</button></div><div class="bkc" id="c-dd"></div></section>
      <section class="box"><div class="box-h">Calendar-year returns <span class="legend"><span><i class="sw-box" style="background:var(--blue)"></i>Strategy</span><span><i class="sw-box" style="background:var(--bh)"></i>Buy &amp; hold</span></span></div><div class="chart-tools"><button class="dbtn sm" data-chart-snapshot="c-yr">Download PNG</button></div><div class="bkc" id="c-yr"></div></section></div>
    ${adrReportHTML(d)}
    ${VaultUI.benchmarkHTML(d,isUS()?'SPX':'NIFTY 50',await ohlc(isUS()?'SPX':'NIFTY 50'))}${VaultUI.detailHTML(d)}
    <section class="box"><div class="box-h">Monthly returns (%) <span class="legend"><small>${Object.keys(periods(d, 4))[0]} – ${Object.keys(periods(d, 4)).slice(-1)[0]} · follows the backtest period · darker = larger</small><span class="join sm"><button class="join-item ${HEAT_DESC ? '' : 'active'}" data-heat="asc">Oldest first</button><button class="join-item ${HEAT_DESC ? 'active' : ''}" data-heat="desc">Newest first</button></span></span></div><div class="chart-tools"><label class="dfield">Colors <select id="heat-palette"><option value="classic">Green / red</option><option value="ocean">Blue / orange</option><option value="violet">Teal / magenta</option><option value="mono">Single hue</option></select></label><button class="dbtn sm" data-report-export="monthly">Download Excel</button>${isAdr()?'<button class="dbtn sm" id="adr-yoy-xlsx" title="Year x month grid in percent units, with Total and Avg rows">Download YOY Excel</button>':''}</div><div class="heat-scroll"><div id="heatbox">${heatTable(d)}</div></div><p class="rnote">Avg/Total is the arithmetic mean of displayed calendar-year returns, including partial years; it is not CAGR. Zero returns display 0.00; absent months stay empty. ${unit?esc(UNIT_VISUAL_BASIS):''}</p></section>
    ${VaultUI.tradeHTML(d,tk)}

    ${unit?'<section class="box"><div class="box-h">Reviewed parameter scope</div><p class="rnote">Fixed DEMA16/8 and one observed bar since raw Buy. Alternate parameters, instrument classes and execution conventions require separate review.</p></section>':account?'<section class="box"><div class="box-h">Reviewed parameter scope</div><p class="rnote">This protected account is limited to the fixed reviewed configuration. Changed indicator, stop or risk settings need separate source and ledger review; no optimization selection inherits this acceptance.</p></section>':await optimSection(v, tk, d, false)}`;
}
let RULE_VISUAL=null;
let STRAT = null, CHARTS = [], METRIC_ANIMATIONS = [],INDICATOR_CHARTS=null,REPORT_OBSERVER=null,SIGNAL_REQUEST=0,SIGNAL_PENDING=null;
function finishMetricAnimations(){METRIC_ANIMATIONS.forEach(a=>a.stop?.());METRIC_ANIMATIONS=[];if(!STRAT?.tiles)return;document.querySelectorAll('[data-tile]').forEach(el=>{const tile=STRAT.tiles[+el.dataset.tile];if(tile)el.textContent=tile[2](tile[1]);});}

// ---------------------------------------------------------------- Bklit-style interactive SVG charts (gradient area, crosshair + tooltip, spring bars)
function tip(el) { let t = el.querySelector('.bk-tip'); if (!t) { t = document.createElement('div'); t.className = 'bk-tip'; el.appendChild(t); } return t; }
const pctAx = v => (v * 100).toFixed(0) + '%';
function bkArea(el, dates, vals, { color, fmt, axFmt = fmt, h = 250, markMin = false }) {
  const W = el.clientWidth || 600, P = { l: 8, r: 52, t: 10, b: 24 }, n = vals.length, step = Math.max(1, Math.floor(n / 500));
  const idx = []; for (let i = 0; i < n; i += step) idx.push(i); if (idx[idx.length - 1] !== n - 1) idx.push(n - 1);
  const lo = Math.min(0, ...vals), hi = Math.max(0, ...vals), X = i => P.l + i / (n - 1) * (W - P.l - P.r), Y = v => P.t + (hi - v) / ((hi - lo) || 1) * (h - P.t - P.b);
  const line = idx.map((i, k) => `${k ? 'L' : 'M'}${X(i).toFixed(1)},${Y(vals[i]).toFixed(1)}`).join(''), gid = 'g' + Math.random().toString(36).slice(2, 8);
  const ticks = [0, .25, .5, .75, 1].map(f => lo + (hi - lo) * f), years = [...new Set(dates.map(x => x.slice(0, 4)))], ys = years.filter((_, k) => k % Math.ceil(years.length / 7) === 0);
  el.innerHTML = `<svg viewBox="0 0 ${W} ${h}" width="100%" height="${h}"><defs><linearGradient id="${gid}" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="${color}" stop-opacity=".24"/><stop offset=".5" stop-color="${color}" stop-opacity=".52"/><stop offset="1" stop-color="${color}" stop-opacity=".88"/></linearGradient>
      <linearGradient id="${gid}f" x1="0" x2="1"><stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset=".1" stop-color="#fff" stop-opacity="1"/><stop offset=".9" stop-color="#fff" stop-opacity="1"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient><mask id="${gid}m"><rect width="${W}" height="${h}" fill="url(#${gid}f)"/></mask></defs>
    <g mask="url(#${gid}m)">${ticks.map(v => `<line x1="${P.l}" x2="${W - P.r}" y1="${Y(v)}" y2="${Y(v)}" class="bk-grid"/>`).join('')}</g>
    ${ticks.map(v => `<text x="${W - P.r + 6}" y="${Y(v) + 4}" class="bk-ax">${axFmt(v)}</text>`).join('')}${ys.map(y => { const i = dates.findIndex(x => x.startsWith(y)); return `<text x="${X(i)}" y="${h - 6}" class="bk-ax" text-anchor="middle">${y}</text>`; }).join('')}
    <path d="${line}L${X(n - 1)},${Y(0)}L${X(0)},${Y(0)}Z" fill="url(#${gid})" class="bk-fill"/><path d="${line}" fill="none" stroke="${color}" stroke-width="2" class="bk-line" style="filter:drop-shadow(0 2px 4px ${color}55)"/>
    ${markMin ? (() => { const i = vals.indexOf(Math.min(...vals)), lx = X(i), ly = Y(vals[i]), right = lx < W * .6; return `<line x1="${P.l}" x2="${W - P.r}" y1="${ly}" y2="${ly}" class="bk-min"/><circle cx="${lx}" cy="${ly}" r="5" fill="${color}" class="bk-mindot"/><g class="bk-minlab"><rect x="${right ? lx + 10 : lx - 178}" y="${ly - 30}" width="168" height="22" rx="6"/><text x="${right ? lx + 18 : lx - 170}" y="${ly - 15}">Worst ${fmt(vals[i])} · ${dates[i]}</text></g>`; })() : ''}
    <line class="bk-cross" y1="${P.t}" y2="${h - P.b}" x1="-10" x2="-10" style="opacity:0"/><circle class="bk-dot" r="4.5" cx="-10" cy="-10" fill="${color}" style="opacity:0"/></svg>`;
  const svg = el.querySelector('svg'), cross = svg.querySelector('.bk-cross'), dot = svg.querySelector('.bk-dot'), t = tip(el);
  svg.addEventListener('mousemove', e => { const r = svg.getBoundingClientRect(), x = (e.clientX - r.left) / r.width * W, i = Math.max(0, Math.min(n - 1, Math.round((x - P.l) / (W - P.l - P.r) * (n - 1))));
    cross.setAttribute('x1', X(i)); cross.setAttribute('x2', X(i)); dot.setAttribute('cx', X(i)); dot.setAttribute('cy', Y(vals[i])); dot.style.opacity = 1; cross.style.opacity = 1;
    t.innerHTML = `<span>${dates[i]}</span><b>${fmt(vals[i])}</b>`; t.style.opacity = 1; t.style.left = Math.min(r.width - 120, X(i) / W * r.width + 12) + 'px'; t.style.top = (Y(vals[i]) / h * r.height - 10) + 'px'; });
  svg.addEventListener('mouseleave', () => { t.style.opacity = 0; cross.setAttribute('x1', -10); cross.setAttribute('x2', -10); dot.setAttribute('cx', -10); dot.style.opacity = 0; cross.style.opacity = 0; });
  const path = svg.querySelector('.bk-line'), len = path.getTotalLength();
  path.style.strokeDasharray = len; path.style.strokeDashoffset = CALM || !MO ? 0 : len;
  anim(path, { strokeDashoffset: [len, 0] }, { duration: 1.1, ease: [.22, 1, .36, 1] }); anim(svg.querySelector('.bk-fill'), { opacity: [0, 1] }, { duration: .8, delay: .3 });
}
function bkBars(el, items, { h = 220 }) {
  const W = el.clientWidth || 600, P = { l: 8, r: 46, t: 10, b: 24 }, n = items.length, all = items.flatMap(x => [x.a, x.b]);
  const lo = Math.min(0, ...all), hi = Math.max(0, ...all), Y = v => P.t + (hi - v) / ((hi - lo) || 1) * (h - P.t - P.b), slot = (W - P.l - P.r) / n, bw = Math.max(2, slot / 2 - 2);
  const ticks = [lo, lo / 2, 0, hi / 2, hi].filter((v, i, a) => a.indexOf(v) === i);
  el.innerHTML = `<svg viewBox="0 0 ${W} ${h}" width="100%" height="${h}">${ticks.map(v => `<line x1="${P.l}" x2="${W - P.r}" y1="${Y(v)}" y2="${Y(v)}" class="bk-grid ${v === 0 ? 'zero' : ''}"/><text x="${W - P.r + 6}" y="${Y(v) + 4}" class="bk-ax">${pctAx(v)}</text>`).join('')}
    ${items.map((x, i) => { const cx = P.l + i * slot; return `<g class="bk-col" data-i="${i}"><rect class="bk-hit" x="${cx}" y="${P.t}" width="${slot}" height="${h - P.t - P.b}"/>
      <rect class="bk-bar" x="${cx + 1}" y="${Math.min(Y(x.a), Y(0))}" width="${bw}" height="${Math.max(.8, Math.abs(Y(x.a) - Y(0)))}" fill="${x.a >= 0 ? 'var(--chart)' : 'var(--red)'}" style="transform-origin:${cx + 1}px ${Y(0)}px"/>
      <rect class="bk-bar" x="${cx + 1 + bw + 1}" y="${Math.min(Y(x.b), Y(0))}" width="${bw}" height="${Math.max(.8, Math.abs(Y(x.b) - Y(0)))}" fill="var(--bh)" style="transform-origin:${cx + 2 + bw}px ${Y(0)}px"/>
      ${i % Math.ceil(n / 9) === 0 ? `<text x="${cx + slot / 2}" y="${h - 6}" class="bk-ax" text-anchor="middle">${x.label}</text>` : ''}</g>`; }).join('')}</svg>`;
  const svg = el.querySelector('svg'), t = tip(el);
  svg.querySelectorAll('.bk-col').forEach(g => { g.addEventListener('mouseenter', () => { const x = items[+g.dataset.i], r = svg.getBoundingClientRect(), bx = g.querySelector('.bk-hit'); svg.classList.add('hov'); g.classList.add('on');
      t.innerHTML = `<span>${x.label}</span><b class="${cls(x.a)}">Strategy ${pct(x.a, 2, true)}</b><b>Buy &amp; hold ${pct(x.b, 2, true)}</b>`; t.style.opacity = 1; t.style.left = Math.min(r.width - 160, (+bx.getAttribute('x') + slot) / W * r.width + 6) + 'px'; t.style.top = '8px'; });
    g.addEventListener('mouseleave', () => { svg.classList.remove('hov'); g.classList.remove('on'); t.style.opacity = 0; }); });
  anim(svg.querySelectorAll('.bk-bar'), { transform: ['scaleY(0)', 'scaleY(1)'] }, { delay: MO?.stagger ? MO.stagger(.012) : 0, type: 'spring', stiffness: 260, damping: 22 });
}
let PXCH = null, HEAT_DESC = false, HEAT_PALETTE='classic';
function heatTable(d) {
  const report=VaultReport.monthly(d),m=report.months,ys=report.annual,years=[...report.years]; if (HEAT_DESC) years.reverse();
  const MON = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'], key = (y, k) => `${y}-${String(k + 1).padStart(2, '0')}`;
  const cap = Math.max(.01, ...Object.values(m).map(Math.abs));
  const flat = v => v == null; // a month with no position open shows blank, not 0.00
  const shades = HEAT_PALETTE==='ocean'?['#1689bb','#d36d28']:HEAT_PALETTE==='violet'?['#078f84','#b746a2']:HEAT_PALETTE==='mono'?[cv('--blue'),cv('--blue')]:[ '#19855d','#cf5550'];
  const intensity = (v, c = cap) => v === 0 ? .1 : Math.round((Math.min(1, Math.abs(v) / c) * .56 + .16) * 100) / 100; // never paler than 30%; exact zero stays a light neutral
  const bg = (v, c = cap) => flat(v) ? 'color-mix(in srgb, var(--ink3) 6%, var(--card))' : v === 0 ? 'color-mix(in srgb, var(--ink3) 12%, var(--card))' : `color-mix(in srgb, ${shades[v>=0?0:1]} ${intensity(v, c)*100}%, var(--card))`;
  const rgb = hex => hex.replace('#','').match(/../g).map(x=>parseInt(x,16));
  const luminance = channels => channels.map(x=>{const v=x/255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
  const ink = (v, c = cap) => {if(flat(v)||v===0)return 'var(--ink)';const alpha=intensity(v, c),card=rgb(cv('--card')),color=rgb(shades[v>=0?0:1]),lum=luminance(color.map((c,i)=>c*alpha+card[i]*(1-alpha)));return (lum+.05)/(luminance(rgb('#10232b'))+.05)>1.05/(lum+.05)?'#10232b':'#fff';};
  const avg = MON.map((_, k) => { const a = years.map(y => m[key(y, k)]).filter(v => v != null); return a.length ? a.reduce((x, y) => x + y, 0) / a.length : null; });
  const capY = Math.max(.01, ...Object.values(ys).filter(v => v != null).map(Math.abs)), capA = Math.max(.002, ...avg.filter(v => v != null).map(Math.abs)); // own scales: yearly and average values are larger/smaller than months
  const cell = v => `<td style="background:${bg(v)};color:${ink(v)}"${flat(v) ? ' title="Outside available period"' : ` title="${pct(v, 2, true)}"`}>${flat(v) ? '' : (v * 100).toFixed(2)}</td>`;
  return `<table class="heat2 heatmap-refined"><thead><tr><th class="y">Year</th>${MON.map(x => `<th>${x}</th>`).join('')}<th class="t">Total</th></tr></thead>
    <tbody>${years.map(y => `<tr><th class="y">${y}</th>${MON.map((_, k) => cell(m[key(y, k)])).join('')}<td class="t" style="background:${bg(ys[y], capY)};color:${ink(ys[y], capY)}">${flat(ys[y]) ? '' : (ys[y] * 100).toFixed(2)}</td></tr>`).join('')}</tbody>
    <tfoot><tr><th class="y">Avg</th>${avg.map(v => `<td style="background:${bg(v, cap)};color:${ink(v, cap)}">${v == null ? '' : (v * 100).toFixed(2)}</td>`).join('')}<td class="t" style="background:${bg(report.annualAverage, capY)};color:${ink(report.annualAverage, capY)}" data-annual-average>${report.annualAverage===null?'':(report.annualAverage*100).toFixed(2)}</td></tr></tfoot></table>`;
}
// 5-minute view of an intraday strategy (BTST): every bar of the selected period in one zoomable chart (engine route
// guru/yt/intraday?compact=1), with an In/Out arrow and a shaded span for every trade, like the Daily chart.
let INTRA = { key: '', data: null };
async function drawIntraday(el) {
  const snapshot = STRAT, request = ++SIGNAL_REQUEST, { d } = snapshot, trades = d.trades || [];
  INDICATOR_CHARTS?.dispose(); INDICATOR_CHARTS = null;
  if (PXCH) { try { PXCH.__ro?.disconnect(); PXCH.remove(); } catch (e) {} CHARTS = CHARTS.filter(c => c.ch !== PXCH); PXCH = null; } el.innerHTML = '';
  document.querySelectorAll('[data-tf]').forEach(b => b.classList.toggle('active', b.dataset.tf === 'I'));
  const read = $('#pxread'), still = () => snapshot === STRAT && TF === 'I' && request === SIGNAL_REQUEST && el.isConnected;
  const old = document.getElementById('intra-bar'); if (old) old.remove();
  const bar = document.createElement('div'); bar.id = 'intra-bar'; bar.className = 'chart-tools intra-bar';
  bar.innerHTML = `<span class="ib-label">Trade</span><button type="button" class="ib-step" id="intra-prev" aria-label="Previous trade" title="Previous trade">‹</button><select id="intra-sel" aria-label="Jump to trade"><option value="">Jump to Trade/Overall</option>${trades.map((t, k) => `<option value="${k}">${esc(t.entry)} · ${t.open ? 'open' : esc(t.reason || '')} · ${t.open ? '' : pct(t.ret, 2, true)}</option>`).join('')}</select><button type="button" class="ib-step" id="intra-next" aria-label="Next trade" title="Next trade">›</button><span class="ib-note">5-minute bars of Nifty front-month futures (the strategy’s own data); 1-minute bars are not loaded in the engine</span>`;
  read.parentNode.insertBefore(bar, read); // above the hover readout, so the chart follows it directly
  document.querySelectorAll('#px3,#px1,#pxall,#px3m').forEach(b => { b.parentElement.style.display = ''; }); $('#px3m').style.display = '';
  try { await optSchema(); } catch (e) { /* defaults fall back to the strategy's own */ }
  const params = { ...bookDefaults(snapshot.v.id), ...decodeP(S.p) };
  const from = d.dates[0], to = d.dates[d.dates.length - 1], key = `${snapshot.tk}|${from}|${to}|${params.src || 1}`;
  read.textContent = 'Loading 5-minute bars…';
  let data = INTRA.key === key ? INTRA.data : null;
  if (!data) {
    try {
      const r = await fetch(`${API}/guru/yt/intraday?` + new URLSearchParams({ ticker: snapshot.tk, from, to, compact: '1', src: String(params.src || 1) }));
      if (!r.ok) throw Error((await r.text()) || r.statusText); data = await r.json(); INTRA = { key, data };
    } catch (e) { if (still()) { read.textContent = ''; el.innerHTML = `<div class="empty">5-minute bars are not available: ${esc(e.message || e)}</div>`; } return; }
  }
  if (!still() || !window.LightweightCharts) return;
  const n = data.n;
  if (!n) { read.textContent = ''; el.innerHTML = '<div class="empty">No 5-minute bars in this period.</div>'; return; }
  const L = LightweightCharts, ch = L.createChart(el, { width: el.clientWidth, height: el.clientHeight, layout: { attributionLogo: false, background: { color: cv('--panel') }, textColor: cv('--ink3'), fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11 },
    grid: { vertLines: { visible: false }, horzLines: { color: cv('--rule') } }, rightPriceScale: { borderColor: cv('--steel') }, timeScale: { borderColor: cv('--steel'), timeVisible: true, secondsVisible: false, minBarSpacing: 0.002, lockVisibleTimeRangeOnResize: true }, crosshair: { mode: L.CrosshairMode.Normal }, handleScale: { axisPressedMouseMove: false } });
  PXCH = ch; CHARTS.push({ ch, el });
  const cs = ch.addCandlestickSeries({ upColor: cv('--green'), downColor: cv('--red'), borderUpColor: cv('--green'), borderDownColor: cv('--red'), wickUpColor: cv('--green'), wickDownColor: cv('--red'), priceLineVisible: false });
  const t0 = performance.now(), rows = new Array(n); for (let i = 0; i < n; i++) rows[i] = { time: data.t[i], open: data.o[i], high: data.h[i], low: data.l[i], close: data.c[i] };
  cs.setData(rows); window.__intraMs = Math.round(performance.now() - t0);
  const layers = tradeLayers(trades, data.t, params), spans = tradeSpans(layers, data.t[n - 1]);
  cs.setMarkers(layerMarkers(layers, { buy: cv('--green'), sell: cv('--red') }));
  el.style.position = 'relative';
  const ov = document.createElement('canvas'); ov.className = 'zone-ov'; el.appendChild(ov);
  const col = t => t.open ? cv('--ink3') : t.ret >= 0 ? cv('--green') : cv('--red');
  const draw = () => {
    if (PXCH !== ch || !el.isConnected || !el.clientWidth || !el.clientHeight) return;
    const ts = ch.timeScale(), w = el.clientWidth, h = el.clientHeight, dpr = devicePixelRatio || 1, plotW = w - ch.priceScale('right').width(), plotH = h - ts.height();
    ov.width = w * dpr; ov.height = h * dpr; ov.style.width = w + 'px'; ov.style.height = h + 'px';
    const g = ov.getContext('2d'); g.scale(dpr, dpr); g.clearRect(0, 0, w, h); g.save(); g.beginPath(); g.rect(0, 0, plotW, plotH); g.clip();
    const half = Math.max(1, ts.options().barSpacing / 2), lw = ts.options().barSpacing * 75; // one session is 75 bars wide
    let lastLabel = -1e9; g.font = `700 11px ${getComputedStyle(document.body).fontFamily}`; g.textAlign = 'center';
    const seen = spans.map(z => ({ z, x0: ts.timeToCoordinate(z.a), x1: ts.timeToCoordinate(z.b) })).filter(o => o.x0 != null && o.x1 != null && o.x1 + half >= 0 && o.x0 - half <= plotW);
    for (const { z, x0, x1 } of seen) { const t = trades[z.index];
      g.fillStyle = rgba_(col(t), .16); g.fillRect(x0 - half, 0, Math.max(2, x1 - x0 + 2 * half), plotH);
      g.fillStyle = rgba_(col(t), .75); g.fillRect(x0 - half, 0, Math.max(2, x1 - x0 + 2 * half), 3); }
    if (lw > 150) for (const { z, x0, x1 } of seen) { const l = layers[z.index]; // profit-booking / hard-stop level, only once a session is wide enough to read
      for (const [lvl, c] of [[l.profitLine, cv('--green')], [l.stopLine, cv('--red')]]) { if (lvl == null) continue; const y = cs.priceToCoordinate(lvl); if (y == null) continue;
        g.strokeStyle = c; g.setLineDash([4, 3]); g.beginPath(); g.moveTo(x0, y); g.lineTo(x1, y); g.stroke(); g.setLineDash([]); } }
    for (const { z, x0, x1 } of seen) { const cx = (x0 + x1) / 2; if (cx < 0 || cx > plotW) continue; const t = trades[z.index];
      const txt = t.open ? 'open' : pct(t.ret, 2, true), tw = g.measureText(txt).width + 10;
      if (cx - tw / 2 < lastLabel + 4) continue; lastLabel = cx + tw / 2;
      g.fillStyle = cv('--panel'); g.fillRect(cx - tw / 2, 6, tw, 16); g.fillStyle = col(t); g.fillText(txt, cx, 18); }
    g.restore();
  };
  const relabel = () => requestAnimationFrame(() => { try { draw(); } catch (e) { /* chart torn down mid-frame */ } });
  const ro = new ResizeObserver(relabel); ro.observe(el); ch.__ro = ro;
  ch.timeScale().subscribeVisibleTimeRangeChange(relabel);
  const last = data.t[n - 1], DAY = 86400, showRange = (a, b) => { ch.timeScale().setVisibleRange({ from: a, to: b }); relabel(); };
  const presets = { px3m: 92 * DAY, px3: 3 * 365 * DAY, px1: 365 * DAY, pxall: 0 };
  const mark = id => document.querySelectorAll('#px3,#px1,#pxall,#px3m').forEach(x => x.classList.toggle('active', x.id === id));
  Object.entries(presets).forEach(([id, span]) => { $('#' + id).onclick = () => { $('#intra-sel').value = ''; if (span) showRange(Math.max(data.t[0], last - span), last); else { ch.timeScale().fitContent(); relabel(); } mark(id); }; });
  const sel = $('#intra-sel');
  sel.onchange = () => { if (sel.value === '') { ch.timeScale().fitContent(); relabel(); mark('pxall'); return; } const w = tradeWindow(trades[+sel.value]); showRange(w.from, w.to); mark(''); };
  const step = by => { if (!trades.length) return; const at = sel.value === '' ? (by > 0 ? -1 : trades.length) : +sel.value, to = Math.min(trades.length - 1, at + by); sel.value = to < 0 ? '' : String(to); sel.onchange(); };
  $('#intra-prev').onclick = () => step(-1); $('#intra-next').onclick = () => step(1);
  ch.timeScale().fitContent(); mark('pxall'); relabel();
  const f = x => x.toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  read.innerHTML = `5-minute candles · ${n.toLocaleString('en-IN')} bars · ${trades.length} trades · hover for open, high, low, close and the trade you were in.`;
  ch.subscribeCrosshairMove(p => { if (!p.time) return; const i = lowerBound(data.t, p.time); if (data.t[i] !== p.time) return;
    const sp = spans.find(z => z.a <= p.time && p.time <= z.b), t = sp ? trades[sp.index] : null, stamp = new Date(p.time * 1000).toISOString().slice(0, 16).replace('T', ' ');
    read.innerHTML = `<b>${stamp}</b> O ${f(data.o[i])} · H ${f(data.h[i])} · L ${f(data.l[i])} · C <b>${f(data.c[i])}</b> · ${t ? `in trade ${esc(t.entry)} to ${t.open ? 'open' : esc(t.exit)}: <b class="${cls(t.ret)}">${t.open ? 'open' : pct(t.ret, 2, true)}</b>${t.reason && !t.open ? ' (' + esc(t.reason) + ')' : ''}` : '<span class="muted">no position</span>'}`; });
}
async function drawSignals(tf) {
  const el = $('#c-px'); if (!el || !STRAT) return; const { d } = STRAT;
  if (tf === 'I' && !(isYt(STRAT.v) && isIntradayStrategy(STRAT.v.needs_series))) tf = TF = 'D';
  if (tf === 'I') return drawIntraday(el);
  { const old = document.getElementById('intra-bar'); if (old) old.remove(); const m3 = $('#px3m'); if (m3) { m3.style.display = 'none'; m3.classList.remove('active'); } }
  const generation=busy,snapshot=STRAT,request=++SIGNAL_REQUEST;
  const px = tf === 'D' ? STRAT.px : STRAT.px?.full_history||STRAT.d.ohlc?VaultReport.aggregate(STRAT.px,tf):await ohlc(STRAT.tk, tf);
  if(generation!==busy||snapshot!==STRAT||tf!==TF||request!==SIGNAL_REQUEST||!reportVisible(el))return;
  INDICATOR_CHARTS?.dispose();INDICATOR_CHARTS=null;
  if (PXCH) { try { PXCH.__ro?.disconnect(); PXCH.remove(); } catch (e) {} CHARTS = CHARTS.filter(c => c.ch !== PXCH); PXCH = null; } el.innerHTML = '';
  if (!px || !window.LightweightCharts) { el.innerHTML = '<div class="empty">Price data not available for this index.</div>'; return; }
  document.querySelectorAll('[data-tf]').forEach(b => b.classList.toggle('active', b.dataset.tf === tf));
  const L = LightweightCharts, ch = L.createChart(el, { width: el.clientWidth, height: el.clientHeight, layout: { attributionLogo: false, background: { color: cv('--panel') }, textColor: cv('--ink3'), fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11 },
    grid: { vertLines: { visible: false }, horzLines: { color: cv('--rule') } }, rightPriceScale: { borderColor: cv('--steel') }, timeScale: { borderColor: cv('--steel'), minBarSpacing: 0.05, lockVisibleTimeRangeOnResize: true }, crosshair: { mode: L.CrosshairMode.Normal }, handleScale: { axisPressedMouseMove: false } });
  PXCH = ch; CHARTS.push({ ch, el });
  const cs = ch.addCandlestickSeries({ upColor: cv('--green'), downColor: cv('--red'), borderUpColor: cv('--green'), borderDownColor: cv('--red'), wickUpColor: cv('--green'), wickDownColor: cv('--red'), priceLineVisible: false });
  cs.setData(px.d.map((t, i) => ({ time: t, open: px.o[i], high: px.h[i], low: px.l[i], close: px.c[i] })));
  // daily index -> candle index (a weekly/monthly candle is dated on the last trading day of its period)
  const barOf = new Array(d.dates.length); for (let i = 0, b = 0; i < d.dates.length; i++) { while (b < px.d.length - 1 && px.d[b] < d.dates[i]) b++; barOf[i] = b; }
  const n = px.d.length, perBar = {};
  // trade zones: a solid translucent band from the entry candle to the exit candle (green = win, red = loss, grey = still open),
  // drawn on an overlay so it stays one block at any zoom; one return label per trade, skipped where labels would collide
  const unit=unitFor({...snapshot,state:S,params:snapshot.indicatorParameters}),unitSignals=unit?unitPriceSignals(d,unit,barOf):null;
  const zones = unitSignals?unitSignals.zones:d.trades.map(t => ({ a: barOf[t.legs[0][0]], b: t.open ? n - 1 : barOf[t.exit_i], t }));
  d.trades.forEach(t => { const a = barOf[t.legs[0][0]]; (perBar[a] ||= { in: [], out: [] }).in.push(t); if (!t.open) (perBar[barOf[t.exit_i]] ||= { in: [], out: [] }).out.push(t); });
  // Arrow color denotes the action. Return colors remain in the independent trade shading.
  const arrows = TRUST.actionMarkers(d.trades,px.d,barOf,{buy:cv('--green'),sell:cv('--red')});
  arrows.sort((x, y) => (x.time < y.time ? -1 : x.time > y.time ? 1 : 0)); cs.setMarkers(arrows);
  el.style.position = 'relative';
  const ov = document.createElement('canvas'); ov.className = 'zone-ov'; el.appendChild(ov);
  const draw = () => {
    if (PXCH !== ch || !el.isConnected || !el.clientWidth || !el.clientHeight) return; // hidden tabs retain their last valid chart geometry
    const ts = ch.timeScale(), w = el.clientWidth, h = el.clientHeight, dpr = devicePixelRatio || 1, plotW = w - ch.priceScale('right').width(), plotH = h - ts.height();
    ov.width = w * dpr; ov.height = h * dpr; ov.style.width = w + 'px'; ov.style.height = h + 'px';
    const g = ov.getContext('2d'); g.scale(dpr, dpr); g.clearRect(0, 0, w, h); g.save(); g.beginPath(); g.rect(0, 0, plotW, plotH); g.clip();
    const half = Math.max(1, ts.options().barSpacing / 2), x = b => ts.logicalToCoordinate(b);
    let lastLabel = -1e9; g.font = `700 11px ${getComputedStyle(document.body).fontFamily}`; g.textAlign = 'center';
    const col = t => unitSignals?({open:cv('--ink3'),win:cv('--green'),nonwin:cv('--red')}[unitSignals.annotation(t).outcome]):t.open ? cv('--ink3') : t.ret >= 0 ? cv('--green') : cv('--red');
    for (const z of zones) { const x0 = x(z.a), x1 = x(z.b); if (x0 == null || x1 == null || x1 + half < 0 || x0 - half > plotW) continue;
      g.fillStyle = rgba_(col(z.t), .16); g.fillRect(x0 - half, 0, Math.max(2, x1 - x0 + 2 * half), plotH);
      g.fillStyle = rgba_(col(z.t), .75); g.fillRect(x0 - half, 0, Math.max(2, x1 - x0 + 2 * half), 3); }
    for (const z of zones) { const x0 = x(z.a), x1 = x(z.b); if (x0 == null || x1 == null) continue; const cx = (x0 + x1) / 2; if (cx < 0 || cx > plotW) continue;
      const txt = z.t.open ? 'open' : pct(unitSignals?unitSignals.annotation(z.t).return:z.t.ret, 2, true), tw = g.measureText(txt).width + 10;
      if (cx - tw / 2 < lastLabel + 4) continue; lastLabel = cx + tw / 2;
      g.fillStyle = cv('--panel'); g.fillRect(cx - tw / 2, 6, tw, 16); g.fillStyle = col(z.t); g.fillText(txt, cx, 18); }
    g.restore();
  };
  const relabel = () => requestAnimationFrame(() => { try { draw(); } catch (e) { /* chart torn down mid-frame */ } });
  const ro = new ResizeObserver(relabel); ro.observe(el); ch.__ro = ro;
  ch.timeScale().subscribeVisibleLogicalRangeChange(relabel);
  const selectedKestner = snapshot.indicators?.status==='ready'&&d.kestner_research&&isKestnerVisualMethod(snapshot.v.id);
  const span = { D: [756, 252], W: [156, 52], M: [36, 12] }[tf], show = bars => {
    if(selectedKestner){const range=kestnerReportLogicalRange(px.d,d.dates,bars);if(range)ch.timeScale().setVisibleLogicalRange(range);return;}
    if (bars) ch.timeScale().setVisibleLogicalRange({ from: Math.max(0, n - bars), to: n - 1 }); else ch.timeScale().fitContent();
  };
  const btns = { px3: span[0], px1: span[1], pxall: 0 };
  Object.entries(btns).forEach(([id, bars]) => { $('#' + id).onclick = () => { show(bars); relabel(); document.querySelectorAll('#px3,#px1,#pxall').forEach(x => x.classList.toggle('active', x.id === id)); }; });
  const cur = document.querySelector('#px3.active,#px1.active,#pxall.active')?.id || 'px3'; show(btns[cur]); relabel();
  if (S.from || S.to) { ch.timeScale().setVisibleLogicalRange({ from: Math.max(0, barOf[0] - 3), to: Math.min(n - 1, barOf[d.dates.length - 1] + 3) }); document.querySelectorAll('#px3,#px1,#pxall').forEach(x => x.classList.remove('active')); relabel(); }
  const at = {}; px.d.forEach((t, i) => at[t] = i); const f = x => x.toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const what = { D: '', W: 'Week to ', M: 'Month to ' }[tf];
  $('#pxread').innerHTML = `${{ D: 'Daily', W: 'Weekly', M: 'Monthly' }[tf]} candles · ${n} bars · hover for open, high, low, close and the trades in each candle.`;
  ch.subscribeCrosshairMove(p => { const b = p.time ? at[p.time] : undefined; if (b === undefined) return; const chg = b ? px.c[b] / px.c[b - 1] - 1 : 0;
    let tradeTxt;
    if (tf === 'D') { const tr = unitSignals?unitSignals.positionAt(b):d.trades.find(t => barOf[t.legs[0][0]] <= b && b <= (t.open ? n - 1 : barOf[t.exit_i])); tradeTxt = tr ? `in trade ${tr.entry} to ${tr.open ? unitSignals?'open mark '+tr.exit:'open' : tr.exit}: <b class="${unitSignals?unitSignals.annotation(tr).outcome==='win'?'pos':unitSignals.annotation(tr).outcome==='open'?'':'neg':cls(tr.ret)}">${pct(unitSignals?unitSignals.annotation(tr).return:tr.ret, 2, true)}</b>${tr.legs.length > 1 ? ` (${tr.legs.length} pieces)` : ''}` : '<span class="muted">no position</span>'; }
    else { const g = perBar[b],closedSum=unitSignals&&g?.out.length?unitSignals.closedReturnSum(g.out):null;
      const returns=g?.out.length?unitSignals?`: Closed return sum <b class="${closedSum>0?'pos':'neg'}">${pct(closedSum,2,true)}</b>`:`: <b class="${cls(g.out.reduce((a, t) => a * (1 + t.ret), 1) - 1)}">${pct(g.out.reduce((a, t) => a * (1 + t.ret), 1) - 1, 2, true)}</b> combined`:'';
      tradeTxt = g ? `${g.in.length} entr${g.in.length === 1 ? 'y' : 'ies'} · ${g.out.length} exit${g.out.length === 1 ? '' : 's'}${returns}` : '<span class="muted">no trades in this candle</span>'; }
    $('#pxread').innerHTML = `<b>${what}${px.d[b]}</b> O ${f(px.o[b])} · H ${f(px.h[b])} · L ${f(px.l[b])} · C <b>${f(px.c[b])}</b> <span class="${cls(chg)}">${pct(chg, 2, true)}</span> · ${tradeTxt}`; });
  const model=buildStrategyIndicators({id:STRAT.v.id,book:STRAT.v.book,ticker:STRAT.tk,prices:STRAT.px,fullHistory:STRAT.px?.full_history===true,parameters:STRAT.indicatorParameters,displayDates:px.d,sourceReport:STRAT.d});
  INDICATOR_CHARTS=mountStrategyIndicators({root:document.querySelector('[data-strategy-indicators]'),chart:ch,priceSeries:cs,result:model,library:L,pricePoints:px.d.map((time,i)=>({time,value:px.c[i]})),chartOptions:{layout:{attributionLogo:false,background:{color:cv('--panel')},textColor:cv('--ink3'),fontFamily:getComputedStyle(document.body).fontFamily,fontSize:11},grid:{vertLines:{visible:false},horzLines:{color:cv('--rule')}},timeScale:{borderColor:cv('--steel'),minBarSpacing:ch.timeScale().options().minBarSpacing,lockVisibleTimeRangeOnResize:ch.timeScale().options().lockVisibleTimeRangeOnResize},crosshair:{mode:L.CrosshairMode.Normal}}});
}
const safe = (name, f) => { try { f(); } catch (e) { console.warn(name, e); } };
function drawStrategy() {
  if (!STRAT||STRAT.pristine) return; if (R.optim) {safe('optim',STRAT.ath?athOptInit:optimInit);return;} const { d, tiles } = STRAT;
  safe('tiles', () => {
  // count-up tiles
  finishMetricAnimations(); // Financial values are shown atomically, without intermediate mixed counts/returns.
  });
  if(d.kind!=='basket'&&d.kind!=='universe')safe('optim', optimInit);
  const report=document.querySelector('[data-strategy-report]');
  if(report&&typeof ResizeObserver==='function'){
    REPORT_OBSERVER?.disconnect();REPORT_OBSERVER=new ResizeObserver(()=>resizeVisibleReportCharts());REPORT_OBSERVER.observe(report);
  }
  void drawVisibleReportCharts();
  if($('#heat-palette')){$('#heat-palette').value=HEAT_PALETTE;$('#heat-palette').onchange=e=>{HEAT_PALETTE=e.target.value;$('#heatbox').innerHTML=heatTable(d);};}
}
const reportVisible=el=>!!(el?.isConnected&&el.clientWidth&&el.clientHeight);
function resizeVisibleReportCharts(){
  CHARTS.forEach(({ch,el})=>{if(reportVisible(el))ch.resize(el.clientWidth,el.clientHeight);});
  if(STRAT&&!R.optim)void drawVisibleReportCharts();
}
async function drawVisibleReportCharts(){
  if(!STRAT||STRAT.pristine||R.optim)return;const {d}=STRAT;
  // Each widget is initialized only after its existing container is visible.
  // Keeping it mounted preserves range, series toggles and indicator choices.
  safe('equity', () => { const el = $('#c-eq'); if (el && window.LightweightCharts) {
    if(!reportVisible(el)||CHARTS.some(item=>item.el===el))return;
    const L = LightweightCharts, fmt = { type: 'custom', formatter: v => v.toFixed(2) + '×', minMove: 0.01 };
    const ch = L.createChart(el, { width: el.clientWidth, height: el.clientHeight, layout: { attributionLogo: false, background: { color: cv('--panel') }, textColor: cv('--ink3'), fontFamily: getComputedStyle(document.body).fontFamily, fontSize: 11 },
      grid: { vertLines: { visible: false }, horzLines: { color: cv('--rule') } }, rightPriceScale: { borderColor: cv('--steel'), mode: L.PriceScaleMode.Logarithmic }, timeScale: { borderColor: cv('--steel'), minBarSpacing: 0.01 }, crosshair: { mode: L.CrosshairMode.Normal }, handleScale: { axisPressedMouseMove: false } });
    const bh = ch.addLineSeries({ color: cv('--bh'), lineWidth: 2, priceFormat: fmt, lastValueVisible: false, priceLineVisible: false, visible: false });
    const st = ch.addAreaSeries({ lineColor: cv('--chart'), topColor: rgba_(cv('--chart'), .30), bottomColor: rgba_(cv('--chart'), .02), lineWidth: 2, priceFormat: fmt, priceLineVisible: false });
    bh.setData(d.dates.map((t, i) => ({ time: t, value: d.bh[i] }))); st.setData(d.dates.map((t, i) => ({ time: t, value: d.equity[i] }))); ch.timeScale().fitContent(); CHARTS.push({ ch, el });
    VaultUI.equityNavigator(ch,d);
    $('#cmp').onchange = e => bh.applyOptions({ visible: e.target.checked });
    $('#logs').onchange = e => ch.priceScale('right').applyOptions({ mode: e.target.checked ? L.PriceScaleMode.Logarithmic : L.PriceScaleMode.Normal });
  } });
  if(d.kind!=='basket'&&d.kind!=='universe'&&reportVisible($('#c-px'))&&!PXCH){
    const snapshot=STRAT,tf=TF;
    if(!SIGNAL_PENDING||SIGNAL_PENDING.snapshot!==snapshot||SIGNAL_PENDING.tf!==tf){
      const pending={snapshot,tf,promise:drawSignals(tf)};SIGNAL_PENDING=pending;
      pending.promise.finally(()=>{if(SIGNAL_PENDING===pending)SIGNAL_PENDING=null;}).catch(error=>console.warn('signals',error));
    }
    await SIGNAL_PENDING.promise;
    if(snapshot!==STRAT)return;
  }
  safe('drawdown', () => { const el=$('#c-dd');if(!reportVisible(el)||el.dataset.reportWidth===String(el.clientWidth))return;let pk = 1; const dd = d.equity.map(v => { pk = Math.max(pk, v); return v / pk - 1; }); bkArea(el, d.dates, dd, { color: cv('--red'), fmt: v => pct(v, 2), axFmt: pctAx, markMin: true });el.dataset.reportWidth=String(el.clientWidth); });
  safe('positions', () => { const el=$('#c-pos');if(!el||!d.positions?.length||!reportVisible(el)||el.dataset.reportWidth===String(el.clientWidth))return;bkArea(el, d.dates, d.positions, { color: cv('--blue'), fmt: v => fx(v, 0) + ' open', axFmt: v => fx(v, 0) });el.dataset.reportWidth=String(el.clientWidth); });
  safe('years', () => { const el=$('#c-yr');if(!reportVisible(el)||el.dataset.reportWidth===String(el.clientWidth))return;const ys = periods(d, 4), yb = periods({ dates: d.dates, equity: d.bh }, 4); bkBars(el, Object.keys(ys).map(k => ({ label: k, a: ys[k], b: yb[k] })), {});el.dataset.reportWidth=String(el.clientWidth); });
}
addEventListener('resize',resizeVisibleReportCharts);
addEventListener('beforeprint',()=>{document.body.classList.add('report-print');resizeVisibleReportCharts();});
addEventListener('afterprint',()=>{document.body.classList.remove('report-print');resizeVisibleReportCharts();});

// ---------------------------------------------------------------- render + motion
let busy = 0, lastName = '', LIST_HASH = ''; // the list (with its filters) the user last looked at
function viewSaved() {
  const list = VaultUI.savedList(), I = VaultUI.ICON;
  const rows = list.map(x => { const v = ALLCAT.find(c => c.id === x.id); return `<div class="saved-row">${I.saved}<div><a class="t" href="${esc(x.hash)}">${esc(x.name)}</a><small>${esc(v ? BOOK[v.book]?.title || '' : '')}${v ? ' · ' : ''}${esc(LABEL[x.tk] || x.tk)} · saved ${esc(x.at)}</small></div><button type="button" class="ibtn" data-unsave="${esc(x.key)}" title="Remove from Saved" aria-label="Remove from Saved">${I.save}</button></div>`; }).join('');
  return `<section class="box"><div class="box-h">Saved strategies <small>${list.length} saved · stored in this browser</small></div>${list.length ? `<div class="saved-list">${rows}</div>` : '<p class="rnote">Nothing saved yet. Open a strategy and press Save to keep it here, with its index and settings.</p>'}</section>`;
}
function savedCount() { const n = VaultUI.savedList().length, el = $('#saved-count'); if (el) { el.textContent = n; el.hidden = !n; } }
document.addEventListener('vault-saved', () => { savedCount(); if (R.name === 'saved') render(); });
document.addEventListener('click', e => { const b = e.target.closest('[data-unsave]'); if (b) VaultUI.writeSaved(VaultUI.savedList().filter(x => x.key !== b.dataset.unsave)); });
async function render() {
  const my=++busy,ticket=vaultStore.begin(R.path),route=resolveRoute(location.hash);
  if(R.name==='guru'||R.name==='all'||R.name==='yt')LIST_HASH=location.hash||'#/';
  STRAT=null;RULE_VISUAL=null;if(R.name!=='strategy')bookCtx(R.name==='yt'?'':S.book||'stc9');
  try {
    const custom=customPages.find(page=>page.path===route.path);
    let home=null,reading=null,html='';
    if(!custom){
      if(R.name==='home')home=await homeModel();
      else if(R.name==='source')reading=readingModel();
      else html=R.name==='saved'?viewSaved():R.name==='strategy'?await viewStrategy():await viewList();
    }
    if(my!==busy)return;
    if(home)HOME_STATS=home.authorStats;
    cleanupCharts();
    const focus=document.activeElement?.id,pos=document.activeElement?.selectionStart;
    pendingFocus={id:focus,pos};
    const status=custom?.label||(R.name==='strategy'?'Strategy':`${isUS()?'US':'India'} · ${S.tk==='ALL'?'all indices':(R.name==='yt'&&S.family==='ytbtst'&&btstFuturesSymbol(S.tk))||LABEL[S.tk]||S.tk} · ${TABS[S.tab]?.name||'Research'}`);
    vaultStore.commit(ticket,{html,home,reading,route,state:{...S},status,strategyRules:route.name==='strategy'?RULE_VISUAL:null,strategyAccount:route.name==='strategy'||route.name==='optimization'?STRAT?.sourceView??null:null,strategyPristine:route.name==='strategy'||route.name==='optimization'?STRAT?.pristine??null:null,strategyUnit:route.name==='strategy'||route.name==='optimization'?STRAT?.unit??null:null});
  }catch(error){if(my===busy)vaultStore.fail(ticket,error.message||'Research data is unavailable.');}
}
let pendingFocus=null;
function cleanupCharts(){
  SIGNAL_REQUEST++;SIGNAL_PENDING=null;
  REPORT_OBSERVER?.disconnect();REPORT_OBSERVER=null;
  INDICATOR_CHARTS?.dispose();INDICATOR_CHARTS=null;
  METRIC_ANIMATIONS.forEach(animation=>animation.stop?.());METRIC_ANIMATIONS=[];
  CHARTS.forEach(({ch})=>{try{ch.__ro?.disconnect();ch.remove();}catch{}});CHARTS=[];PXCH=null;
}
function commitReactPage(){
  VaultUI.init();
  const focus=pendingFocus;pendingFocus=null;
  if(focus?.id){const element=document.getElementById(focus.id);if(element){element.focus();try{element.setSelectionRange(focus.pos,focus.pos);}catch{}}}
  const key=R.name+(R.id||'')+(R.optim?'/optimization':'');if(key!==lastName)scrollTo(0,0);lastName=key;
  drawStrategy();
}
// ---------------------------------------------------------------- events
document.addEventListener('click', e => {
  const sum = e.target.closest('details.dd > summary');
  if (sum) { const me = sum.parentElement; document.querySelectorAll('details.dd[open]').forEach(d => d !== me && d.removeAttribute('open')); if (!me.open) setTimeout(() => anim(me.querySelector('.menu'), { opacity: [0, 1], transform: ['translateY(-6px) scale(.98)', 'translateY(0) scale(1)'] }, { type: 'spring', stiffness: 420, damping: 30 }), 0); return; }
  if (!e.target.closest('details.dd')) document.querySelectorAll('details.dd[open]').forEach(d => d.removeAttribute('open'));
  const lb=e.target.closest('[data-library-set]');if(lb){e.preventDefault();return setState(Object.fromEntries(new URLSearchParams(lb.dataset.librarySet)));}
  const hb = e.target.closest('[data-heat]'); if (hb && STRAT) { HEAT_DESC = hb.dataset.heat === 'desc'; $('#heatbox').innerHTML = heatTable(STRAT.d); document.querySelectorAll('[data-heat]').forEach(x => x.classList.toggle('active', x === hb)); return; }
  const positions=e.target.closest('[data-position-choice]');if(positions){const choice=positions.dataset.positionChoice;return setState({position_choice:choice,...(choice==='custom'?{}:{max_positions:choice})});}
  const membership=e.target.closest('[data-membership]');if(membership){const mode=membership.dataset.membership,base=S.basket.replace(/_past$/,'');return setState({membership:mode,basket:S.region==='us'&&base?base+(mode==='past'?'_past':''):S.basket,...(S.run_scope==='basket'?{from:'',to:''}:{})});}
  const basketButton=e.target.closest('[data-basket]');if(basketButton){if(basketButton.disabled)return;const key=basketButton.dataset.basket;return go('/s/'+encodeURIComponent(R.id),{...S,run_scope:'basket',basket:key,from:'',to:''});}
  const tfb = e.target.closest('[data-tf]'); if (tfb) { TF = tfb.dataset.tf; drawSignals(TF); return; }
  if(e.target.closest('.source-pdfs'))return;
  const pr = e.target.closest('[data-preset]'); if (pr && STRAT) { const x = (YT_PRESETS[STRAT.v.id] || [])[+pr.dataset.preset]; if (x) { e.preventDefault(); return go(R.path, { ...S, p: encodeP(x.params), ...(x.ticker ? { tk: x.ticker } : {}) }); } }
  const t = e.target.closest('[data-set], [data-fam], [data-book], [data-guru], [data-guru-strats], [data-sort]'); if (!t) return;
  const d = t.dataset;
  e.preventDefault();
  if (d.set != null) return setState(Object.fromEntries(new URLSearchParams(d.set)));
  if (d.fam) return setState({ family: d.fam, mode: 'variants' });
  if (d.sort) return setState({ sort: d.sort });
  if (d.book) return go('/guru/' + BOOK[d.book].guru.id, {view:'grid',book:d.book,tk:S.tk,basket:S.basket,from:S.from,to:S.to, ...(BOOK[d.book].guru.id==='ross'?{tab:'awaiting'}:{}) });
  if (d.guru) return go('/guru/'+d.guru,{view:'grid',tk:S.tk,basket:S.basket,from:S.from,to:S.to});
  if (d.guruStrats) return go('/guru/'+d.guruStrats,{view:'grid',tk:S.tk,basket:S.basket,from:S.from,to:S.to});
});
document.addEventListener('submit', e => { if(e.target.id==='position-limit'){e.preventDefault();const count=positionLimit(String(new FormData(e.target).get('max_positions')));if(!count){document.getElementById('position-error').textContent='Use a whole number from 1 to 100.';return;}return setState({max_positions:String(count),position_choice:'custom'});} if(e.target.id==='library-search-form'){e.preventDefault();return setState({library_q:document.getElementById('library-query').value.trim(),library_limit:'24',library_page:'1',source_page:'1'});}if (e.target.id !== 'period') return; e.preventDefault();
  const fd = new FormData(e.target), t = selectedSpan(S.tk), from = fd.get('from'), to = fd.get('to'), cost = '0',slippage=String(Math.max(0,+fd.get('slippage')||0)),rollover=String(Math.max(0,+fd.get('rollover')||0));
  if (!from || !to || from >= to) { $('#perr').textContent = 'From must be before To.'; return; }
  go(R.path, { ...S, from: from <= t.first ? '' : from, to: to >= t.last ? '' : to, cost,slippage,rollover }); });
// YT strategies: the backtest and optimization pages share one From/To. A typed date applies by itself once it is a
// complete date inside the data range, and Run optimization, Apply and the page links use a date not yet applied.
function ytPeriod() {
  const f = document.getElementById('period'); if (!f || !STRAT || !isYt(STRAT.v)) return {};
  const [a, b] = [f.elements.from, f.elements.to]; if (!a?.value || !b?.value || !a.checkValidity() || !b.checkValidity() || a.value >= b.value) return {};
  const t = selectedSpan(S.tk), win = coverageWindow(STRAT.v, S.tk), from = a.value <= t.first || (win && !S.from && a.value === win.from) ? '' : a.value, to = b.value >= t.last || (win && !S.to && b.value === win.to) ? '' : b.value;
  return from === (S.from || '') && to === (S.to || '') ? {} : { from, to };
}
let ytPeriodTimer; document.addEventListener('change', e => { if (e.target.form?.id !== 'period' || !['from', 'to'].includes(e.target.name)) return; clearTimeout(ytPeriodTimer); ytPeriodTimer = setTimeout(() => { const p = ytPeriod(); if (Object.keys(p).length) go(R.path, { ...S, ...p }); }, 700); });
document.addEventListener('click', e => { const a = e.target.closest('#open-optimization, .back-btn[href^="#/s/"]'); if (!a) return; const p = ytPeriod(); if (!Object.keys(p).length) return; e.preventDefault(); clearTimeout(ytPeriodTimer); go('/' + (R.optim ? 's' : 'o') + '/' + encodeURIComponent(STRAT.v.id), { ...S, ...p }); }, true);
document.addEventListener('click', e => { const b = e.target.closest('[data-period]'); if (!b) return; const [from, to] = b.dataset.period.split('|'); go(R.path, { ...S, from, to }); });
document.addEventListener('keydown', e => { if (e.key === 'Enter' && e.target.matches?.('.bookcard[data-book]')) e.target.click(); });
document.addEventListener('keydown', e => { if (e.key === 'Escape') document.querySelectorAll('details.dd[open]').forEach(d => d.removeAttribute('open')); });
document.addEventListener('click', e => { const bk = e.target.closest('[data-uni-basket],[data-uni-set]'); if (!bk || bk.disabled || !STRAT?.v) return; const id = STRAT.v.id;
  if (bk.dataset.uniBasket) return setState({ run_scope: 'universe', basket: '', tk: 'NIFTY 50', only: '', uni_basket: bk.dataset.uniBasket === uniDefaultBasket(id) ? '' : bk.dataset.uniBasket });
  return setState({ run_scope: 'universe', basket: '', tk: 'NIFTY 50', only: '', uni_set: bk.dataset.uniSet === 'cp' ? '' : bk.dataset.uniSet }); });
document.addEventListener('change', e => {if(e.target.id==='yt-stock')return setState({run_scope:'universe',basket:'',tk:'NIFTY 50',from:'',to:'',only:e.target.value});if(e.target.id==='strategy-stock'){if(!e.target.value)return;return setState({run_scope:'single',basket:'',tk:e.target.value,from:'',to:''});}if(e.target.id==='library-stock')return setState({tk:e.target.value,from:'',to:''});if(e.target.id==='library-instrument'){const value=e.target.value;if(value.startsWith('i:'))return setState({basket:'',tk:value.slice(2),from:'',to:''});const key=value.slice(2),basket=BASKETS.find(b=>b.id===key);if(basket?.symbols?.length)return setState({basket:key,tk:basket.symbols[0],from:'',to:''});return;}if(e.target.id==='library-sort')return setState({library_sort:e.target.value,library_limit:'24',library_page:'1',source_page:'1'}); if(e.target.id==='basket-universe'){const base=S.basket.replace(/_past$/,''),key=base+(e.target.value==='current_plus_past'?'_past':''),basket=BASKETS.find(b=>b.id===key),symbol=S.tk.split(':').at(-1),match=basket?.symbols?.find(x=>x.split(':').at(-1)===symbol);return setState({basket:key,tk:match||basket?.symbols?.[0]||S.tk,from:'',to:''});} if(e.target.id==='basket-symbol')return setState({tk:e.target.value});const k = e.target.dataset?.key; if (k) setState({ [k]: e.target.value }); });
let qt; document.addEventListener('input', e => { if (e.target.id !== 'q') return; clearTimeout(qt); qt = setTimeout(() => setState({ q: e.target.value }), 300); });
addEventListener('hashchange', () => { if(!GURUS_API.length||!ALLCAT.length)return;readRoute(); render(); });

const THEMES = [['midnight','Midnight','linear-gradient(135deg,#0d1727 50%,#4eade8 50%)'],['forest','Forest','linear-gradient(135deg,#eef4ed 50%,#357552 50%)'],['sand','Sand','linear-gradient(135deg,#f6f0e5 50%,#bd6935 50%)'],['plum','Plum','linear-gradient(135deg,#f6edf3 50%,#8c477d 50%)'],['cobalt','Cobalt','linear-gradient(135deg,#edf2fc 50%,#315ad6 50%)'],['steel', 'Steel', 'linear-gradient(135deg,#f3f3f3 50%,#2f5a8f 50%)'], ['aqua', 'Aqua', 'linear-gradient(135deg,#ffffff 50%,#10adbc 50%)'], ['lagoon', 'Lagoon', 'linear-gradient(135deg,#e9f8fa 50%,#00798a 50%)'], ['glacier', 'Glacier', 'linear-gradient(135deg,#f8fcfd 50%,#3cc0c9 50%)']];
function setTheme(t, redraw) { if (t === 'steel') delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = t; try { localStorage.setItem('vault-theme', t); } catch (e) {}
  document.querySelectorAll('#themes button').forEach(b => b.classList.toggle('on', b.dataset.theme === t)); if (redraw && ALLCAT.length) { lastName = ''; render(); } }
async function initializeRuntime(account) {
  try {
    setTheme(account.settings.theme||'aqua', false);
    const get = async u => { const response = await fetch(API + u); if (!response.ok) throw new Error(await response.text()); return response.json(); };
    const [connorsCat, rossCat, chandeCat, tk, gurus] = await Promise.all([get('/guru/connors/strategies'), get('/guru/ross/strategies'), get('/guru/chande/strategies'), get('/tickers'), get('/gurus')]);
    const pristineCat=gurus.some(owner=>owner.id==='pristine')?await get('/guru/pristine/strategies'):[];
    const ytCat=gurus.some(owner=>owner.id==='yt')?await get('/guru/yt/strategies'):[];
    const cat=[...connorsCat.map(v=>({...v,api:'connors',guru:BOOK[v.book]?.guru.id||'connors'})),...rossCat.map(v=>({...v,api:'ross',guru:'ross'})),...chandeCat.map(v=>({...v,api:'chande',guru:'chande'})),...pristineCat.map(v=>({...v,api:'pristine',guru:'pristine'})),...ytCat.map(v=>({...v,api:'yt',guru:'yt'}))]; // guru = the book's author; api = the engine namespace that serves it
    TICK_ALL_IN=tk;tk.forEach(t=>{if(!LABEL[t.ticker])LABEL[t.ticker]=t.symbol||t.ticker.split(':').at(-1);});
    TICKERS = TICK_IN = tk.filter(t => IN_INDICES.includes(t.ticker)); GURUS_API = gurus;
    try { const u = await (await fetch(API_US + '/tickers')).json(); TICK_ALL_US=u;u.forEach(t=>{if(!LABEL[t.ticker])LABEL[t.ticker]=t.symbol||t.ticker.split(':').at(-1);}); TICK_US = US_INDICES.map(k => u.find(t => t.ticker === k)).filter(Boolean); if (!TICK_US.length) TICK_US = null; } catch (e) { TICK_US = null; }
    ALLCAT = cat.map((v, i) => ({ ...v, ord: i, book: v.book || 'stc9', style: v.group.split(' · ')[0] })); CAT = marketCat();
    if (ytCat.length) { try { await optSchema(API_IN); } catch (e) { /* rule templates fall back to their placeholders */ } }
    if (ytCat.some(v => v.id?.startsWith('ytath_'))) { try { const r = await fetch(API_IN + '/guru/yt/ath-universe'); if (r.ok) { const u = await r.json(); if (Array.isArray(u?.ids)) ATH_UNIVERSE = u; } } catch (e) { /* no ATH data: the ATH sub-variants stay unavailable */ } }
    if (ytCat.some(v => v.id?.startsWith('ytbtst_'))) { try { const r = await fetch(API_IN + '/guru/yt/universe'); if (r.ok) { const u = await r.json(); if (Array.isArray(u?.ids)) UNIVERSE = u; } } catch (e) { /* no stock universe: universe choices stay hidden */ } }
    Object.values(BOOK).forEach(b => { if (CAT.some(v => v.book === b.id)) b.live = true; });
    const metadataFiles=['baskets.json','baskets-us.json','book-coverage.json','pristine-reading-index.json','book-file-index.json','book-categories.json','research-share-library.json','tharp-source-review.json','library-progress.json','research-share-review.json'];
    const metadata=await Promise.all(metadataFiles.map(async file=>{try{const response=await fetch('data/'+file);return response.ok?await response.json():null;}catch{return null;}}));
    for(const [offset,region] of [[0,'in'],[1,'us']]){try{const data=metadata[offset];if(data){const pool=region==='us'?TICK_ALL_US:TICK_ALL_IN,keys=new Set(pool.map(t=>t.ticker)),items=(Array.isArray(data)?data:data.baskets||[]).map(b=>({...b,symbols:(b.symbols||[]).filter(key=>keys.has(key))}));if(region==='us')BASKETS_US=items;else BASKETS_IN=items;}}catch{}}
    if(metadata[2])BOOK_COVERAGE=metadata[2];
    try{const data=metadata[3];if(data){const entries=(data.entries||[]).map(b=>({...b,author_metadata:b.source_author,years_reported:b.year?[String(b.year)]:[]}));BOOK_COVERAGE.authors.pristine={id:'pristine',name:'Pristine Trading',source_titles:entries,catalogue_books:GURU.pristine.books.map(b=>({id:b.id,title:b.title,summary:b.blurb})),counts:{catalogue_books:3,source_title_candidates:entries.length,local_pdf_files:(data.counts?.source_locations||0)},note:'Three curated reading entries, plus related Pristine manuals and articles with their recorded author credits. Source-title candidates, document versions and split materials are not independently verified distinct authored books. Original source bytes were preserved. Selected TPM1 daily branches have separate source review; other sources, execution modes and original performance are not thereby certified.'};}}catch{}
    if(metadata[4])BOOK_FILE_INDEX=metadata[4];
    if(metadata[5])BOOK_CATEGORIES=metadata[5];
    try{if(metadata[6])mergeResearchLibrary(metadata[6]);}catch{}
    try{if(metadata[7])applyReadingReviews(metadata[7]);}catch{}
    if(metadata[8]){
      LIBRARY_PROGRESS=metadata[8];
      for(const link of LIBRARY_PROGRESS.approved_source_book_links||[]){const b=BOOK[link.book_id];if(b&&b.guru.id===link.author_id&&link.evidence&&typeof link.source_id==='string')b.sourceIds=[...new Set([...(b.sourceIds||[]),link.source_id])];}
    }
    try{if(metadata[9])applyShareReview(metadata[9]);}catch{}
    finalizeReadingIndex();
    readRoute(); render();
  } catch (error) { vaultStore.fail(vaultStore.begin('/'),error.message||'Could not reach the calculation engine.'); }
}



async function homeModel(){
 const selectedTicker=S.tk==='ALL'||S.market!=='india'?TICKERS[0].ticker:S.tk;
 const [h,readiness]=await Promise.all([heroStats(),getReadiness(selectedTicker,S.from||'',S.to||'')]);
 const inventoryRecords=libraryRecords(),allRecords=inventoryRecords.filter(record=>inLibraryScope(libraryScope(record,CURATED_AUTHOR_IDS),S.library_scope)),scopeAuthorIds=new Set(allRecords.flatMap(recordAuthorIds)),scopeAuthors=LIB.filter(author=>scopeAuthorIds.has(author.id)),authors=VaultLibraryBrowser.matchingAuthors(scopeAuthors,allRecords,S.library_q,S.book_category),live=LIB.filter(g=>g.books.some(b=>b.live));
 const scoped=Object.fromEntries(await Promise.all(live.map(async g=>[g.id,await heroStats(g.id)])));
 const summary=await getSum(h.tk,S.from||'',S.to||''),states=Object.fromEntries(LIB.map(g=>[g.id,VaultLibraryDiscovery.authorAvailability(g.books.map(b=>b.id),CAT,summary,readiness)]));
 pristineAuthorStatus(states.pristine);
 const scopedRecords=scopedLibraryRecords(allRecords),scopeCounts=new Map([...scopedRecords].map(([id,records])=>[id,{total:records.length,catalogue:records.filter(record=>record.kind==='catalogue').length,reading:records.filter(record=>record.kind==='source').length}]));
 const pool=S.region==='us'?TICK_ALL_US:TICK_ALL_IN,etfs=S.region==='us'?TICK_ALL_US.filter(t=>['SPY','QQQ','MDY','IWM','SMH','DIA'].includes(t.ticker)):[];
 const selected={region:S.region,tk:h.tk,basket:S.basket,label:LABEL[h.tk]||h.tk,kind:S.basket?'stock':etfs.some(t=>t.ticker===h.tk)?'ETF':'index',from:S.from||'',to:S.to||''};
 // Discover compatible measured results on at most one stock and one ETF.
 // This never runs an entire basket or treats admissions as backtest results.
 const references=[];
 if(S.library_view!=='books'&&authors.some(g=>!states[g.id].availableFamilies&&CAT.some(v=>v.guru===g.id&&v.status==='live'&&v.instrument_policy?.allowed_types?.some(type=>['individual_stock','etf'].includes(type))))) {
  for(const context of referenceBacktestContexts(S.region,BASKETS,pool,etfs)) {
   try {const sample=await getSum(context.tk,context.from,context.to),ready=await getReadiness(context.tk,context.from,context.to);references.push({context,summary:sample,readiness:ready});}
   catch { /* No successful measured summary: leave the selected context honest. */ }
  }
 }
 const contexts=Object.fromEntries(authors.map(g=>{
  const selectedResult={context:selected,availability:states[g.id],summary,readiness};
  const alternatives=references.map(sample=>({...sample,availability:VaultLibraryDiscovery.authorAvailability(g.books.map(b=>b.id),CAT,sample.summary,sample.readiness)}));
  return [g.id,chooseBacktestContext(selectedResult,alternatives)];
 }));
 const progress=currentLibraryProgress();
 const list=orderAuthors(authors,{sort:S.library_sort,counts:Object.fromEntries([...scopeCounts].map(([id,c])=>[id,c.total])),availability:Object.fromEntries(Object.entries(contexts).map(([id,c])=>[id,{...c.availability,availableFamilies:progress.counts(GURU[id].books.map(b=>b.id)).backtested}])),positions:Object.fromEntries(LIB.map((g,i)=>[g.id,i]))});
 const books=S.library_view==='books'?VaultLibraryBrowser.search(allRecords,{query:S.library_q,category:S.book_category,sort:S.library_sort}).filter(r=>!S.book_progress||recordProgress(r).status===S.book_progress):[];
 const total=S.library_view==='books'?books.length:list.length,pages=Math.max(1,Math.ceil(total/24)),page=Math.max(1,Math.min(pages,+S.library_page||1)),offset=(page-1)*24;
 const authorCards=list.slice(offset,offset+24).map(g=>{
  const counts=scopeCounts.get(g.id)||{total:0,catalogue:0,reading:0},result=contexts[g.id],state=result.availability,context=result.context,changed=context!==selected,matching=VaultLibraryBrowser.search(scopedRecords.get(g.id)||[],{query:S.library_q,category:S.book_category});
  const preview=matching.length?matching.slice(0,2):(g.books.some(b=>b.live)?g.books.filter(b=>b.live):g.books).slice(0,2);
  const availableIDs=new Set(state.variantReasons.filter(v=>v.status==='available').map(v=>v.id));
  const best=CAT.filter(v=>v.guru===g.id&&availableIDs.has(v.id)).map(v=>({v,s:result.summary[v.id]?.stats})).filter(row=>TRUST.hasResult(result.summary[row.v.id])&&row.s.trades>=20&&Number.isFinite(row.s.sharpe)).sort((a,b)=>b.s.sharpe-a.s.sharpe)[0];
  const link=href('/guru/'+g.id,{view:'grid',book:'',book_category:S.book_category,region:context.region,tk:context.tk,basket:context.basket,from:context.from,to:context.to,tab:'sharpe'});
  scoped[g.id]={...scoped[g.id],tk:context.tk,tested:state.availableFamilies,booksTested:new Set(state.variantReasons.filter(v=>v.status==='available').map(v=>v.book)).size,best,href:link};
  return {id:g.id,curated:CURATED_AUTHOR_IDS.has(g.id),name:['?','unknown','anonymous'].includes(g.name?.trim().toLowerCase())?'Source credit awaiting review':g.name,href:link,photo:photoOf(g),initials:initials(g.name),tagline:gTag(g),preview:preview.map(b=>b.title).join(' · ')+(matching.length>2?` · +${matching.length-2} more`:''),entries:libraryEntryCounts(g.id).total,catalogue:libraryEntryCounts(g.id).catalogue,reading:libraryEntryCounts(g.id).reading,families:progress.counts(g.books.map(b=>b.id)).strategies,available:progress.counts(g.books.map(b=>b.id)).backtested,availableContext:state.availableFamilies?`${context.label} · ${context.kind} · ${context.region==='us'?'US':'India'}${context.from||context.to?' · '+(context.from||'First available bar')+' to '+(context.to||'Latest available bar'):''}`:undefined,contextChanged:changed,openLabel:changed?`Open ${context.kind} backtests`:'Open the vault',status:state.status,statusTitle:progress.counts(g.books.map(b=>b.id)).backtested?'Backtested':'In progress',statusText:progress.counts(g.books.map(b=>b.id)).backtested?'Available computed backtests. Book rules and declared adaptations remain labelled separately.':'Books remain available for reading. Open this author to see In progress and Not Backtested entries with their reasons.',sharpe:best?fx(best.s.sharpe):null};
 });
 const bookCards=books.slice(offset,offset+24).map(record=>{const b=record.book||record.source||record;return {id:record.id,kind:record.kind,title:record.title,author:record.kind==='source'&&record.source?.author_metadata&&record.source.author_metadata!=='?'?record.source.author_metadata+' · '+record.authorName:record.authorName,summary:record.kind==='catalogue'?readingSummary(record.author||GURU[record.authorId],b):b.summary||'Original reading material indexed from the source vault. Content and objective rules still need review.',year:String(record.year||'Edition year unconfirmed'),scope:libraryScope(record,CURATED_AUTHOR_IDS),categories:libraryScope(record,CURATED_AUTHOR_IDS)==='trading'?record.categories:[],href:libraryBookLink(record),live:!!b.live,progressStatus:recordProgress(record).status,progressLabel:recordProgress(record).label,progressReason:recordProgress(record).reason};});
 const searchAuthors=VaultLibraryBrowser.matchingAuthors(scopeAuthors,allRecords,'',S.book_category).map(g=>({id:g.id,curated:CURATED_AUTHOR_IDS.has(g.id),name:['?','unknown','anonymous'].includes(g.name?.trim().toLowerCase())?'Source credit awaiting review':g.name,photo:photoOf(g),initials:initials(g.name),href:guruLink(g.id)}));
 const indices=S.region==='us'?TICK_US:TICK_IN;
 const enabled=ALLCAT.filter(variant=>variant.status==='live');
 return {listedPeople:list.filter(g=>g.person).length,identifiedAuthors:identifiedAuthorCount,libraryTotals:libraryTotals(LIB.flatMap(g=>g.books.map(b=>b.id))),strategiesFound:progress.strategiesFound,backtestedStrategies:progress.backtestedStrategies,progressCurrent:progress.current,summaryIssues:summaryIssues(h.tk,S.from||'',S.to||''),authorStats:scoped,market:libraryMarket(S),implementedMethods:new Set(enabled.map(variant=>variant.book+':'+variant.family)).size,enabledVariants:enabled.length,implementedBooks:new Set(enabled.map(variant=>variant.book)).size,authors:scopeAuthorIds.size,curatedAuthors:CURATED_AUTHOR_IDS.size,scope:S.library_scope,inventoryGroups:LIB.length,inventoryEntries:inventoryRecords.length,entries:allRecords.length,families:h.tested,symbol:LABEL[h.tk]||h.tk,symbolCount:TICKERS.length,basket:!!S.basket,authorCards,searchAuthors,bookCards,page:{page,pages,total,key:'library_page',label:S.library_view==='books'?'books':'authors'},query:S.library_q,category:S.book_category,view:S.library_view,etfs:etfs.map(t=>({ticker:t.ticker,name:LABEL[t.ticker]||t.ticker})),indices:(indices||[]).map(t=>({ticker:t.ticker,name:LABEL[t.ticker]||t.ticker})),baskets:BASKETS.map(b=>({id:b.id,name:b.name,symbols:b.symbols||[]})),symbols:TICKERS.map(t=>({ticker:t.ticker,name:LABEL[t.ticker]||t.ticker})),usAvailable:!!TICK_US,guilloche:GUILL,connorsLink:guruLink('connors'),strategiesLink:href('/strategies',{tk:S.tk,basket:S.basket})};
}
let startup;
window.VaultRuntime={
 bootstrap:account=>startup||(startup=initializeRuntime(account)),commit:commitReactPage,cleanup:cleanupCharts,patch:setState,changeTheme:theme=>setTheme(theme,true),
 activateReportTab:resizeVisibleReportCharts,pristineCurrent,exportPristine:(data,view,kind)=>VaultUI.pristineExport(data,view,kind),
 openAuthor:(id,event)=>window.openGuru(id,event),closeAuthor:()=>window.closeGuru(),actions:()=>VaultUI.actions(),themes:THEMES,
 saved:()=>VaultUI.savedList().map(item=>({...item,book:BOOK[ALLCAT.find(v=>v.id===item.id)?.book]?.title||'',symbol:LABEL[item.tk]||item.tk})),removeSaved:key=>VaultUI.writeSaved(VaultUI.savedList().filter(item=>item.key!==key))
};

function readingModel(){
 const record=libraryRecords().find(item=>item.kind==='source'&&item.id===R.id),indexed=SOURCE_ROW_BY_ID.get(R.id);
 const author=record?.author||indexed?.author,source=record?.source||indexed?.source;if(!author||!source)throw new Error('Reading source unavailable in the current index.');
 const paths=new Set([...(source.local_copies||[]).map(item=>item.path),...(source.files||[]).flatMap(file=>[...(file.local_candidates||[]),...(file.verified_local_copy?.path?[file.verified_local_copy.path]:[])]),...(source.verified_local_copy?.path?[source.verified_local_copy.path]:[])]);
 const files=[...new Map([...paths].map(path=>BOOK_FILE_INDEX.files[path.replace(/^Books\//,'')]).filter(Boolean).map(file=>[file.id,{id:file.id,filename:file.filename,attachment:file.disposition==='attachment'}])).values()];
 const list=value=>Array.isArray(value)?value.join('; '):value||'';const review=source.review;
 const scope=libraryScope({kind:'source',title:source.title,authorId:author.id,source},CURATED_AUTHOR_IDS);
 return {scopeNotice:scope==='technical'?'Supporting technical reference. This entry is not counted as a trading-strategy or money-management book.':scope==='pending'?'Market relevance is awaiting source review. This inventory entry is not counted as a trading book.':'Trading-library relevance is screened separately from complete rule verification and published performance.',id:source.id,title:source.title,authorName:author.name,authorHref:guruLink(author.id),credit:source.source_identity?.credit||(source.author_metadata&&source.author_metadata!=='?'?source.author_metadata:'Individual author unconfirmed'),year:String(source.source_identity?.year||'Edition year unconfirmed'),categories:scope==='trading'?(categoryLookup(source.id,source,author,'source')?.categories||[]).map(canonicalCategory):[],summary:source.summary||'This reading source is indexed from the vault. A detailed content summary and mechanical rule review are still pending.',publisher:source.publisher_source&&/^https?:\/\//.test(source.publisher_source)?source.publisher_source:null,files,source:true,sourceFiles:(source.files||[]).map(file=>file.filename),countBasis:source.count_basis||'An indexed source-title candidate, not a certified count of distinct authored books.',review:review?{status:review.status_label||'Bounded source review',limits:review.reading_limits||'This is a bounded content review; it does not certify the complete book or original trading performance.',outline:(review.outline||[]).slice(0,8).map(topic=>({label:topic.label,detail:topic.detail||''})),rules:(review.rules||[]).map(rule=>({name:rule.name,completeness:rule.complete?'Complete for the stated scope':rule.completeness_kind||'Incomplete entry/exit system',specified:[rule.inputs&&'Inputs: '+list(rule.inputs),rule.entry&&'Entry: '+rule.entry,rule.exit&&'Exit: '+rule.exit].filter(Boolean),needed:[list(rule.data_requirement),list(rule.missing)].filter(Boolean)})),evidence:(review.evidence||[]).map(evidence=>({file:evidence.file||'Source document',pages:Array.isArray(evidence.page_range)?evidence.page_range.map(page=>Array.isArray(page)?page[0]===page[1]?String(page[0]):page.join('–'):page).join(', '):String(evidence.page_range||''),finding:evidence.finding||''}))}:null};
}
