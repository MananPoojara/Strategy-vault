import test from 'node:test';
import assert from 'node:assert/strict';
import {tradeExitCheck,tradeExitCheckMarkup} from '../../frontend/react/src/services/trade-exit-check.ts';
const trade=(reason,snapshot={})=>({reason,signal:'2020-01-06',open:false,dir:1,avg_px:100,at_exit:{date:'2020-01-06',...snapshot}});
test('a50SMA rule cannot consume a5SMA custom or fixed snapshot',()=>{
 const t=trade('Close < 50-day SMA',{vs_ma_p:-3,ma_period:5,ma_kind:'SMA',vs_sma5:-3});
 assert.equal(tradeExitCheck(t).available,false);
 assert.equal(tradeExitCheck({...t,at_exit:{...t.at_exit,ma_period:50}}).ok,true);
 assert.equal(tradeExitCheck(trade('Close < 50-day SMA',{vs_ma_p:-3})).available,false);
});
test('SMA matches exact labelled period and kind and remains finite',()=>{
 for(const patch of [{ma_kind:'EMA'},{ma_period:'50'},{vs_ma_p:NaN},{vs_ma_p:null},{vs_ma_p:'-2'}])
  assert.equal(tradeExitCheck(trade('Close < 50-day SMA',{ma_period:50,ma_kind:'SMA',vs_ma_p:-2,...patch})).available,false);
 assert.equal(tradeExitCheck(trade('Close > 200-day SMA',{vs_sma200:2})).ok,true);
 assert.equal(tradeExitCheck(trade('Close > 5-day SMA',{vs_sma5:-2})).ok,false);
});
test('RSI uses matching Wilder period and correctly named fixed RSI2 only',()=>{
 assert.equal(tradeExitCheck(trade('RSI(14) > 70',{rsi_p:90,rsi_period:2,rsi_kind:'Wilder',rsi2:90})).available,false);
 assert.equal(tradeExitCheck(trade('RSI(14) > 70',{rsi_p:90,rsi_period:14,rsi_kind:'Wilder'})).ok,true);
 assert.equal(tradeExitCheck(trade('RSI(14) > 70',{rsi_p:90,rsi_period:14,rsi_kind:'Cutler'})).available,false);
 assert.equal(tradeExitCheck(trade('RSI(2) > 70',{rsi_p:1,rsi_period:14,rsi_kind:'Wilder',rsi2:80})).ok,true);
});
test('signal date must match even when next-open fill is later',()=>{
 const t={...trade('Close < 50-day SMA',{ma_period:50,ma_kind:'SMA',vs_ma_p:-1}),exit:'2020-01-07',entry_kind:'open'};
 assert.equal(tradeExitCheck(t).available,true);
 for(const date of ['2020-01-07',undefined])assert.equal(tradeExitCheck({...t,at_exit:{...t.at_exit,date}}).available,false);
});
test('trend bands require the matching SMA period and mark missing context unavailable',()=>{
 assert.equal(tradeExitCheck(trade('≥ 1.5% below 100-day SMA',{vs_trend_p:-2,trend_period:200,trend_kind:'SMA'})).available,false);
 assert.equal(tradeExitCheck(trade('≥ 1.5% below 100-day SMA',{vs_trend_p:-2,trend_period:100,trend_kind:'SMA'})).ok,true);
});
test('open and unrecognized exit reasons do not acquire a fabricated check',()=>{
 assert.equal(tradeExitCheck({...trade('Close < 50-day SMA'),open:true}),null);
 for(const value of [null,{},trade('moving-average exit'),trade('<script>alert(1)</script>')])assert.equal(tradeExitCheck(value),null);
 assert.equal(tradeExitCheckMarkup(trade('<script>alert(1)</script>')),'');
 assert.match(tradeExitCheckMarkup(trade('Close < 50-day SMA')),/data-exit-check="unavailable"/);
});
test('recorded holding bars and direction-adjusted signal price checks stay diagnostic',()=>{
 assert.equal(tradeExitCheck({...trade('3 bars held'),bars:3}).ok,true);
 assert.equal(tradeExitCheck(trade('3 bars held')).available,false);
 assert.equal(tradeExitCheck({...trade('Close ≥ 2% profit',{close:95}),dir:-1}).ok,true);
 assert.equal(tradeExitCheck(trade('Close ≥ 2% profit',{close:105})).ok,true);
});
