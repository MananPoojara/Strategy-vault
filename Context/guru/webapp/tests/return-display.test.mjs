import test from 'node:test';
import assert from 'node:assert/strict';
import {returnDisplay} from '../../frontend/react/src/services/return-display.ts';

const closed = ret => ({open: false, ret});
const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-12, `${actual} !== ${expected}`);

test('unrounded reporting returns take precedence without changing legacy ledger fields', () => {
  const trades = [{...closed(.004412), ret_full_precision:.0044123456789},
    {...closed(-.003457), ret_full_precision:-.0034567890123}];
  near(returnDisplay({trades}).value, .0044123456789-.0034567890123);
  near(returnDisplay({trades},'compound').value, (1+.0044123456789)*(1-.0034567890123)-1);
  assert.equal(trades[0].ret,.004412);
  assert.equal(returnDisplay({trades:[{...closed(.1),ret_full_precision:null}]}).value,null);
});

test('fixed view sums actual net closed-trade returns, independently of rounded averages or summary stats', () => {
  const result = returnDisplay({trades: [closed(.2), closed(-.1)], stats: {avg_net_pnl: .44, trades: 211, net_profit: 6000}});
  near(result.value, .1);
  assert.equal(result.mode, 'fixed');
  assert.equal(result.closedTrades, 2);
  assert.equal(result.label, 'Net Profit');
  assert.match(result.description, /Additive/);
  assert.match(result.description, /No share quantity supplied/);
  assert.equal(result.unavailableReason, null);
});

test('compounding multiplies actual closed-return factors: +20% followed by -10% is +8%', () => {
  const result = returnDisplay({trades: [closed(.2), closed(-.1)]}, 'compound');
  near(result.value, .08);
  assert.equal(result.mode, 'compound');
  assert.match(result.reportMode, /Compounded closed-trade/);
  assert.equal(result.closedTrades, 2);
});

test('terminal open marks and open rows with unavailable returns never contribute to either view', () => {
  const data = {trades: [closed(.1), {open: true, ret: 4}, {open: true, ret: null}]};
  for (const mode of ['fixed', 'compound']) {
    const result = returnDisplay(data, mode);
    near(result.value, .1);
    assert.equal(result.closedTrades, 1);
    assert.equal(result.unavailableReason, null);
  }
});

test('empty or exclusively open positions have zero closed return and zero closed count', () => {
  for (const trades of [[], [{open: true, ret: .5}]]) {
    for (const mode of ['fixed', 'compound']) {
      const result = returnDisplay({trades}, mode);
      assert.equal(result.value, 0);
      assert.equal(result.closedTrades, 0);
    }
  }
});

test('missing trade arrays, ambiguous open flags and non-object rows hold the display', () => {
  for (const data of [null, {}, {trades: null}, {trades: {}}, {trades: [null]}, {trades: [{ret: .1}]}, {trades: [{open: 'false', ret: .1}]}]) {
    const result = returnDisplay(data);
    assert.equal(result.value, null);
    assert.equal(result.closedTrades, null);
    assert.ok(result.unavailableReason);
  }
});

test('missing, nonnumeric or nonfinite closed returns are unavailable rather than silently dropped', () => {
  for (const ret of [undefined, null, '0.1', Number.NaN, Infinity, -Infinity]) {
    for (const mode of ['fixed', 'compound']) {
      const result = returnDisplay({trades: [closed(.05), closed(ret)]}, mode);
      assert.equal(result.value, null);
      assert.equal(result.closedTrades, 2);
      assert.ok(result.unavailableReason);
    }
  }
});

test('nonpositive compound factors hold the model while additive losses remain actual reported losses', () => {
  for (const ret of [-1, -1.25]) {
    const data = {trades: [closed(.2), closed(ret)]};
    assert.equal(returnDisplay(data, 'compound').value, null);
    assert.match(returnDisplay(data, 'compound').unavailableReason, /positive/);
    near(returnDisplay(data, 'fixed').value, .2 + ret);
  }
});

test('arithmetic overflow or exhausted compound precision hold the display without mutating trades', () => {
  const data = Object.freeze({trades: Object.freeze([Object.freeze(closed(Number.MAX_VALUE)), Object.freeze(closed(Number.MAX_VALUE))])});
  assert.equal(returnDisplay(data, 'fixed').value, null);
  assert.equal(returnDisplay(data, 'compound').value, null);
  assert.equal(returnDisplay({trades: Array.from({length: 1100}, () => closed(-.5))}, 'compound').value, null);
  assert.equal(data.trades[0].ret, Number.MAX_VALUE);
});

test('small unrounded returns and weighted-position net returns are read directly from each ret', () => {
  const data = {trades: [{...closed(.00441234), legs: [[1, 100, .25], [2, 98, .75]]}, closed(.00449999)]};
  near(returnDisplay(data).value, .00891233);
  near(returnDisplay(data, 'compound').value, (1.00441234 * 1.00449999) - 1);
  assert.equal(returnDisplay(data).closedTrades, 2);
});
