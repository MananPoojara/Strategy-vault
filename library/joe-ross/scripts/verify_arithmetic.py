#!/usr/bin/env python3
"""Recompute the recorded source examples independently of Rust and market data.

Run from any directory. The three CCI values are a private arithmetic fixture,
never a dataset or published performance result. This validates arithmetic,
not strategy fidelity, fills, market observations or a statistical sample.
"""
from decimal import Decimal as D
import json
import math
from pathlib import Path

record = json.loads((Path(__file__).resolve().parents[1] / 'INDEPENDENT-MATH-CHECK.json').read_text())

# Printed bond quotes, visually checked on Hook printed p.138 / PDF p.70.
pairs = [((103,20),(103,8)), ((103,16),(103,2)),
         ((103,14),(102,19)), ((102,27),(101,29)), ((101,29),(100,28))]
ranges = [(h[0]*32+h[1])-(l[0]*32+l[1]) for h,l in pairs]
assert ranges == record['bond']['ranges_32nds']
assert sum(ranges) == record['bond']['sum']
assert D(sum(ranges))/D(len(ranges)) == D(str(record['bond']['mean']))
assert 103*32+14-23 == 102*32+23 == record['bond']['printed_integer_subtraction']['stop_32nds']

# Manual printed p.137: projected values, not executable observations.
open_, high, low, close = map(D, ('24','25','23','23.5'))
x = (open_+high+low+2*close)/5
lo, hi = 2*x-high, 2*x-low
expected = record['congestion_projection']
for key, actual in [('weighted_x',x), ('projected_low',lo),
                    ('projected_high',hi), ('projected_typical',(lo+hi)/2)]:
    assert actual == D(expected[key]), (key,actual,expected[key])

# Hook p.167 prints an integer projection without defining a general rounding rule.
projection = record['hook_projection_rounding']
open_, high, low, close = (D(projection[k]) for k in ('open','high','low','close'))
exact_x = (open_+high+low+2*close)/5
printed_x = D(projection['printed_weighted_x'])
assert exact_x != printed_x
for prefix, x in [('exact',exact_x), ('printed',printed_x)]:
    lo, hi = 2*x-high, 2*x-low
    for key, actual in [('weighted_x',x), ('projected_low',lo),
                        ('projected_high',hi), ('projected_midpoint',(lo+hi)/2)]:
        assert actual == D(projection[f'{prefix}_{key}'])
assert projection['general_rounding_policy_verified'] is False
assert projection['observed_market_prices'] is False

fixture = list(map(D, record['cci_scale_conflict']['private_arithmetic_fixture']))
mean = sum(fixture)/len(fixture)
md = sum(abs(v-mean) for v in fixture)/len(fixture)
cci = record['cci_scale_conflict']
printed = (fixture[-1]-mean)/(D(cci['printed_divisor'])*md)
standard = (fixture[-1]-mean)/(D(cci['standard_divisor'])*md)
assert printed == D(cci['printed_output'])
assert standard == D(cci['standard_output'])
assert standard/printed == D(cci['ratio']) == 100

assert math.isclose(math.erf(2/math.sqrt(2))*100, record['normal_two_sigma_percent'], abs_tol=1e-12)
assert math.isclose(20/22*100, record['gap_chart_reaction_percent'], abs_tol=1e-12)
assert record['performance_verified'] is False
spread = json.loads((Path(__file__).resolve().parents[1] / 'SPREADS-ARITHMETIC-CHECK.json').read_text())
assert D('17.5')/32*1000 == D(spread['conditional_fraction_value'])
assert (3+D('17.5')/32-D(26)/32)*1000 == D(spread['conditional_profit_before_costs'])
assert D(spread['conditional_fraction_value']) != D(spread['printed_fraction_value'])
assert D(spread['conditional_profit_before_costs']) != D(spread['printed_profit'])
assert spread['actual_trade_verified'] is False
option=record['manual_option_example']
for buy,multiple,exit_,gain in zip(option['purchase_values'],option['exit_value_multiples'],option['conditional_exit_values'],option['conditional_gains_before_costs']):
    assert D(buy)*D(multiple)==D(exit_)
    assert D(exit_)-D(buy)==D(gain)
assert option['actual_trade_verified'] is False
ticks=record['manual_sp_tick_conflict']
assert D(ticks['printed_unit_value'])*D(ticks['quantity_ticks'])==D(ticks['conditional_contract_value'])
assert D(ticks['conditional_contract_value'])*D(ticks['contracts'])==D(ticks['conditional_total_value'])
assert D(ticks['printed_contract_value'])/D(ticks['quantity_ticks'])==D(ticks['implied_unit_from_printed_total'])
assert ticks['contract_and_trade_verified'] is False
print('Source arithmetic matches the independent record; no backtest performance verified.')
