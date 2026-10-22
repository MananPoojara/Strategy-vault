/** Small synthetic adapter projection from frozen independent0c7 references.
 * It is not a real SPX/NIFTY result and public admission must reject it.
 * Unit reference SHA 611f4c55a7e6452ee8ae12f93ac6dcc89404e3002a2b0a3aefe3ab7ec41a5a2b; stat reference SHA a70c91160e4d44788ae6915b001609226b56f773f7c3a4c54fdce5e46cf4e3c7. */
import type {UnitExposureAdmission} from "../../src/services/unit-exposure-report";
const BODY = {
  "id": "hbqts_dema16_8",
  "book": "hbqts",
  "ticker": "TEST_ONLY_SYNTHETIC",
  "dates": [
    "2024-01-02",
    "2024-01-03",
    "2024-01-04"
  ],
  "close": [
    100.0,
    99.0,
    99.099
  ],
  "equity": [
    1.0,
    0.9995,
    1.0
  ],
  "gross_equity": [
    1.0,
    1.0,
    1.001
  ],
  "bh": [
    1.0,
    0.99,
    0.99099
  ],
  "ohlc": {
    "d": [
      "2024-01-02",
      "2024-01-03",
      "2024-01-04"
    ],
    "o": [
      100.0,
      99.0,
      99.099
    ],
    "h": [
      100.0,
      99.0,
      99.099
    ],
    "l": [
      100.0,
      99.0,
      99.099
    ],
    "c": [
      100.0,
      99.0,
      99.099
    ],
    "v": [
      null,
      null,
      null
    ]
  },
  "trades": [
    {
      "dir": 1,
      "legs": [
        [
          1,
          99.0,
          1
        ]
      ],
      "entry": "2024-01-03",
      "exit": "2024-01-04",
      "exit_i": 2,
      "exit_px": 99.099,
      "avg_px": 99.0,
      "ret": 0.0,
      "ret_full_precision": 0.0,
      "bars": 1,
      "open": false,
      "reason": "Synthetic source Sell",
      "entry_kind": "close",
      "signal": "2024-01-04",
      "at_entry": {},
      "at_exit": {},
      "exits": [],
      "accounting": {
        "symbol": "TEST_ONLY_SYNTHETIC",
        "signal_side": "Long",
        "pts": 0.09900000000000375,
        "days_held": 1,
        "gross_ret": 0.001,
        "net_ret": 0.0,
        "fee_ret": 0.0,
        "slippage_ret": 0.001,
        "cost_status": "modeled_round_trip_cost",
        "gross_pnl": null,
        "fees": null,
        "slippage": null,
        "taxes": null,
        "net_pnl": null,
        "mae": null,
        "mfe": null,
        "money_status": "No funded capital",
        "return_basis": "Audited unit returns",
        "pts_basis": "Exit minus entry",
        "excursion_basis": "No interior observations",
        "mae_ret_observed": null,
        "mfe_ret_observed": null,
        "excursion_bars": 0,
        "excursion_status": "unavailable"
      }
    }
  ],
  "unit_exposure_report": {
    "model": "canonical_decimal_unit_exposure_v2",
    "book": "hbqts",
    "id": "hbqts_dema16_8",
    "source_convention": "qts_dema16_8_first_close_index_unit_exposure_v2",
    "source_performance_reproduced": false,
    "native_amibroker_parity": false,
    "arithmetic": "pinned Rust Display canonical-decimal rational; binary64 nearest-even including subnormals; nonzero-to-zero underflow and overflow rejected",
    "initial_equity": 1,
    "open_exit_cost": "estimated_liquidation",
    "days": [
      {
        "source_index": 0,
        "date": "2024-01-02",
        "equity": 1.0,
        "gross_equity": 1.0,
        "benchmark_equity": 1.0,
        "net_daily_return": 0.0,
        "gross_daily_return": 0.0,
        "benchmark_daily_return": 0.0
      },
      {
        "source_index": 1,
        "date": "2024-01-03",
        "equity": 0.9995,
        "gross_equity": 1.0,
        "benchmark_equity": 0.99,
        "net_daily_return": -0.0005,
        "gross_daily_return": 0.0,
        "benchmark_daily_return": -0.01
      },
      {
        "source_index": 2,
        "date": "2024-01-04",
        "equity": 1.0,
        "gross_equity": 1.001,
        "benchmark_equity": 0.99099,
        "net_daily_return": 0.0005002501250625312,
        "gross_daily_return": 0.001,
        "benchmark_daily_return": 0.001
      }
    ],
    "trade_returns": [
      {
        "trade_index": 0,
        "entry_source_index": 1,
        "exit_source_index": 2,
        "open": false,
        "entry_allocation": 1.0,
        "gross_return": 0.001,
        "net_return": 0.0,
        "fee_return": 0.0,
        "slippage_return": 0.001,
        "gross_contribution": 0.001,
        "net_contribution": 0.0,
        "fee_contribution": 0.0,
        "slippage_contribution": 0.001
      }
    ],
    "basis": "Test-only independent synthetic reference projection; no actual index admission."
  },
  "model_initial_equity": 1,
  "stats": {
    "total_return": 0.0,
    "cagr": 0.0,
    "vol": 0.007939,
    "sharpe": 0.002646,
    "sortino": 0.004585,
    "max_dd": -0.0005,
    "max_dd_days": 1.0,
    "mar": 0.0,
    "exposure": 0.666667,
    "bh_total": -0.00901,
    "bh_cagr": -0.808508,
    "bh_max_dd": -0.01,
    "years": 0.005476,
    "trades": 1.0,
    "win_rate": 0.0,
    "avg_trade": 0.0,
    "median_trade": 0.0,
    "avg_win": null,
    "avg_loss": 0.0,
    "payoff": null,
    "profit_factor": null,
    "best": 0.0,
    "worst": 0.0,
    "t_stat": null,
    "max_consec_losses": 1.0,
    "avg_bars": 1.0,
    "avg_deployed": 1.0,
    "open_trade": null,
    "trades_per_year": 182.625,
    "net_profit": 0.0,
    "closed_return_sum": 0.0,
    "compounded_return": 0.0,
    "car_mdd": 0.0,
    "expectancy_ratio": null,
    "avg_hold_days": 1.0,
    "monthly_thickness": 0.0,
    "gross_total_return": 0.001,
    "gross_avg_trade": 0.001,
    "gross_win_rate": 1.0,
    "gross_sharpe": 9.165151,
    "gross_max_dd": 0.0,
    "avg_net_pnl": 0.0,
    "ledger_gross_return": 0.001,
    "fee_return": 0.0,
    "slippage_return": 0.001
  },
  "gross_stats": {
    "total_return": 0.001,
    "cagr": 0.200255,
    "vol": 0.009165,
    "sharpe": 9.165151,
    "sortino": null,
    "max_dd": 0.0,
    "max_dd_days": 0.0,
    "mar": null,
    "exposure": 0.666667,
    "bh_total": -0.00901,
    "bh_cagr": -0.808508,
    "bh_max_dd": -0.01,
    "years": 0.005476,
    "trades": 1.0,
    "win_rate": 1.0,
    "avg_trade": 0.001,
    "median_trade": 0.001,
    "avg_win": 0.001,
    "avg_loss": null,
    "payoff": null,
    "profit_factor": null,
    "best": 0.001,
    "worst": 0.001,
    "t_stat": null,
    "max_consec_losses": 0.0,
    "avg_bars": 1.0,
    "avg_deployed": 1.0,
    "open_trade": null,
    "trades_per_year": 182.625,
    "net_profit": 0.001,
    "closed_return_sum": 0.001,
    "compounded_return": 0.001,
    "car_mdd": null,
    "expectancy_ratio": null,
    "avg_hold_days": 1.0,
    "monthly_thickness": 0.03
  },
  "benchmark_stats": {
    "total_return": -0.00901,
    "cagr": -0.808508,
    "sharpe": -7.829259,
    "max_dd": -0.01,
    "vol": 0.096561,
    "sortino": -8.248636,
    "mar": -80.850792
  },
  "benchmark_symbol": "TEST_ONLY_SYNTHETIC",
  "costs": {
    "fees_bps": 0,
    "slippage_bps": 5,
    "rollover_bps": 0,
    "total_side_bps": 5,
    "taxes": null,
    "rolls": null,
    "basis": "Two-side entry-allocation deductions",
    "rollover_status": "Unsupported",
    "slippage_status": "Modelled per side",
    "money_status": "No funded capital",
    "open_trade_cost_basis": "Estimated liquidation"
  },
  "accounting_basis": "Test-only independent synthetic reference projection; no actual index admission.",
  "provenance": null
};
const ADMISSION = {
  "id": "hbqts_dema16_8",
  "book": "hbqts",
  "ticker": "TEST_ONLY_SYNTHETIC",
  "region": "test_only",
  "definition": {
    "id": "hbqts_dema16_8",
    "book": "hbqts",
    "instrument_policy": {
      "requirement": "index",
      "allowed_types": [
        "index"
      ],
      "preferred_basket": null,
      "execution_scope": "single_symbol_index_timing_research",
      "source_basis": "Test-only independent synthetic reference projection; no actual index admission.",
      "source_convention": "qts_dema16_8_first_close_index_unit_exposure_v2",
      "market_adaptation": true,
      "source_performance_reproduced": false,
      "source_rule_reviewed": true,
      "independent_selected_ledger_checked": true,
      "source_price_unit": null,
      "native_india_currency_admitted": null,
      "native_currency_constraint": null
    }
  },
  "parameters": {
    "fast": 16,
    "slow": 8,
    "hold": 1
  },
  "instrument": {
    "classification_basis": "independent_synthetic_reference",
    "metadata": {
      "ticker": "TEST_ONLY_SYNTHETIC",
      "first": "2024-01-02",
      "last": "2024-01-04"
    }
  },
  "costs": {
    "fees_bps": 0,
    "slippage_bps": 5,
    "rollover_bps": 0
  }
};
export const UNIT_FIXTURE_BINDING = {"scope": "independent_reference_projection_test_only", "case_id": "SYN_exact_zero_cost_trade_terminal_first_exit_f0_s5", "reference_sha256": "611f4c55a7e6452ee8ae12f93ac6dcc89404e3002a2b0a3aefe3ab7ec41a5a2b"};
export const unitExposureFixture = () => ({data: structuredClone(BODY), admission: structuredClone(ADMISSION) as UnitExposureAdmission});
