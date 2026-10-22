//! Exact-ID unit-report serialization; ordinary legacy detail remains unchanged.
use crate::{accounting::Costs, connors::Ctx, unit_exposure_report as unit, Run};
fn list(v: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", v.into_iter().collect::<Vec<_>>().join(","))
}
/// Existing completed-close diagnostic shape, with source close retained at full
/// precision. Generic RSI/SMA context is not the DEMA source-rule calculation.
fn context(ctx: &Ctx, i: usize) -> String {
    let n = unit::number;
    let pct = |a: f64, b: f64| n((a / b - 1.) * 100.);
    format!("{{\"date\":\"{}\",\"close\":{},\"rsi2\":{},\"vs_sma200\":{},\"vs_sma5\":{},\"vix_rsi2\":{},\"rsi_p\":null,\"rsi_period\":null,\"rsi_kind\":null,\"vs_ma_p\":null,\"ma_period\":null,\"ma_kind\":null,\"vs_trend_p\":null,\"trend_period\":null,\"trend_kind\":null,\"basis\":\"Completed daily close indicator context; not a source-rule or execution-price verification\"}}",ctx.r[i].date,n(ctx.c[i]),n(ctx.rsi2[i]),pct(ctx.c[i],ctx.ma200[i]),pct(ctx.c[i],ctx.ma5[i]),n(ctx.vix.rsi2.get(i).copied().unwrap_or(f64::NAN)))
}
pub fn render(ctx: &Ctx, run: &Run, costs: Costs) -> Result<String, String> {
    let report = run
        .unit_exposure_report
        .as_ref()
        .ok_or("unit_exposure_report_shape: mandatory reviewed unit report absent")?;
    let n = unit::number;
    let (from, to) = (run.from, run.to);
    let dates = list((from..=to).map(|i| format!("\"{}\"", ctx.r[i].date)));
    let mut trades = Vec::new();
    for (i, t) in run.trades.iter().enumerate() {
        let a = &report.trade_returns[i];
        let entry = t.legs[0].i;
        trades.push(format!("{{\"dir\":1,\"legs\":[[{},{},1]],\"entry\":\"{}\",\"exit\":\"{}\",\"exit_i\":{},\"exit_px\":{},\"avg_px\":{},\"ret\":{},\"ret_full_precision\":{},\"bars\":{},\"open\":{},\"reason\":\"{}\",\"entry_kind\":\"close\",\"at_entry\":{},\"at_exit\":{},\"signal\":\"{}\",\"exits\":[],\"accounting\":{}}}",entry-from,n(t.legs[0].px),ctx.r[entry].date,ctx.r[t.exit_i].date,t.exit_i-from,n(t.exit_px),n(t.legs[0].px),n(a.net_return),n(a.net_return),t.exit_i-entry,t.open,crate::esc(&t.reason),context(ctx,entry),context(ctx,t.signal_i),ctx.r[t.signal_i].date,report.trade_accounting_json(ctx,t,i)?));
    }
    let ohlc = format!(
        "{{\"d\":{},\"o\":{},\"h\":{},\"l\":{},\"c\":{},\"v\":{}}}",
        dates,
        list((from..=to).map(|i| n(ctx.o[i]))),
        list((from..=to).map(|i| n(ctx.h[i]))),
        list((from..=to).map(|i| n(ctx.l[i]))),
        list((from..=to).map(|i| n(ctx.c[i]))),
        list((from..=to).map(|i| ctx.r[i].volume.map_or("null".into(), n)))
    );
    let stats = report.financial_stats(ctx, &run.trades, from, to);
    let gross = report.common_stats(ctx, &run.trades, from, to, unit::View::Gross);
    let benchmark = report.benchmark_stats(ctx, from, to);
    Ok(format!("{{\"id\":\"{}\",\"book\":\"{}\",\"ticker\":\"{}\",\"dates\":{},\"close\":{},\"equity\":{},\"gross_equity\":{},\"bh\":{},\"trades\":{},\"ohlc\":{},\"unit_exposure_report\":{},\"model_initial_equity\":1,\"stats\":{},\"gross_stats\":{},\"benchmark_symbol\":\"{}\",\"benchmark_stats\":{},\"costs\":{},\"accounting_basis\":\"{}\"}}",unit::ID,unit::BOOK,crate::esc(&ctx.r[from].ticker),dates,list((from..=to).map(|i|n(ctx.c[i]))),list(report.days.iter().map(|d|n(d.equity))),list(report.days.iter().map(|d|n(d.gross_equity))),list(report.days.iter().map(|d|n(d.benchmark_equity))),list(trades),ohlc,report.json(),unit::stats_json(&stats),unit::stats_json(&gross),crate::esc(&ctx.r[from].ticker),unit::stats_json(&benchmark),unit::costs_json(costs),crate::esc(unit::BASIS)))
}
