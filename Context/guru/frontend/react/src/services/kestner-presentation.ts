/** Display-only Kestner labels and recorded review copy. No data, account,
 * indicator, grouping, admission or authored-research conclusions are created. */
export interface CatalogueDisplayDefinition {
 readonly id?:unknown; readonly book?:unknown; readonly name?:unknown;
 readonly family?:unknown; readonly family_name?:unknown; readonly instrument_policy?:unknown;
}
type Json=Record<string,unknown>;
const record=(value:unknown):Json|null=>value!==null&&typeof value==='object'&&!Array.isArray(value)?value as Json:null;
const text=(value:unknown):string=>typeof value==='string'?value:'';
/** Preserve raw definitions/family IDs. Only the exact singleton placeholder
 * displays the actual backend method name; genuine family/variant labels stay. */
export function catalogueMethodLabel(def:CatalogueDisplayDefinition,definitions:readonly CatalogueDisplayDefinition[]):string {
 const fallback=text(def.family_name)||text(def.name)||text(def.id);
 if(def.book!=='lkqts'||def.family_name!=='Kestner single system'||typeof def.id!=='string'||!def.id||def.family!==def.id||!text(def.name).trim())return fallback;
 const members=definitions.filter(row=>row.book==='lkqts'&&row.family===def.family);
 return members.length===1&&members[0].id===def.id?text(def.name):fallback;
}
export interface SourceAssessmentRow {readonly key:string;readonly label:string;readonly status:string;readonly values:readonly string[]}
export interface KestnerSourceAssessment {readonly title:string;readonly subtitle:string;readonly verdict:string;readonly note:string;readonly rows:readonly SourceAssessmentRow[]}
/** Backend source-policy metadata is distinct from an attached authored GPT
 * conclusion. This presenter neither invents saved evidence nor certifies runs. */
export function kestnerSourceAssessment(strategy:CatalogueDisplayDefinition,result:unknown,issue:unknown,savedEvidence:unknown):KestnerSourceAssessment|null {
 const policy=record(strategy.instrument_policy);
 if(strategy.book!=='lkqts'||typeof strategy.id!=='string'||!strategy.id.startsWith('lkqts_')||!policy||policy.source_rule_reviewed!==true||savedEvidence)return null;
 const run=record(result),stats=record(run?.stats),problem=record(issue),dates=run?.dates,trades=stats?.trades;
 const computed=!issue&&!run?.error&&Array.isArray(dates)&&dates.length>1&&typeof trades==='number'&&Number.isFinite(trades)&&trades>=0;
 const basis=text(policy.source_basis),convention=text(policy.source_convention);
 const sourceValues=computed?['See “Source rules reviewed · computed market application” and its declared assumptions above.']: [basis||'The backend records source-rule review for this method; the selected run remains subject to its own data and account checks.'];
 const assumptions=computed?['The source-rule notice above states the reviewed interpretation, execution assumptions and limitations.']: [convention||basis||'Only the scope explicitly supplied by the backend policy is recorded here.'];
 return {title:'Saved GPT conclusion',subtitle:'Saved analyst record and source-rule review are separate',verdict:'No saved GPT-specific source conclusion is attached to this section.',note:'The source-rule review remains recorded. Its existence does not supply an authored GPT assessment, percentile study or original-performance reproduction. Opening this page does not call an AI model.',rows:[
  {key:'saved_conclusion',label:'Saved GPT conclusion',status:'Not saved',values:['No source-specific authored GPT conclusion is attached to this section. This is separate from the recorded source-rule review.']},
  {key:'source_review',label:'Source-rule review',status:'Recorded scope',values:sourceValues},
  {key:'assumptions',label:'Declared assumptions',status:'Recorded scope',values:assumptions},
  {key:'selected_ledger',label:'Selected ledger checks',status:policy.independent_selected_ledger_checked===true?'Recorded selected cases':'Not separately recorded',values:[policy.independent_selected_ledger_checked===true?'The backend records selected reference-ledger checks. Their stated scope does not independently certify every new instrument, date window or configuration.':'This section supplies no additional independent ledger-audit claim.']},
  {key:'current_selection',label:'Current selection',status:computed?'Computed report':'Withheld',values:[computed?'A computed report is available for the selected instrument and dates; its source, data and account guards still apply.':text(problem?.note)||'No supported computed report is supplied for this selection.']},
  {key:'original_performance',label:'Original book performance',status:policy.source_performance_reproduced===false?'Not reproduced':'Uncertified',values:[policy.source_performance_reproduced===false?'Source-rule review and simulated local results do not reproduce the original book study, population or sizing.':'This presenter establishes no original book-performance reproduction.']}
 ]};
}
