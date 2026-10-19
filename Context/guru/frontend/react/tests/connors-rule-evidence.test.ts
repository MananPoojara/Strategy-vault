import {describe,expect,it} from 'vitest';
import {makeRuleVisualModel,buildRuleEvidence,type RuleEvidence} from '../src/services/rule-evidence';
import {directionalPlus} from '../src/services/strategy-indicators';

const days=(n:number)=>Array.from({length:n},(_,i)=>new Date(Date.UTC(2020,0,i+1)).toISOString().slice(0,10));
const params={trend:2,rsi_n:2,exit_rsi:65,di_n:2,di_lt:10,days:2};
function model(closes:number[],id='dmi_3',p:Record<string,number>=params,trades:unknown[]=[]){
  const d=days(closes.length);return makeRuleVisualModel({strategy:{id,name:id,book:id.startsWith('spmt_')?'spmt':'stc9'},ticker:'TEST',symbol:'TEST',run:{dates:d,trades},prices:{ticker:'TEST',full_history:true,d,o:closes,h:closes.map(c=>c+1),l:closes.map(c=>c-1),c:closes},parameters:p});
}
const find=(e:RuleEvidence,i:number,prefix:string)=>{const c=e.at(i).find(c=>c.label.startsWith(prefix));expect(c,`Missing ${prefix}`).toBeDefined();return c!;};

