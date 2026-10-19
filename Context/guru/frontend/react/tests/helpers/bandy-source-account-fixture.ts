/** Arithmetic-only reporting contract fixture, never source or market results. */
import {BANDY_ACCOUNT_MODEL} from '../../src/services/bandy-source-account';
export function bandyAccountFixture(macd=false, bps=5) {
  const dates=['2020-01-06','2020-01-07','2020-01-08','2020-01-09'],prices=[100,100,110,120],side=bps/10000;
  const target=macd ? .25 : 1, phase=macd ? 'open' as const : 'close' as const;
  const firstNotional=macd ? target : 1/(1+side),q1=firstNotional/100;
  const entryCost=firstNotional*side,exitNotional=q1*110,exitCost=exitNotional*side,gross=q1*10,net=gross-entryCost-exitCost;
  const secondEquity=1+net,secondNotional=macd ? target*secondEquity : secondEquity/(1+side),q2=secondNotional/120,secondCost=secondNotional*side;
  const days=dates.map((date,i)=>({date,source_index:100+i,
    equity:[1,1-entryCost,secondEquity,secondEquity-secondCost][i],
    gross_same_quantity_equity:[1,1,1+gross,1+gross][i],
    costs_paid:[0,entryCost,entryCost+exitCost,entryCost+exitCost+secondCost][i],
    free_cash:[1,1-firstNotional-entryCost,secondEquity,secondEquity-secondNotional-secondCost][i],
    quantity:[0,q1,0,q2][i],position_value:[0,firstNotional,0,secondNotional][i],direction:i===1||i===3 ? 1 : 0}));
  const trade_accounts=[{entry:dates[1],exit:dates[2],open:false,entry_signal_date:macd?dates[0]:dates[1],exit_signal_date:macd?dates[1]:dates[2],
    entry_phase:phase,exit_phase:phase,raw_instrument_gross_return:.1,entry_equity_units:1,entry_allocation_fraction:firstNotional,
    quantity:q1,entry_notional_units:firstNotional,exit_notional_units:exitNotional,gross_account_contribution:gross,net_account_contribution:net,
    entry_cost_account_contribution:entryCost,exit_cost_account_contribution:exitCost,gross_fraction_of_actual_entry_equity:gross,
    net_fraction_of_actual_entry_equity:net,open_liquidation_cost_estimate:0},
    {entry:dates[3],exit:dates[3],open:true,entry_signal_date:macd?dates[2]:dates[3],exit_signal_date:null,
    entry_phase:phase,exit_phase:'mark' as const,raw_instrument_gross_return:0,entry_equity_units:secondEquity,entry_allocation_fraction:secondNotional/secondEquity,
    quantity:q2,entry_notional_units:secondNotional,exit_notional_units:null,gross_account_contribution:0,net_account_contribution:-secondCost,
    entry_cost_account_contribution:secondCost,exit_cost_account_contribution:0,gross_fraction_of_actual_entry_equity:0,
    net_fraction_of_actual_entry_equity:-secondCost/secondEquity,open_liquidation_cost_estimate:secondNotional*side}];
  const trades=trade_accounts.map((t,i)=>({dir:1,entry:t.entry,exit:t.exit,open:t.open,entry_kind:t.entry_phase,
    legs:[[i===0?1:3,i===0?100:120,t.entry_allocation_fraction]],exit_px:i===0?110:120,exit_i:i===0?2:3,signal:t.open?t.exit:t.exit_signal_date,
    ret:Number(t.net_fraction_of_actual_entry_equity.toFixed(6)),ret_full_precision:t.net_fraction_of_actual_entry_equity,
    accounting:{symbol:'AAPL',signal_side:'Long',pts:i===0?10:0,days_held:i===0?1:0,cost_status:t.open?'actual_entry_cost_only_open_mark':'modeled_filled_side_costs',excursion_bars:0,excursion_status:'unavailable',mae_ret_observed:null,mfe_ret_observed:null,gross_ret:t.gross_fraction_of_actual_entry_equity,net_ret:t.net_fraction_of_actual_entry_equity,
      fee_ret:0,slippage_ret:(t.entry_cost_account_contribution+t.exit_cost_account_contribution)/t.entry_equity_units,
      gross_pnl:null,net_pnl:null,taxes:null,fees:null,slippage:null,mae:null,mfe:null}}));
  return {id:macd?'hbiab_macd_example':'hbiab_ma5_20',book:'hbiab',dates,benchmark_symbol:'AAPL',close:prices,equity:days.map(d=>d.equity),gross_equity:days.map(d=>d.gross_same_quantity_equity),trades,
    costs:{fees_bps:0,slippage_bps:bps,rollover_bps:0,total_side_bps:bps},
    stats:{total_return:days.at(-1)!.equity-1,net_profit:net,closed_return_sum:net,avg_net_pnl:net,trades:1},gross_stats:{total_return:gross},
    source_account:{model:BANDY_ACCOUNT_MODEL,book_performance_reproduced:false,basis:'Arithmetic-only normalized fractional cash fixture.',
      account_contribution_basis:'Dimensionless contributions and actual pre-entry equity fractions.',rejected_entries:0,
      sizing:{target_equity_fraction:target,origin:macd?'source_rule':'declared_research_convention',cash_reserved:true,fractional_positions:true},
      terminal_entry_pending:false,terminal_exit_pending:false,pending_order:null,days,trade_accounts}};
}
