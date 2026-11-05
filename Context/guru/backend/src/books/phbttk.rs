//! Price Headley, "Big Trends Toolkit 2.0 for MetaStock - User's Manual" (Equis / BigTrends, 2008, 75 pp.).
//! Book id `phbttk`. Source copy: Books/Price Headley/BTTK_2 0.pdf (from R:/USB HDD 1/MetaStock/@Systems & Indicators/
//! Price Headleys Big Trends Toolkit 2.0 (bigtrends.com)/Price Headleys Big Trend v.2.0 for Metastock/BTTK_2 0.pdf).
//! The add-on's MetaStock formulas are compiled into Bigtrendv2setup.exe and are not printed, so only rules the manual
//! states completely are Live: the Percent R confirmation system (p.36-38). Percent R = 100 x (Close - lowest Low of
//! 30 bars) / (highest High of 30 bars - lowest Low of 30 bars) (Williams' %R on Headley's 0-100 scale, 80 = near the
//! high). "Crosses over 80" = MetaStock Cross: above 80 today, at or below 80 yesterday.
//! Lane K30 (3 Nov 2026): the add-on's MetaStock System Tester records were read from the installer (MS10PRFT.DTA) and now run
//! as shipped (section "K30" below); they replace the manual-only holds of the other systems.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/headley/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["phbttk_pctr_long", "phbttk_pctr_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "phbttk";
const G: &str = "Big Trends Toolkit 2.0";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, dir, status, rules, notes| StrategyDef {
        id, name, group: G, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let opt = "the manual adds exit \"(2) on the close 2 weeks before the expiration day (usually close by the first Friday of the expiration month)\" for the option trade";
    vec![
        mk("phbttk_pctr_long", "Percent R (30) bull confirmation system", "Long", Live, &[
            "E|\"A Bullish Setup occurs when the 30-bar Percent R crosses over the 80% threshold. It is 'Bull Confirmed' when the security closes above the high of the Setup bar.\" (p.36) - buy on that close.",
            "F|\"If we never get the confirmation, then I turn off the setup if Percent R comes back to read 79% or lower\" (p.36)",
            "X|\"If we do get confirmation, then the next reading of 79% or lower will set my next closing stop, where an exit will then occur on any close under the low of the bar when Percent R crossed back under 80% to 79% or lower.\" (p.36); \"exit this position (1) when the close is less than the low on the bar Percent R last crossed back under 80 to give a reading of 79.00 or lower\" (p.37). Until the first such reading after entry there is no stop.",
            "F|[Book silent] Signals and fills at the close (MetaStock system test default). A new setup bar (a fresh cross over 80) replaces an unconfirmed one. A 'cross back under 80 to 79 or lower' = %R <= 79 today and > 79 yesterday. One position at a time.",
            "F|Exit \"(2) on the close 2 weeks before the expiration day\" (p.37) belongs to the call-option trade (\"buying a call option two months before expiration\"); a stock/index position has no expiration day, so the stock rule of p.36 is coded. The option trade is phbttk_options (not testable).",
        ], "Book: US stocks (RIMM example), no statistics printed. Run on the indexes (instrument swap)."),
        mk("phbttk_pctr_short", "Percent R (30) bear confirmation system", "Short", Live, &[
            "E|\"A bearish Setup occurs when the 30-bar Percent R crosses under the 20% threshold. It is Bear Confirmed when the security closes below the low of the Setup bar.\" (p.37) - sell short on that close.",
            "F|\"If we never get the confirmation, then I turn off the setup if Percent R comes back to read 21% or higher\" (p.37)",
            "X|\"the next reading of 21% or higher will set my next closing stop, where an exit will then occur on any close over the high of the bar when Percent R crossed back over 20% to 21% or [higher]\" (p.37-38; printed 'or lower', a misprint: 'exit this position (1) when the close is greater than the high on the bar Percent R last crossed back over 20 to give a reading of 21.00 or higher', p.38)",
            "F|[Book silent] As the long side: fills at the close; cross under 20 = below 20 today, at or above 20 yesterday; cross back over 20 to 21 or higher = %R >= 21 today and < 21 yesterday.",
        ], "Book: US stocks, no statistics printed. Run on the indexes (instrument swap)."),
        mk("phbttk_options", "Option trades and the expiration-day exit (all BTTK systems)", "Long/Short", NotTestable("rules not fully defined: the expiry month is only given as a range (\"out 3-4 months\", \"2nd expiration month\", \"two months before expiration\"; Big Trends Toolkit 2.0 p.11); real option prices are held for SPY/QQQ/IWM, but there is no single rule to run"),
            &["X|\"(2) on the close 2 weeks before the expiration day (usually close by the first Friday of the expiration month)\" (p.11, 13, 19, 21, 25, 26, 29-31, 35, 37-38, 46, 48-49)"], opt),
        mk("phbttk_nasdaq_rs", "NASDAQ vs S&P relative strength market timing", "Long/Short", NotTestable("rules not fully defined: no confirmation window (\"closes in a future day above the NASDAQ Composite high on the day of the bullish RS crossover\") and no exit rule (Big Trends Toolkit 2.0 p.6); it would also need the NASDAQ Composite (the vault has NDX, not COMP)"),
            &["E|\"When the 10-day simple moving average crosses above the 21-day simple moving average of the relative strength (RS) line, this creates the bullish Setup ... It is ONLY confirmed when the NASDAQ Composite CLOSES in a future day above the NASDAQ Composite high on the day of the bullish RS crossover.\" (p.6)"], "Seen (p.5-8)."),
        // ---- K30 (3 Nov 2026): the add-on's System Tester records (MS10PRFT.DTA in Bigtrendv2setup.exe, read by batch X;
        // authors/batch-x/keep/2617/PRFT_slots.txt) supply the formulas the manual leaves out; they run as shipped.
        mk("phbttk_bb80", "BTTK 80 Day Bollinger Bands (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "top:=BBandTop(C,80,S,2); el:=Cross(C,ValueWhen(1,Cross(C,top),H)) AND C>top; xl:=Cross(ValueWhen(1,Cross(MDI(14),PDI(14)),L),C); lt:=If(el,1,If(xl,0,PREV)); Cross(lt,0.5)""##,
            r##"X|Sell (slot 3): "cross(valuewhen(1,cross(mdi(14),pdi(14)),l),c)""##,
            r##"E|Sell Short (slot 5): "bot:=bbandbot(c,80,s,2); cross(valuewhen(1,cross(bot,c),l),c) AND C<bot""##,
            r##"X|Buy to Cover (slot 8): "cross(c,valuewhen(1,cross(pdi(14),mdi(14)),h))""##,
            r##"F|Manual p.47-48 (words): "a first close above the upper 80-bar Bollinger Band, followed by a close above that bar's high"; the shipped record gives the 2-SD width and the DMI-cross exit"##,
        ], BB80),
        mk("phbttk_accel_bands", "BTTK Acceleration Bands (20) (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "top:=Mov(H*(1+2*(((H-L)/((H+L)/2))*1000)*.001),20,S); el:=Sum(C>top,2)=2; xl:=C<Mov(C,10,E); lt:=If(el,1,If(xl,0,PREV)); Cross(lt,0.5)""##,
            r##"X|Sell (slot 3): "c<mov(c,10,e)""##,
            r##"E|Sell Short (slot 5): "bot:=Mov(L*(1-2*(((H-L)/((H+L)/2))*1000)*.001),20,S); es:=Sum(C<bot,2)=2; xs:=C>Mov(C,10,E); st:=If(es,-1,If(xs,0,PREV)); Cross(-0.5,st)""##,
            r##"X|Buy to Cover (slot 8): "c>mov(c,10,e)""##,
            r##"F|Manual p.16 (words): two closes above the upper 20-bar band buy. The 80-bar version of the manual (p.19) has no shipped record and is not coded. The band formula is the one of phbt_accel_daily (Big Trends in Trading), with a 10-day EMA exit instead of the close back inside the band"##,
        ], ACCEL),
        mk("phbttk_accel_bb", "BTTK Acceleration Bands + Bollinger Bands (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "a20:=Mov(H*(1+2*(((H-L)/((H+L)/2))*1000)*.001),20,S); a80:=Mov(H*(1+2*(((H-L)/((H+L)/2))*1000)*.001),80,S); b20:=BBandTop(C,20,S,2); bc:=Sum(C>a20,2)=2 AND Sum(C>a80,2)=2 AND b20>a20 AND C>b20; sc:=Cross(ValueWhen(1,Cross(a20,b20),L),C) AND BarsSince(Cross(a20,b20))<=20; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(trade=0,0.5)""##,
            r##"E|Sell Short (slot 5): "a20:=Mov(L*(1-2*(((H-L)/((H+L)/2))*1000)*.001),20,S); a80:=...,80,S); b20:=BBandBot(C,20,S,2); bc:=Sum(C<a20,2)=2 AND Sum(C<a80,2)=2 AND b20<a20 AND C<b20; sc:=Cross(C,ValueWhen(1,Cross(b20,a20),H)) AND BarsSince(Cross(b20,a20))<=20; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Buy to Cover (slot 8): the same lines ending "Cross(trade=0,0.5)""##,
        ], ACCELBB),
        mk("phbttk_dmi_diff", "BTTK DMI Difference (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "x:=Mov(PDI(14)-MDI(14),10,S); bset:=Cross(x,Mov(x,10,S)); el:=Cross(C,ValueWhen(1,bset,H)) AND BarsSince(bset)<5; xl:=ADX(14)>PDI(14); lt:=If(el,1, If(xl,0,PREV)); Cross(lt,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(0.5,lt)""##,
            r##"E|Sell Short (slot 5): "x:=Mov(PDI(14)-MDI(14),10,S); sset:=Cross(Mov(x,10,S),x); es:=Cross(ValueWhen(1,sset,L),C) AND BarsSince(sset)<5; xs:=ADX(14)>MDI(14); st:=If(es,-1,If(xs,0,PREV)); Cross(-0.5,st)""##,
            r##"X|Buy to Cover (slot 8): the same lines ending "Cross(st,-0.5)""##,
            r##"F|Manual p.9-11 (words) gives no entry trigger; the shipped record does"##,
        ], DMID),
        mk("phbttk_efficiency", "BTTK Efficiency Ratio +30/-30 (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "ef:=(ROC(C,20,$)/Sum(Abs(ROC(C,1,$)),20))*100; el:=Cross(C,ValueWhen(1,Cross(ef,30),H)) AND BarsSince(Cross(ef,30))<5; xl:=Cross(ValueWhen(1,Cross(30,ef),L),C); lt:=If(el,1, If(xl,0,PREV)); Cross(lt,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(0.5,lt)""##,
            r##"E|Sell Short (slot 5): "es:=Cross(ValueWhen(1,Cross(-30,ef),L),C) AND BarsSince(Cross(-30,ef))<5; xs:=Cross(C,ValueWhen(1,Cross(ef,-30),H)); st:=If(es,-1, If(xs,0,PREV)); Cross(-0.5,st)""##,
            r##"X|Buy to Cover (slot 8): the same lines ending "Cross(st,-0.5)""##,
            r##"F|Manual p.23 (words): period and confirmation window not given; the shipped record gives 20 bars and a 5-bar window"##,
        ], EFF),
        StrategyDef { needs_volume: true, ..mk("phbttk_force_lt", "BTTK Force Index Long Term Trend 60/120 (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "ma1:=Mov(ROC(C,1,$)*V,60,S); ma2:=Mov(ROC(C,1,$)*V,120,S); bset:=Cross(ma1,ma2) AND Min(ma1,ma2)>0; sset:=Cross(ma2,ma1); setup:=If(bset,1, If(sset,-1,PREV)); bc:=Cross(C,ValueWhen(1,bset,H)) AND BarsSince(bset)<=20 AND setup=1; sc:=Cross(ValueWhen(1,sset,L),C) AND setup=-1; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(trade=0,0.5)""##,
            r##"E|Sell Short (slot 5): "bset:=Cross(ma1,ma2); sset:=Cross(ma2,ma1) AND Max(ma1,ma2)<0; setup:=If(bset,1, If(sset,-1,PREV)); xs:=Cross(C,ValueWhen(1,bset,H)) AND setup=1; es:=Cross(ValueWhen(1,sset,L),C) AND BarsSince(sset)<=20 AND setup=-1; trade:=If(es,1,If(xs,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Buy to Cover (slot 8): the same lines ending "Cross(trade=0,0.5)""##,
        ], FORCE) },
        StrategyDef { needs_volume: true, ..mk("phbttk_force_st", "BTTK Force Index Short Term Trend 14/28 (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): as the 60/120 record with "ma1:=Mov(ROC(C,1,$)*V,14,S); ma2:=Mov(ROC(C,1,$)*V,28,S);""##,
            r##"X|Sell, Sell Short, Buy to Cover (slots 3, 5, 8): as the 60/120 record with the 14/28 averages"##,
        ], FORCE) },
        mk("phbttk_mom_div", "BTTK Momentum Divergence (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "md:=((MACD()-LLV(MACD(),40))/(HHV(MACD(),40)-LLV(MACD(),40)))*100; cd:=((C-LLV(C,40))/(HHV(C,40)-LLV(C,40)))*100; bc:=Min(md,cd)=100; sc:=Max(md,cd)<90; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(trade=0,0.5)""##,
            r##"E|Sell Short (slot 5): "bc:=Max(md,cd)=0; sc:=Min(md,cd)>10; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Buy to Cover (slot 8): the same lines ending "Cross(trade=0,0.5)""##,
        ], MOMD),
        StrategyDef { needs_volume: true, ..mk("phbttk_options_shark", "BTTK Options Shark, run on the underlying (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "ef:=(ROC(C,20,$)/Sum(Abs(ROC(C,1,$)),20))*100; actop:=Mov(H*(1+2*(((H-L)/(H+L)/2)*1000)*.001),20,S); bbtop:=bbandtop(c,80,s,2); vcon:=v> ref(mov(v,20,s),-1); con1:= cross(c>max(actop,bbtop),0.5); cross(c,valuewhen(1,con1,h)) AND barssince(con1)<=5 AND ef>30 AND (vcon or valuewhen(1,con1,vcon)) AND roc(adx(14),1,$)>0""##,
            r##"X|Sell (slot 3): "econ1:=cross(bbtop,l); econ2:=cross(30,ef); cross(valuewhen(1,econ1,l),c) OR cross(valuewhen(1,econ2,l),c) OR ref(OptionExp(),-1) =1""##,
            r##"E|Sell Short (slot 5): "acbot:=Mov(L*(1-2*(((H-L)/(H+L)/2)*1000)*.001),20,S); bbbot:=bbandbot(c,80,s,2); con1:= cross(c<min(acbot,bbbot),0.5); cross(c,valuewhen(1,con1,l)) AND barssince(con1)<=5 AND ef<-30 AND (vcon or valuewhen(1,con1,vcon)) AND roc(adx(14),1,$)>0""##,
            r##"X|Buy to Cover (slot 8): "econ1:=cross(h,bbbot); econ2:=cross(ef,-30); cross(c,valuewhen(1,econ1,h)) OR cross(c,valuewhen(1,econ2,h)) OR ref(OptionExp(),-1) =1""##,
            r##"F|Record note: "designed to identify options to buy ... this test should be run on the underlying security""##,
        ], SHARK) },
        mk("phbttk_pct_above_ma", "BTTK Percentage Above 200 Day MA (shipped system test)", "Long", Live, &[
            r##"E|Buy (slot 0): "x:=((C-Mov(C,200,S))/Mov(C,200,S))*100; xbp:=If(x>100 AND Ref(x<100,-1),H, PREV ); Cross(C, xbp ) AND Alert(x>100,11)""##,
            r##"X|Sell (slot 3): "x:=((C-Mov(C,200,S))/Mov(C,200,S))*100; xsp:= If(x>100 AND Ref(x<100,-1),L, PREV ); Cross(xsp,C)""##,
            r##"F|Manual p.41 (words) gives no cancellation; the shipped record limits the buy to 11 bars after the percentage was last above 100 (Alert). The 400-day record is phbttk_pct_above_ma400"##,
        ], PCT200),
        mk("phbttk_pct_above_ma400", "BTTK Percentage Above 400 Day MA (shipped system test, US only)", "Long", Live, &[
            r##"E|Buy (slot 0): "x:=((C-Mov(C,400,S))/Mov(C,400,S))*100; xbp:=If(x>100 AND Ref(x<100,-1),H, PREV ); bc:=Cross(C, xbp ) AND Alert(x>100,11); sc:=Ref(OptionExp(),-1) =1; trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=1,0.5)""##,
            r##"X|Sell (slot 3): the same lines ending "Cross(trade=0,0.5)" (out on the day after the monthly option expiry)"##,
        ], PCT400),
        mk("phbttk_stoch_conf", "BTTK Stochastics Confirmation (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "x:=cross(Stoch(21,10),81); barssince(x)<=5 AND cross(c,valuewhen(1,x,H))""##,
            r##"X|Sell (slot 3): "cross(valuewhen(1,cross(79,Stoch(21,10)),L),c)""##,
            r##"E|Sell Short (slot 5): "x:=cross(19,Stoch(21,10)); barssince(x)<=5 AND cross(valuewhen(1,x,L),c)""##,
            r##"X|Buy to Cover (slot 8): "cross(c,valuewhen(1,cross(Stoch(21,10),21),H))""##,
            r##"F|Manual p.44 (words): "%K setting of 21 periods with exponential smoothing"; the shipped record calls Stoch(21,10) (simple %K slowing of 10)"##,
        ], STOCH),
        mk("phbttk_pctr_retest", "BTTK Williams %R Retest System (shipped system test)", "Long/Short", Live, &[
            r##"E|Buy (slot 0): "p1:=WillR(30)+100; x:=Cross(p1,80); bc:=BarsSince(x)<=5 AND Cross(C,ValueWhen(1,x,H)); sc:=Cross(ValueWhen(1,Cross(79.5,p1),L),C); trade:=If(bc,1,If(sc,0,PREV)); br:=Ref(trade=1 AND BarsSince(bc)<30,-1) AND Cross(79.5,p1); trade1:=If(br,1,If(sc,0,PREV)); Cross(trade1=1,0.5)""##,
            r##"X|Sell (slot 3): "p1:=WillR(30)+100; x:=Cross(p1,80); bc:=BarsSince(x)<=5 AND Cross(C,ValueWhen(1,x,H)); sc:=Cross(ValueWhen(1,Cross(79,p1),L),C); trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=0,0.5)""##,
            r##"E|Sell Short (slot 5): "x:=Cross(20,p1); bc:=BarsSince(x)<=5 AND Cross(ValueWhen(1,x,L),C); sc:=Cross(C,ValueWhen(1,Cross(p1,21),H)); trade:=If(bc,1,If(sc,0,PREV)); br:=Ref(trade=1 AND BarsSince(bc)<30,-1) AND Cross(p1,20.5); trade1:=If(br,1,If(sc,0,PREV)); Cross(trade1=1,0.5)""##,
            r##"X|Buy to Cover (slot 8): "x:=Cross(20,p1); bc:=...; sc:=Cross(C,ValueWhen(1,Cross(p1,21),H)); trade:=If(bc,1,If(sc,0,PREV)); Cross(trade=0,0.5)""##,
            r##"F|Manual p.38 (words) contradicts itself; the shipped record is used. Each order formula is evaluated as stored (the Buy uses 79.5, the Sell 79)"##,
        ], RETEST),
        mk("phbttk_other", "NAZRS template, 2x 200-day line", "Long/Short", NotTestable("rules not fully defined: listed as add-on tools and templates with no rules in the manual (Big Trends Toolkit 2.0 p.1-2, p.5-8)"),
            &["F|BTTK NAZRS System template, BTTK Line at Cross of 2 Times 200 Day MA (p.1-2); the Options Shark system test is phbttk_options_shark"], "Seen; no rules printed."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "phbttk_pctr_long" => Some(("phbttk_pctr", "Headley Percent R confirmation", "bull")),
        "phbttk_pctr_short" => Some(("phbttk_pctr", "Headley Percent R confirmation", "bear")),
        "phbttk_pct_above_ma" => Some(("phbttk_pct_above_ma", "Headley Percentage above the 200 / 400-day MA", "200-day")),
        "phbttk_pct_above_ma400" => Some(("phbttk_pct_above_ma", "Headley Percentage above the 200 / 400-day MA", "400-day (US only)")),
        "phbttk_bb80" => Some(("phbttk_bb80", "Headley 80-day Bollinger Band breakout (BTTK)", "as shipped")),
        "phbttk_accel_bands" => Some(("phbttk_accel_bands", "Headley Acceleration Bands 20 (BTTK)", "as shipped")),
        "phbttk_accel_bb" => Some(("phbttk_accel_bb", "Headley Acceleration Bands + Bollinger Bands (BTTK)", "as shipped")),
        "phbttk_dmi_diff" => Some(("phbttk_dmi_diff", "Headley DMI Difference (BTTK)", "as shipped")),
        "phbttk_efficiency" => Some(("phbttk_efficiency", "Headley Efficiency Ratio +30/-30 (BTTK)", "as shipped")),
        "phbttk_force_lt" => Some(("phbttk_force_lt", "Headley Force Index long-term trend (BTTK)", "60/120")),
        "phbttk_force_st" => Some(("phbttk_force_st", "Headley Force Index short-term trend (BTTK)", "14/28")),
        "phbttk_mom_div" => Some(("phbttk_mom_div", "Headley Momentum Divergence (BTTK)", "as shipped")),
        "phbttk_options_shark" => Some(("phbttk_options_shark", "Headley Options Shark on the underlying (BTTK)", "US only")),
        "phbttk_stoch_conf" => Some(("phbttk_stoch_conf", "Headley Stochastics Confirmation (BTTK)", "as shipped")),
        "phbttk_pctr_retest" => Some(("phbttk_pctr_retest", "Headley Williams %R Retest (BTTK)", "as shipped")),
        _ if id.starts_with("phbttk_") => Some(("phbttk_other", "Big Trends Toolkit 2.0 - other systems (not testable)", "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if reviewed_k30(id) {
        return Some(if LONG_ONLY.contains(&id) { vec![] } else { vec![p("side", "Sides traded: 0 = both as shipped, 1 = long side only, -1 = short side only", 0., -1., 1., 1.), msws::REVERSE_PARAM] });
    }
    let long = match id { "phbttk_pctr_long" => true, "phbttk_pctr_short" => false, _ => return None };
    Some(vec![
        p("len", "Percent R bars", 30., 5., 60., 1.),
        if long { p("lvl", "Setup: %R crosses over", 80., 50., 95., 1.) } else { p("lvl", "Setup: %R crosses under", 20., 5., 50., 1.) },
        if long { p("off", "Turn-off / stop reading at or below", 79., 45., 94., 1.) } else { p("off", "Turn-off / stop reading at or above", 21., 6., 55., 1.) },
    ])
}

pub fn pct_r(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|t| {
        if t + 1 < n { return f64::NAN }
        let hh = h[t + 1 - n..=t].iter().cloned().fold(f64::MIN, f64::max);
        let ll = l[t + 1 - n..=t].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { 100. * (c[t] - ll) / (hh - ll) } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if reviewed_k30(id) { return run_k30(id, x, from, to, p); }
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let d = if id == "phbttk_pctr_long" { 1.0 } else { -1.0 };
    let to = to.min(x.c.len().saturating_sub(1));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let r = pct_r(h, l, c, g("len").max(2.) as usize);
    let (lvl, off) = (g("lvl"), g("off"));
    // in the long frame: setup = cross over lvl, off-event = reading <= off after > off
    let a = |v: f64| d * v;
    let mut out = Vec::new();
    let mut setup: Option<usize> = None;
    let mut pos: Option<(usize, f64)> = None; // entry bar, stop level (NaN = none yet)
    for t in from.max(1)..=to {
        if !r[t].is_finite() || !r[t - 1].is_finite() { continue }
        let cross = a(r[t]) > a(lvl) && a(r[t - 1]) <= a(lvl);
        let back = a(r[t]) <= a(off) && a(r[t - 1]) > a(off);
        if let Some((i, mut stop)) = pos {
            if back { stop = if d > 0. { l[t] } else { h[t] } }
            if stop.is_finite() && a(c[t]) < a(stop) {
                out.push(Trade { dir: d, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: t, exit_px: c[t], reason: "closing stop".into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                pos = None;
            } else { pos = Some((i, stop)); }
            continue;
        }
        if cross { setup = Some(t); continue }
        if let Some(s) = setup {
            if a(c[t]) > a(if d > 0. { h[s] } else { l[s] }) { pos = Some((t, f64::NAN)); setup = None; }
            else if a(r[t]) <= a(off) { setup = None; }
        }
    }
    if let Some((i, _)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

// ===================================================================== K30: the shipped System Tester records
// Lane K30 (titles loop, 3 Nov 2026), to-code rows 372-384 (lane X1, lead decision 181). The add-on installer's MS10PRFT.DTA
// holds the systems as MetaStock formula text (slot 0 Buy, 3 Sell, 5 Sell Short, 8 Buy to Cover; read by batch X into
// authors/batch-x/keep/2617/PRFT_slots.txt). They use ValueWhen, PREV, Alert and OptionExp, which the shared msws interpreter
// lacks, so each record is written out below; the fills are the msws engine's (msws::simulate_conv, reverse by default since K32).
// Independent check authors/titles-1030/K30w/k30_ref.py.
use crate::books::msws;

pub const REVIEWED_K30: &[&str] = &["phbttk_accel_bands", "phbttk_accel_bb", "phbttk_bb80", "phbttk_dmi_diff", "phbttk_efficiency", "phbttk_force_lt",
    "phbttk_force_st", "phbttk_mom_div", "phbttk_options_shark", "phbttk_pct_above_ma", "phbttk_pct_above_ma400", "phbttk_pctr_retest", "phbttk_stoch_conf"];
pub fn reviewed_k30(id: &str) -> bool { REVIEWED_K30.contains(&id) }
/// OptionExp() = the US monthly equity option expiry (lead decision 181): US instruments only.
pub const US_ONLY: &[&str] = &["phbttk_options_shark", "phbttk_pct_above_ma400"];
const LONG_ONLY: &[&str] = &["phbttk_pct_above_ma", "phbttk_pct_above_ma400"];

macro_rules! conv { () => { " [Declared readings] The record's formulas run as stored (MS10PRFT.DTA, Bigtrendv2setup.exe). Fills: the vault's MetaStock System Tester convention (msws engine): signal and fill at the close; a position closes by its own Sell / Buy to Cover formula, from the bar after entry; MetaStock System Tester reverses on an opposite entry (7.2 Order of Evaluation; Pro 8.0 p.332): an opposite entry while a position is open closes it and opens the other side at that close, checked before the close formula; BTTK 2.0 is a MetaStock 10 file and versions 9-11 are not documented in the share; p.reverse = 0 = the earlier reading (an opposite entry is ignored while a position is open; an exit and an opposite entry on one bar reverse); no same-direction re-entry on the exit bar; one position, one unit; parameter side = 1 / -1 runs one side. A value that is not yet defined is never true and makes what uses it undefined (Cross needs both bars; ValueWhen and BarsSince are undefined before their condition first holds); PREV starts at 0 on the first bar where the rest of its formula is defined. Indicators as the msws engine: Mov S/E (the EMA seeded with its first value, shown from bar n), PDI/MDI/ADX Wilder 14, BBandTop/Bot(C,n,S,2) = SMA n +/- 2 population SD, MACD() = EMA(alpha .15) - EMA(alpha .075) of the close seeded on the first close (undefined for 25 bars), Stoch(k,s) = 100 Sum_s(C - LLV(L,k))/Sum_s(HHV(H,k) - LLV(L,k)), WillR(30) = -100 (HHV(H,30) - C)/(HHV(H,30) - LLV(L,30)). The manual prints no statistics for these systems; runs on daily indices, ETFs and stocks, US and India, are a declared application." } }
const BB80: &str = concat!("Buy when a close crosses above the high of the bar on which the close last crossed above the 80-bar 2-SD upper band (while still above the band); out when the close crosses under the low of the bar of the last -DI cross above +DI. Short: close crosses under the low of the last cross under the lower band; cover on a close above the high of the last +DI cross above -DI.", conv!());
const ACCEL: &str = concat!("Two closes above the upper 20-bar Acceleration Band buy (the state flips at the first such bar); a close under the 10-day EMA sells. Mirror short on two closes under the lower band, covered on a close above the 10-day EMA. Band = 20-bar SMA of H x (1 + 2 x (H-L)/((H+L)/2)), the formula of phbt_accel_daily.", conv!());
const ACCELBB: &str = concat!("Buy when two closes are above both the 20- and 80-bar upper Acceleration Bands and the close is above the 20-bar 2-SD upper Bollinger band, which is above the 20-bar Acceleration Band; out when, within 20 bars of the 20-bar Acceleration Band crossing back above the Bollinger band, the close crosses under that bar's low. Short mirrored on the lower bands.", conv!());
const DMID: &str = concat!("x = 10-bar SMA of (+DI - -DI); setup = x crosses its own 10-bar SMA; buy on a close crossing above the setup bar's high within 4 bars of it; out when ADX(14) rises above +DI. Short mirrored (ADX above -DI covers).", conv!());
const EFF: &str = concat!("Efficiency ratio = 100 x 20-bar change / sum of 20 absolute daily changes. Buy on a close crossing above the high of the bar where it crossed over +30, within 4 bars; out on a close crossing under the low of the bar where it last crossed back under +30. Short mirrored at -30.", conv!());
const FORCE: &str = concat!("Force Index = daily change x volume, averaged over the two lengths. Long set-up: the short average crosses above the long one with both above 0; buy when the close crosses above that bar's high within 20 bars while the set-up stands; out when the close crosses under the low of the bar of the next cross down. Short mirrored (both below 0). Needs the instrument's own volume (no cash index).", conv!());
const MOMD: &str = concat!("MACD and the close are each scaled 0-100 over 40 bars; long when both read 100, out when both are under 90; short when both read 0, covered when both are above 10.", conv!());
const SHARK: &str = concat!("Run on the underlying, as the record's note says (the record is meant for buying calls or puts). Buy when, within 5 bars of the close moving above both the 20-bar Acceleration Band (formula as stored: (H-L)/(H+L)/2) and the 80-bar 2-SD Bollinger band, the close crosses above that bar's high, with efficiency ratio > 30, volume above its prior 20-day average (today or on the set-up bar) and ADX rising; out on a close under the low of the last cross of the band down through the low, of the last efficiency cross under 30, or on the day after the monthly option expiry. Short side as stored: Cross(C, ValueWhen(1,con1,L)) is a close crossing ABOVE the set-up bar's low, and ADX rising is required on both sides. OptionExp() (lead decision 181) = 1 on the last trading bar on or before the third Friday of the month, within that week (a holiday Friday moves it to the Thursday), as agtsic_expiry; US instruments only (India's expiry calendar differs and is not substituted). Needs the instrument's own volume.", conv!());
const PCT200: &str = concat!("x = percent above the 200-day SMA. When x crosses up through 100 (from below 100), that bar's high is the buy level and its low the sell level (kept until the next such cross); buy when the close crosses above the buy level while x was above 100 on one of the last 11 bars; out when the close crosses under the sell level. It needs the close at twice its 200-day average, which indices rarely reach.", conv!());
const PCT400: &str = concat!("As the 200-day record on the 400-day SMA, but out at the close of the day after the monthly option expiry. OptionExp() (lead decision 181) = 1 on the last trading bar on or before the third Friday of the month, within that week (a holiday Friday moves it to the Thursday); US instruments only.", conv!());
const STOCH: &str = concat!("Stoch(21,10) crossing over 81 is the set-up; buy when within 5 bars the close crosses above its high; out when the close crosses under the low of the last cross under 79. Short at a cross under 19 and a close under that bar's low within 5 bars; cover on a close above the high of the last cross over 21.", conv!());
const RETEST: &str = concat!("p1 = WillR(30) + 100 (Headley's 0-100 Percent R). The Buy formula re-enters a confirmed long that is still within 30 bars of its confirmation when p1 crosses back under 79.5 (the retest); the Sell formula is the plain confirmation state with a 79 exit level, so the shipped long sells on a close under the low of the last cross under 79. Each order formula is evaluated as stored. Short mirrored (20.5 retest, 21 exit).", conv!());

const NAN: f64 = f64::NAN;
type S = Vec<f64>;
fn fin(v: f64) -> bool { v.is_finite() }
fn tru(v: f64) -> bool { fin(v) && v != 0. }
fn bv(v: bool) -> f64 { v as u8 as f64 }
fn kv(v: f64, n: usize) -> S { vec![v; n] }
/// Two-argument operation; undefined when either side is, or when the result is not finite (division by 0).
fn z(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> S {
    a.iter().zip(b).map(|(&x, &y)| if fin(x) && fin(y) { let r = f(x, y); if r.is_finite() { r } else { NAN } } else { NAN }).collect()
}
fn gt(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x > y)) }
fn le(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x <= y)) }
fn lt(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x < y)) }
fn eq(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x == y)) }
fn and(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x != 0. && y != 0.)) }
fn or(a: &[f64], b: &[f64]) -> S { z(a, b, |x, y| bv(x != 0. || y != 0.)) }
fn cross(a: &[f64], b: &[f64]) -> S {
    (0..a.len()).map(|i| if i >= 1 && fin(a[i]) && fin(b[i]) && fin(a[i - 1]) && fin(b[i - 1]) { bv(a[i] > b[i] && a[i - 1] <= b[i - 1]) } else { NAN }).collect()
}
fn back(v: &[f64], k: usize) -> S { (0..v.len()).map(|i| if i >= k { v[i - k] } else { NAN }).collect() }
fn win(v: &[f64], n: usize, f: impl Fn(&[f64]) -> f64) -> S {
    (0..v.len()).map(|i| if i + 1 < n { NAN } else { let w = &v[i + 1 - n..=i]; if w.iter().all(|x| fin(*x)) { f(w) } else { NAN } }).collect()
}
fn sum(v: &[f64], n: usize) -> S { win(v, n, |w| w.iter().sum()) }
fn hhv(v: &[f64], n: usize) -> S { win(v, n, |w| w.iter().copied().fold(f64::MIN, f64::max)) }
fn llv(v: &[f64], n: usize) -> S { win(v, n, |w| w.iter().copied().fold(f64::MAX, f64::min)) }
fn roc_d(v: &[f64], k: usize) -> S { z(v, &back(v, k), |a, b| a - b) }
/// ValueWhen(1, c, x): x on the latest bar where c was true; undefined before the first.
fn vw(c: &[f64], x: &[f64]) -> S { let mut v = NAN; c.iter().zip(x).map(|(&q, &y)| { if tru(q) { v = y; } v }).collect() }
/// BarsSince(c): bars since c was last true; undefined before the first and wherever c is undefined.
fn bs(c: &[f64]) -> S {
    let mut last: Option<usize> = None;
    (0..c.len()).map(|i| { if tru(c[i]) { last = Some(i); } match last { Some(j) if fin(c[i]) => (i - j) as f64, _ => NAN } }).collect()
}
/// If(c1, v1, If(c2, v2, PREV)); undefined where any input is; PREV = 0 on the first defined bar.
fn prev_if(c1: &[f64], v1: &[f64], c2: &[f64], v2: &[f64]) -> S {
    let mut p = NAN;
    (0..c1.len()).map(|i| {
        p = if fin(c1[i]) && fin(v1[i]) && fin(c2[i]) && fin(v2[i]) {
            if c1[i] != 0. { v1[i] } else if c2[i] != 0. { v2[i] } else if fin(p) { p } else { 0. }
        } else { NAN };
        p
    }).collect()
}
/// Alert(c, n): c true on this bar or one of the n - 1 before; undefined where c is.
fn alert(c: &[f64], n: usize) -> S { (0..c.len()).map(|i| if fin(c[i]) { bv(c[i + 1 - n.min(i + 1)..=i].iter().any(|q| tru(*q))) } else { NAN }).collect() }
fn sdp(v: &[f64], n: usize) -> S { win(v, n, |w| { let m = w.iter().sum::<f64>() / n as f64; (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt() }) }
fn bband(c: &[f64], n: usize, d: f64) -> S { z(&msws::sma(c, n), &sdp(c, n), |m, s| m + d * s) }
fn accel(x: &Ctx, n: usize, up: bool, printed_shark: bool) -> S {
    let v: S = (0..x.c.len()).map(|i| {
        let (h, l) = (x.h[i], x.l[i]);
        let w = if printed_shark { (h - l) / (h + l) / 2.0 } else { (h - l) / ((h + l) / 2.0) };
        if up { h * (1.0 + 2.0 * (w * 1000.0) * 0.001) } else { l * (1.0 - 2.0 * (w * 1000.0) * 0.001) }
    }).map(|v| if v.is_finite() { v } else { NAN }).collect();
    msws::sma(&v, n)
}
fn ema_a(v: &[f64], a: f64) -> S {
    let mut p = NAN;
    v.iter().map(|&x| { if fin(x) { p = if fin(p) { p + a * (x - p) } else { x }; } p }).collect()
}
fn macd(c: &[f64]) -> S { let mut m = z(&ema_a(c, 0.15), &ema_a(c, 0.075), |a, b| a - b); for v in m.iter_mut().take(25) { *v = NAN; } m }
fn eff(c: &[f64]) -> S { z(&roc_d(c, 20), &sum(&roc_d(c, 1).iter().map(|v| v.abs()).collect::<S>(), 20), |a, s| (a / s) * 100.0) }
fn willr1(x: &Ctx) -> S {
    let (hh, ll) = (hhv(&x.h, 30), llv(&x.l, 30));
    (0..x.c.len()).map(|i| { let d = hh[i] - ll[i]; if fin(d) && d > 0. { -100.0 * (hh[i] - x.c[i]) / d + 100.0 } else { NAN } }).collect()
}
/// Day of week, 0 = Sunday (Sakamoto), of YYYY-MM-DD.
fn dow(y: i64, m: i64, d: i64) -> i64 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    (y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d) % 7
}
fn ymd(s: &str) -> (i64, i64, i64) { (s[..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(1), s[8..10].parse().unwrap_or(1)) }
/// OptionExp(): 1 on the last bar on or before the month's third Friday, within that week (lead decision 181; as agtsic_expiry).
fn option_exp(x: &Ctx) -> S {
    (0..x.r.len()).map(|i| {
        let (y, m, d) = ymd(&x.r[i].date);
        let fri = 1 + (5 - dow(y, m, 1) + 7) % 7 + 14;
        if d > fri || d <= fri - 7 { return 0.; }
        if d == fri { return 1.; }
        bv(match x.r.get(i + 1) { Some(n) => { let (y2, m2, d2) = ymd(&n.date); (y2, m2) != (y, m) || d2 > fri } None => false })
    }).collect()
}

/// (Buy, Sell, Sell Short, Buy to Cover) of a record, as stored; NaN = undefined.
fn k30_formulas(id: &str, x: &Ctx) -> Option<[S; 4]> {
    let n = x.c.len();
    let (h, l, c) = (&x.h[..], &x.l[..], &x.c[..]);
    let k = |v: f64| kv(v, n);
    let none = || k(NAN);
    let (pdi, mdi, adx) = msws::dmi(x, 14);
    Some(match id {
        "phbttk_bb80" => {
            let top = bband(c, 80, 2.);
            let el = and(&cross(c, &vw(&cross(c, &top), h)), &gt(c, &top));
            let xl = cross(&vw(&cross(&mdi, &pdi), l), c);
            let lt_ = prev_if(&el, &k(1.), &xl, &k(0.));
            let bot = bband(c, 80, -2.);
            [cross(&lt_, &k(0.5)), xl.clone(), and(&cross(&vw(&cross(&bot, c), l), c), &lt(c, &bot)), cross(c, &vw(&cross(&pdi, &mdi), h))]
        }
        "phbttk_accel_bands" => {
            let (top, bot, e10) = (accel(x, 20, true, false), accel(x, 20, false, false), msws::ema(c, 10));
            let el = eq(&sum(&gt(c, &top), 2), &k(2.));
            let xl = lt(c, &e10);
            let es = eq(&sum(&lt(c, &bot), 2), &k(2.));
            let xs = gt(c, &e10);
            [cross(&prev_if(&el, &k(1.), &xl, &k(0.)), &k(0.5)), xl.clone(), cross(&k(-0.5), &prev_if(&es, &k(-1.), &xs, &k(0.))), xs.clone()]
        }
        "phbttk_accel_bb" => {
            let side = |up: bool| -> (S, S) {
                let (a20, a80) = (accel(x, 20, up, false), accel(x, 80, up, false));
                let b20 = bband(c, 20, if up { 2. } else { -2. });
                let (bc, sc) = if up {
                    (and(&and(&and(&eq(&sum(&gt(c, &a20), 2), &k(2.)), &eq(&sum(&gt(c, &a80), 2), &k(2.))), &gt(&b20, &a20)), &gt(c, &b20)),
                     and(&cross(&vw(&cross(&a20, &b20), l), c), &le(&bs(&cross(&a20, &b20)), &k(20.))))
                } else {
                    (and(&and(&and(&eq(&sum(&lt(c, &a20), 2), &k(2.)), &eq(&sum(&lt(c, &a80), 2), &k(2.))), &lt(&b20, &a20)), &lt(c, &b20)),
                     and(&cross(c, &vw(&cross(&b20, &a20), h)), &le(&bs(&cross(&b20, &a20)), &k(20.))))
                };
                let t = prev_if(&bc, &k(1.), &sc, &k(0.));
                (cross(&eq(&t, &k(1.)), &k(0.5)), cross(&eq(&t, &k(0.)), &k(0.5)))
            };
            let ((el, xl), (es, xs)) = (side(true), side(false));
            [el, xl, es, xs]
        }
        "phbttk_dmi_diff" => {
            let xx = msws::sma(&z(&pdi, &mdi, |a, b| a - b), 10);
            let mx = msws::sma(&xx, 10);
            let (bset, sset) = (cross(&xx, &mx), cross(&mx, &xx));
            let el = and(&cross(c, &vw(&bset, h)), &lt(&bs(&bset), &k(5.)));
            let lt_ = prev_if(&el, &k(1.), &gt(&adx, &pdi), &k(0.));
            let es = and(&cross(&vw(&sset, l), c), &lt(&bs(&sset), &k(5.)));
            let st = prev_if(&es, &k(-1.), &gt(&adx, &mdi), &k(0.));
            [cross(&lt_, &k(0.5)), cross(&k(0.5), &lt_), cross(&k(-0.5), &st), cross(&st, &k(-0.5))]
        }
        "phbttk_efficiency" => {
            let ef = eff(c);
            let (u, d) = (cross(&ef, &k(30.)), cross(&k(-30.), &ef));
            let el = and(&cross(c, &vw(&u, h)), &lt(&bs(&u), &k(5.)));
            let xl = cross(&vw(&cross(&k(30.), &ef), l), c);
            let lt_ = prev_if(&el, &k(1.), &xl, &k(0.));
            let es = and(&cross(&vw(&d, l), c), &lt(&bs(&d), &k(5.)));
            let xs = cross(c, &vw(&cross(&ef, &k(-30.)), h));
            let st = prev_if(&es, &k(-1.), &xs, &k(0.));
            [cross(&lt_, &k(0.5)), cross(&k(0.5), &lt_), cross(&k(-0.5), &st), cross(&st, &k(-0.5))]
        }
        "phbttk_force_lt" | "phbttk_force_st" => {
            let (p1, p2) = if id == "phbttk_force_lt" { (60, 120) } else { (14, 28) };
            let vol: S = x.r.iter().map(|r| r.volume.unwrap_or(NAN)).collect();
            let fi = z(&roc_d(c, 1), &vol, |a, b| a * b);
            let (ma1, ma2) = (msws::sma(&fi, p1), msws::sma(&fi, p2));
            let (up, dn) = (cross(&ma1, &ma2), cross(&ma2, &ma1));
            // long record
            let bset = and(&up, &gt(&z(&ma1, &ma2, f64::min), &k(0.)));
            let setup = prev_if(&bset, &k(1.), &dn, &k(-1.));
            let bc = and(&and(&cross(c, &vw(&bset, h)), &le(&bs(&bset), &k(20.))), &eq(&setup, &k(1.)));
            let sc = and(&cross(&vw(&dn, l), c), &eq(&setup, &k(-1.)));
            let tl = prev_if(&bc, &k(1.), &sc, &k(0.));
            // short record
            let sset = and(&dn, &lt(&z(&ma1, &ma2, f64::max), &k(0.)));
            let setup2 = prev_if(&up, &k(1.), &sset, &k(-1.));
            let xs = and(&cross(c, &vw(&up, h)), &eq(&setup2, &k(1.)));
            let es = and(&and(&cross(&vw(&sset, l), c), &le(&bs(&sset), &k(20.))), &eq(&setup2, &k(-1.)));
            let ts = prev_if(&es, &k(1.), &xs, &k(0.));
            [cross(&eq(&tl, &k(1.)), &k(0.5)), cross(&eq(&tl, &k(0.)), &k(0.5)), cross(&eq(&ts, &k(1.)), &k(0.5)), cross(&eq(&ts, &k(0.)), &k(0.5))]
        }
        "phbttk_mom_div" => {
            let m = macd(c);
            let norm = |v: &[f64]| { let (lo, hi) = (llv(v, 40), hhv(v, 40)); z(&z(v, &lo, |a, b| a - b), &z(&hi, &lo, |a, b| a - b), |a, b| (a / b) * 100.0) };
            let (md, cd) = (norm(&m), norm(c));
            let (mn, mx) = (z(&md, &cd, f64::min), z(&md, &cd, f64::max));
            let tl = prev_if(&eq(&mn, &k(100.)), &k(1.), &lt(&mx, &k(90.)), &k(0.));
            let ts = prev_if(&eq(&mx, &k(0.)), &k(1.), &gt(&mn, &k(10.)), &k(0.));
            [cross(&eq(&tl, &k(1.)), &k(0.5)), cross(&eq(&tl, &k(0.)), &k(0.5)), cross(&eq(&ts, &k(1.)), &k(0.5)), cross(&eq(&ts, &k(0.)), &k(0.5))]
        }
        "phbttk_options_shark" => {
            let ef = eff(c);
            let vol: S = x.r.iter().map(|r| r.volume.unwrap_or(NAN)).collect();
            let vcon = gt(&vol, &back(&msws::sma(&vol, 20), 1));
            let adx_up = gt(&roc_d(&adx, 1), &k(0.));
            let ox = eq(&back(&option_exp(x), 1), &k(1.));
            let (actop, bbtop) = (accel(x, 20, true, true), bband(c, 80, 2.));
            let con1 = cross(&gt(c, &z(&actop, &bbtop, f64::max)), &k(0.5));
            let el = and(&and(&and(&and(&cross(c, &vw(&con1, h)), &le(&bs(&con1), &k(5.))), &gt(&ef, &k(30.))), &or(&vcon, &vw(&con1, &vcon))), &adx_up);
            let xl = or(&or(&cross(&vw(&cross(&bbtop, l), l), c), &cross(&vw(&cross(&k(30.), &ef), l), c)), &ox);
            let (acbot, bbbot) = (accel(x, 20, false, true), bband(c, 80, -2.));
            let con2 = cross(&lt(c, &z(&acbot, &bbbot, f64::min)), &k(0.5));
            let es = and(&and(&and(&and(&cross(c, &vw(&con2, l)), &le(&bs(&con2), &k(5.))), &lt(&ef, &k(-30.))), &or(&vcon, &vw(&con2, &vcon))), &adx_up);
            let xs = or(&or(&cross(c, &vw(&cross(h, &bbbot), h)), &cross(c, &vw(&cross(&ef, &k(-30.)), h))), &ox);
            [el, xl, es, xs]
        }
        "phbttk_pct_above_ma" | "phbttk_pct_above_ma400" => {
            let p = if id == "phbttk_pct_above_ma" { 200 } else { 400 };
            let m = msws::sma(c, p);
            let xp = z(&z(c, &m, |a, b| a - b), &m, |a, b| (a / b) * 100.0);
            let up = and(&gt(&xp, &k(100.)), &back(&lt(&xp, &k(100.)), 1));
            let xbp = prev_if(&up, h, &k(0.), &k(0.));
            let bc = and(&cross(c, &xbp), &alert(&gt(&xp, &k(100.)), 11));
            if p == 200 {
                let xsp = prev_if(&up, l, &k(0.), &k(0.));
                [bc, cross(&xsp, c), none(), none()]
            } else {
                let t = prev_if(&bc, &k(1.), &eq(&back(&option_exp(x), 1), &k(1.)), &k(0.));
                [cross(&eq(&t, &k(1.)), &k(0.5)), cross(&eq(&t, &k(0.)), &k(0.5)), none(), none()]
            }
        }
        "phbttk_stoch_conf" => {
            let st = msws::stoch(x, 21, 10);
            let u = cross(&st, &k(81.));
            let d = cross(&k(19.), &st);
            [and(&le(&bs(&u), &k(5.)), &cross(c, &vw(&u, h))), cross(&vw(&cross(&k(79.), &st), l), c),
             and(&le(&bs(&d), &k(5.)), &cross(&vw(&d, l), c)), cross(c, &vw(&cross(&st, &k(21.)), h))]
        }
        "phbttk_pctr_retest" => {
            let p1 = willr1(x);
            // long: Buy with the 79.5 exit level and the retest; Sell with the 79 level
            let xu = cross(&p1, &k(80.));
            let bcl = and(&le(&bs(&xu), &k(5.)), &cross(c, &vw(&xu, h)));
            let lvl_l = |v: f64| cross(&vw(&cross(&k(v), &p1), l), c);
            let (sc795, sc79) = (lvl_l(79.5), lvl_l(79.));
            let tr = prev_if(&bcl, &k(1.), &sc795, &k(0.));
            let br = and(&back(&and(&eq(&tr, &k(1.)), &lt(&bs(&bcl), &k(30.))), 1), &cross(&k(79.5), &p1));
            let tr1 = prev_if(&br, &k(1.), &sc795, &k(0.));
            let trs = prev_if(&bcl, &k(1.), &sc79, &k(0.));
            // short: both formulas use the 21 cover level
            let xd = cross(&k(20.), &p1);
            let bcs = and(&le(&bs(&xd), &k(5.)), &cross(&vw(&xd, l), c));
            let scs = cross(c, &vw(&cross(&p1, &k(21.)), h));
            let ts = prev_if(&bcs, &k(1.), &scs, &k(0.));
            let brs = and(&back(&and(&eq(&ts, &k(1.)), &lt(&bs(&bcs), &k(30.))), 1), &cross(&p1, &k(20.5)));
            let ts1 = prev_if(&brs, &k(1.), &scs, &k(0.));
            [cross(&eq(&tr1, &k(1.)), &k(0.5)), cross(&eq(&trs, &k(0.)), &k(0.5)), cross(&eq(&ts1, &k(1.)), &k(0.5)), cross(&eq(&ts, &k(0.)), &k(0.5))]
        }
        _ => return None,
    })
}

fn run_k30(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed_k30(id) || from >= to || to >= x.r.len() { return None; }
    let side = p.get("side").copied().unwrap_or(0.0).round() as i32;
    let [el, xl, es, xs] = k30_formulas(id, x)?;
    let b = |v: &S, on: bool| -> Vec<bool> { v.iter().map(|q| on && tru(*q)).collect() };
    let sig = msws::Sig { el: b(&el, side >= 0), xl: b(&xl, side >= 0), es: b(&es, side <= 0), xs: b(&xs, side <= 0) };
    // Decisions 184/185 (lane K32): MetaStock System Tester reverses on an opposite entry; p.reverse = 0 = the earlier reading.
    Some(msws::simulate_conv(x, from, to, &sig, msws::EXEC_DEFAULT, msws::reverse_on(p)))
}

// Instrument admission: the BTTK 2.0 shipped System Tester records 2026-11-03 (titles lane K30). The two records that read
// OptionExp() run on US instruments only (instrument_policy phbttk_symbol_issue).
pub const ADMISSION_K30: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "phbttk" && reviewed_k30(def.id),
    label: "Price Headley's Big Trends Toolkit 2.0 ships these MetaStock system tests without a market or a printed test (the manual shows US stock charts); runs on indices, ETFs and stocks, US and India, are a declared application. The two records that exit after the monthly option expiry run on US instruments only.",
    basis: "Big Trends Toolkit 2.0 for MetaStock (BigTrends/Equis 2008): the System Tester records in MS10PRFT.DTA of Bigtrendv2setup.exe (research share USB HDD 1/MetaStock/@Systems & Indicators/Price Headleys Big Trends Toolkit 2.0; slots read by batch X, authors/batch-x/keep/2617/PRFT_slots.txt), the manual BTTK_2 0.pdf for the words; independent check authors/titles-1030/K30w/k30_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn confirm_then_closing_stop() {
        let mut v: Vec<(f64, f64, f64)> = (0..30).map(|i| { let b = 100. - (i % 5) as f64; (b + 1., b - 1., b) }).collect();
        v.push((106., 99., 105.5)); // 30: %R crosses over 80 -> setup, high 106
        v.push((107., 105., 106.5)); // 31: close above 106 -> long at 106.5
        v.push((107., 100., 101.)); // 32: %R back to <= 79 -> stop = low 100
        v.push((102., 99., 99.5)); // 33: close under 100 -> exit at 99.5
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: c, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("phbttk_pctr_long", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (31, 106.5, 33, 99.5));
    }
    #[test]
    fn k30_helpers_follow_the_declared_readings() {
        let n = f64::NAN;
        let c = [n, 0., 1., 0., 0., 1., 0.];
        let v: Vec<f64> = (0..7).map(|i| 10. + i as f64).collect();
        let w = vw(&c, &v);
        assert!(w[1].is_nan() && w[2] == 12. && w[4] == 12. && w[6] == 15.);
        let b = bs(&c);
        assert!(b[1].is_nan() && b[2] == 0. && b[4] == 2. && b[5] == 0.);
        // If(e,1,If(x,0,PREV)): undefined while an input is, then PREV starts at 0
        let s = prev_if(&[n, 0., 1., 0., 0.], &[1.; 5], &[0., 0., 0., 1., 0.], &[0.; 5]);
        assert!(s[0].is_nan() && s[1] == 0. && s[2] == 1. && s[3] == 0. && s[4] == 0.);
        let a = alert(&[1., 0., 0., 0.], 3);
        assert_eq!((a[2], a[3]), (1., 0.));
        let cr = cross(&[1., 2., 3.], &[2., 2., 2.]);
        assert!(cr[0].is_nan() && cr[1] == 0. && cr[2] == 1.);
        // OptionExp: third Friday 2008-03-21 was Good Friday -> Thursday 2008-03-20; 2024-06-21 a normal Friday
        let mk = |d: &str| crate::Row { ticker: "T".into(), date: d.into(), open: 1., high: 1., low: 1., close: 1., volume: None };
        let r = vec![mk("2008-03-19"), mk("2008-03-20"), mk("2008-03-24"), mk("2024-06-20"), mk("2024-06-21"), mk("2024-06-24")];
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(option_exp(&x), vec![0., 1., 0., 0., 1., 0.]);
    }
    #[test]
    fn k30_records_run_with_sane_fills() {
        let r = super::super::nijat::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED_K30 {
            assert!(catalogue().iter().any(|d| d.id == *id && d.status == Status::Live), "{id}");
            assert!(family_of(id).is_some() && params(id).is_some(), "{id}");
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            if !id.starts_with("phbttk_pct_above_ma") { assert!(t.len() >= 2, "{id}: {}", t.len()); }
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            for z in &t { assert!(z.legs[0].px == r[z.legs[0].i].close && z.exit_px == r[z.exit_i].close, "{id}"); }
            if !LONG_ONLY.contains(id) {
                let mut p = Params::new();
                p.insert("side".into(), 1.);
                assert!(run(id, &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.), "{id}");
            }
            assert!(run(id, &x, 5, 5, &Params::new()).is_none());
        }
        // decisions 184/185: two-sided records reverse on an opposite entry by default; p.reverse = 0 never does
        let rev = |id: &&str, r: f64| { let mut p = Params::new(); p.insert("reverse".into(), r);
            run(id, &x, 0, x.c.len() - 1, &p).unwrap().iter().filter(|z| z.reason.starts_with("reversed by the opposite Enter")).count() };
        let two: Vec<&&str> = REVIEWED_K30.iter().filter(|id| !LONG_ONLY.contains(*id)).collect();
        assert!(two.iter().any(|id| rev(id, 1.) > 0));
        assert!(two.iter().all(|id| rev(id, 0.) == 0 && params(id).unwrap().iter().any(|d| d.key == "reverse" && d.def == 1.)));
    }
    #[test]
    fn k30_admission_us_only_for_option_expiry() {
        use crate::instrument_policy::{check, m3_withdrawn, policy};
        for d in catalogue().iter().filter(|d| reviewed_k30(d.id)) {
            assert!((ADMISSION_K30.reviewed)(d) && !(crate::books::phbt::ADMISSION.reviewed)(d), "{}", d.id);
            assert_eq!(policy(d).requirement, "index_etf_or_stock", "{}", d.id);
            let us = US_ONLY.contains(&d.id);
            for t in ["SPY", "QQQ", "SPX", "NDX", "DJI"] { assert!(check(d, t, false).is_none() || m3_withdrawn(d.id, t), "{} {t}", d.id); }
            for t in ["NIFTY 50", "NIFTYBANK"] { assert_eq!(check(d, t, false).is_some(), us, "{} {t}", d.id); }
            assert_eq!(check(d, "RELIANCE", true).is_some(), us, "{}", d.id);
        }
    }
}