describe('Connors source-specific input predicates',()=>{
  it('reproduces the independent hand-worked +DI seed, not ADX, and requires every consecutive bar',()=>{
    const m=model([9,10,13,13],'dmi_3',{...params,di_lt:5});m.prices={...m.prices!,h:[10,11,15,16],l:[8,8,3,2]};
    const e=buildRuleEvidence(m);expect(e.scope).toBe('exact');
    // First transitions: +DM1,0 / TR3,12 -> seed .5/7.5*100.
    expect(find(e,2,'Upward-movement').value).toBeCloseTo(100*.5/7.5,12);
    // Next tie gives +DM0, TR14 -> .25/10.75*100.
    expect(find(e,3,'Upward-movement').value).toBeCloseTo(100*.25/10.75,12);
    expect(find(e,3,'Upward-movement').passed).toBe(true);
    expect(find(e,3,'Consecutive').passed).toBe(false); // preceding +DI6.666 is not below5.
  });
  it('keeps equality in the RSI entry cap while the exit and SMA trend tests remain strict',()=>{
    const e=buildRuleEvidence(model([100,113,106],'dmi_1',{...params,days:1}));
    expect(find(e,2,'Entry cap').value).toBe(65);
    expect(find(e,2,'Entry cap').passed).toBe(true);
    expect(find(e,2,'Exit price').passed).toBe(false);
    const flat=buildRuleEvidence(model([10,10,10,10],'dmi_1',{...params,days:1,exit_rsi:50}));
    expect(find(flat,2,'Price trend').value).toBe(0);expect(find(flat,2,'Price trend').passed).toBe(false);
  });
  it('does not reseed Wilder inputs at a later selected From date or backfill warmup',()=>{
    const m=model([10,11,10,12,11,13]);m.dates=m.dates.slice(4);
    const e=buildRuleEvidence(m);expect(find(e,1,'Upward-movement').value).toBeNull();
    expect(find(e,4,'Entry cap').value).toBeCloseTo(50,12);
    expect(find(e,4,'Price trend').value).toBe(-.5);
    expect(buildRuleEvidence({...m,prices:{...m.prices!,full_history:false}}).scope).toBe('unavailable');
  });
  it('uses the reviewed SPMT movement tie budget without changing legacy STC values',()=>{
    // Decimal up/down .1 tie: binary subtraction gives up .10000000000000009
    // and down .09999999999999998. Only SPMT suppresses the roundoff +DM.
    const px={d:days(3),o:[1,1,1],h:[1,1.1,1.2],l:[.9,.8,.7],c:[1,1,1]};
    expect(directionalPlus(px,2,false)[2]).toBeGreaterThan(0);
    expect(directionalPlus(px,2,true)[2]).toBe(0);
    const m=model([1,1,1]);m.prices={...px,full_history:true};
    const stc=buildRuleEvidence(m),spmt=buildRuleEvidence({...m,id:'spmt_dmi_3',book:'spmt'});
    expect(find(stc,2,'Upward-movement').value).toBeGreaterThan(0);
    expect(find(spmt,2,'Upward-movement').value).toBe(0);
  });
  it('keeps final-window entry timing different for STC and SPMT',()=>{
    const m=model([100,101,50,25,20],'rsi2_4',{trend:2,rsi_n:2,exit_rsi:65,rsi_lt:4});m.dates=m.dates.slice(-1);
    const stc=buildRuleEvidence(m),spmt=buildRuleEvidence({...m,id:'spmt_rsi2_4',book:'spmt'});
    expect(find(stc,4,'Oversold').passed).toBe(true);
    expect(find(stc,4,'Entry-window').passed).toBe(false);
    expect(find(spmt,4,'Entry-window').passed).toBe(true);
  });
  it('reads the slow RSI period separately from the selected exit/cap RSI period',()=>{
    const e=buildRuleEvidence(model([10,11,10,12],'rsi4_25',{trend:2,rsi_n:2,exit_rsi:65,slow_n:3,slow_lt:70}));
    // RSI3 seed gains=(1+0+2)/3=1, loss=(0+1+0)/3=1/3 ->75.
    expect(find(e,3,'Slower entry').value).toBe(75);expect(find(e,3,'Slower entry').passed).toBe(false);
    expect(find(e,3,'Exit price').value).toBeCloseTo(100-100/6,12);
    expect(e.series.find(s=>s.key==='slow-rsi')?.label).toBe('Entry RSI(3)');
  });
  it('does not add a nonexistent RSI entry-cap predicate to RSI-level-only methods',()=>{
    const e=buildRuleEvidence(model([10,11,10,12],'rsi2_4',{trend:2,rsi_n:2,exit_rsi:65,rsi_lt:4}));
    expect(e.at(3).some(c=>c.label.startsWith('Entry cap'))).toBe(false);
  });
  it('blocks entry while already held, including the exit close, without borrowing another constituent’s state',()=>{
    const raw={entry_kind:'close',dir:1,entry:'2020-01-02',exit:'2020-01-04',legs:[[1,11,1]],exit_px:12,open:false};
    const m=model([10,11,10,12,11],undefined,params,[raw]),e=buildRuleEvidence(m);
    expect(find(e,1,'Position gate').passed).toBe(true);
    expect(find(e,2,'Position gate').passed).toBe(false);expect(find(e,3,'Position gate').passed).toBe(false);
    expect(find(e,4,'Position gate').passed).toBe(true);
    m.focusTicker='basket:test:AAA';m.trades[0].ticker='basket:test:BBB';
    expect(find(buildRuleEvidence(m),2,'Position gate').passed).toBeNull();
    expect(find(buildRuleEvidence(m),2,'Accepted basket exposure').value).toBe(0);
  });
  it('requires actual matching source-book identity rather than an ID-only match',()=>{
    const m=model([10,11,10,12]);
    expect(buildRuleEvidence({...m,book:''}).scope).toBe('unavailable');
    expect(buildRuleEvidence({...m,book:'hpetf'}).scope).toBe('unavailable');
    expect(buildRuleEvidence({...m,id:'spmt_dmi_3',book:'stc9'}).scope).toBe('unavailable');
  });
  it('does not trust a silently dropped secondary leg or malformed exit tuple',()=>{
    const p={trend:2,rsi_n:2,exit_rsi:75,ma:2,fall:2,first_lt:50,rsi_lt:40,add_fall:3};
    const base={entry_kind:'close',dir:1,entry:'2020-01-03',exit:'2020-01-05',legs:[[2,10,.5]],exit_px:11};
    const badLeg=model([10,11,10,12,11],'r3',p,[{...base,legs:[...base.legs,[999,12,.5]]}]);
    expect(badLeg.trades[0].legs).toHaveLength(1);expect(badLeg.ledgerAvailable).toBe(false);
    expect(find(buildRuleEvidence(badLeg),3,'Scale-in state').passed).toBeNull();
    const badExit=model([10,11,10,12,11],'r3',p,[{...base,exits:[[999,11,.5]]}]);
    expect(badExit.ledgerAvailable).toBe(false);
    expect(find(buildRuleEvidence(badExit),3,'Position gate').passed).toBeNull();
  });
  it('withholds source position permission for missing source dates and same-symbol overlapping holdings',()=>{
    const first={entry_kind:'close',dir:1,entry:'2020-01-02',exit:'2020-01-04',legs:[[1,11,1]],exit_px:12};
    const second={...first,entry:'2020-01-04',exit:'2020-01-05',legs:[[3,12,1]]};
    const overlap=model([10,11,10,12,11],undefined,params,[first,second]);
    // Generic snapshots permit other-family same-close reversal contracts.
    expect(overlap.ledgerAvailable).toBe(true);
    expect(find(buildRuleEvidence(overlap),2,'Position gate').passed).toBeNull();
    const missing=model([10,11,10,12,11],undefined,params,[first]);
    const px=missing.prices!;missing.prices={...px,d:px.d.filter((_,i)=>i!==1),o:px.o.filter((_,i)=>i!==1),h:px.h.filter((_,i)=>i!==1),l:px.l.filter((_,i)=>i!==1),c:px.c.filter((_,i)=>i!==1)};
    expect(find(buildRuleEvidence(missing),1,'Position gate').passed).toBeNull();
  });
  it('leaves missing parameters and missing or invalid recorded position ledgers unresolved',()=>{
    const m=model([10,11,10,12]);
    expect(buildRuleEvidence({...m,parameters:{...params,days:NaN}}).scope).toBe('unavailable');
    expect(find(buildRuleEvidence({...m,ledgerAvailable:false}),3,'Position gate').passed).toBeNull();
    const invalid=makeRuleVisualModel({strategy:{id:'dmi_1'},run:{dates:days(4),trades:[{entry:'bad',exit:'bad'}]},prices:m.prices,parameters:params});
    expect(invalid.ledgerAvailable).toBe(false);
  });
});

describe('course R3 is distinct from HPETF R3',()=>{
  const closes=[100,120,115,113,112,111,110,130];
  const p={trend:2,rsi_n:2,exit_rsi:75,ma:2,fall:2,first_lt:50,rsi_lt:40,add_fall:3};
  it('checks the first falling day of the last N decreases, not the preceding RSI or start of a longer streak',()=>{
    const e=buildRuleEvidence(model(closes,'r3',p));
    // At index6 the latest two-day window starts at5: RSI≈48.78 (<50).
    // The alternate HPETF first_ref=1 predecessor at4 is≈60.61 (>50), so the distinction is material.
    expect(find(e,6,'First falling').value).toBeCloseTo(100*1.25/(1.25+1.3125),12);
    expect(find(e,6,'First falling').passed).toBe(true);
    expect(find(e,6,'RSI falling').value).toBe(4);
    expect(e.note).toContain('HPETF has a selectable first_ref interpretation');
  });
  it('leaves falling streak predicates unknown until every required comparison has finite RSI',()=>{
    const e=buildRuleEvidence(model(closes,'r3',p));
    expect(find(e,2,'RSI falling').passed).toBeNull();
    expect(find(e,3,'RSI falling').passed).toBeNull();
    expect(find(e,4,'RSI falling').passed).toBe(true);
    expect(find(e,4,'Scale-in streak').passed).toBeNull();
    expect(find(e,5,'Scale-in streak').passed).toBe(true);
  });
  it('permits the second half only on a later held bar and gives an exit priority over adding',()=>{
    const raw={entry_kind:'close',dir:1,entry:'2020-01-06',exit:'2020-01-08',legs:[[5,111,.5],[6,110,.5]],exit_px:130,open:false};
    const e=buildRuleEvidence(model(closes,'r3',p,[raw]));
    expect(find(e,5,'Scale-in state').passed).toBe(false);expect(find(e,6,'Scale-in state').passed).toBe(true);
    expect(find(e,6,'Scale-in streak').passed).toBe(true);expect(find(e,6,'Scale-in priority').passed).toBe(true);
    expect(find(e,7,'Scale-in state').passed).toBe(false);expect(find(e,7,'Scale-in priority').passed).toBe(false);
    expect(find(e,7,'Exit price test · RSI').passed).toBe(true);
    expect(e.note).toContain('does not repeat the original entry tests');
  });
});
