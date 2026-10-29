//! Toby Crabel, "Day Trading with Short Term Price Patterns and Opening Range Breakout" (Traders Press, 1990).
//! Source copy: Book-to-rust/Books/Toby Crabel/. Transcription, page refs and verification: library/tcdt/.
//!
//! Every coded test is the book's own computer test: a pattern on Day 2 (yesterday), then on Day 1 (today) an
//! entry "at a predetermined level above or below the open" (ORB) or on the open, exit on the close (or the
//! close N days later where the book says so). "Stops were not used" (printed p.123). Levels are the book's
//! S&P 500 futures values in index points (Crabel's "80 points" = 0.80 index points: printed p.17, open 271.00,
//! high 272.90 = "190 points above the open"). Defaults are those literal values; on other instruments they are
//! an instrument swap only. level_mode = 1 (labelled option) uses the book's own Chapter 1 "Stretch" instead.
use crate::connors::{Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tcdt";
/// Admitted ids (authors/crabel/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["tcdt_oc_pattern", "tcdt_oc_rangeexp", "tcdt_cc_pattern"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) || W2A_REVIEWED.contains(&id) }
// ---- intraday wave 2a block (claude/intraday-w2a, 2026-10-29)
/// Open-to-close rules (Table E printed p.72; Inside Day patterns 1-4 printed p.115): entry on the open, exit on the same
/// close, no stop, so daily bars decide every trade (no intraday order question). Admitted after an independent daily
/// ledger (library/intraday/verify/crabel_w2a_ref.py) matched the engine, US and India.
pub const W2A_REVIEWED: &[&str] = &["tcdt_oc_nr_re", "tcdt_idp1", "tcdt_idp2", "tcdt_idp3", "tcdt_idp4"];
// ---- end intraday wave 2a block ----

#[derive(Clone, Copy, PartialEq)]
enum Pat { Any, Id, Doji, Nr4, DojiNr4, IdNr4, Nr7, Ws4, Ws7, BearHook, BullHook, Bnr2, Bnr3, DojiR1, R1 }

/// One printed ORB test row: id, name, pattern, signed level (index points), side (+1 buy / -1 sell),
/// days held after the entry day, and the printed S&P 1982-88 result (trades, win %, avg win, avg loss, gross, page).
struct Orb { id: &'static str, name: &'static str, pat: Pat, lvl: f64, side: f64, hold: f64,
    t: &'static str, w: &'static str, avg: &'static str }

macro_rules! orb { ($id:expr,$n:expr,$p:ident,$l:expr,$s:expr,$h:expr,$t:expr,$w:expr,$aw:expr,$al:expr,$gp:expr,$pg:expr) => {
    Orb{id:$id,name:$n,pat:Pat::$p,lvl:$l,side:$s,hold:$h,t:$t,w:$w,avg:concat!("avg win ",$aw," / avg loss ",$al,"; gross ",$gp," (",$pg,")")} } }

const B: f64 = 1.0;
const S: f64 = -1.0;

static ORBS: &[Orb] = &[
    // Control group: any day. Printed p.20 ("Indicated" row = % closed beyond the entry level) and p.108.
    orb!("tcdt_ctl_p80","Control ORB · open +80 · buy",Any,0.80,B,0.,"","55%","","","","pr20, pr108"),
    orb!("tcdt_ctl_p160","Control ORB · open +160 · buy",Any,1.60,B,0.,"","58%","","","","pr20, pr108"),
    orb!("tcdt_ctl_p300","Control ORB · open +300 · buy",Any,3.00,B,0.,"","63%","","","","pr20"),
    orb!("tcdt_ctl_p600","Control ORB · open +600 · buy",Any,6.00,B,0.,"","65%","","","","pr20"),
    orb!("tcdt_ctl_m80","Control ORB · open −80 · sell",Any,-0.80,S,0.,"","49%","","","","pr20, pr108"),
    orb!("tcdt_ctl_m160","Control ORB · open −160 · sell",Any,-1.60,S,0.,"","49%","","","","pr20, pr108"),
    orb!("tcdt_ctl_m300","Control ORB · open −300 · sell",Any,-3.00,S,0.,"","49%","","","","pr20"),
    orb!("tcdt_ctl_m600","Control ORB · open −600 · sell",Any,-6.00,S,0.,"","55%","","","","pr20"),
    orb!("tcdt_id_p160","Inside Day ORB · open +160 · buy",Id,1.60,B,0.,"57","61%","$896","$693","$16,124","pr126"),
    orb!("tcdt_id_p80","Inside Day ORB · open +80 · buy",Id,0.80,B,0.,"103","57%","$860","$612","$23,800","pr126"),
    orb!("tcdt_id_m80","Inside Day ORB · open −80 · sell",Id,-0.80,S,0.,"89","48%","$739","$571","$5,520","pr126"),
    orb!("tcdt_id_m160","Inside Day ORB · open −160 · sell",Id,-1.60,S,0.,"58","45%","$691","$762","−$6,429","pr126"),
    orb!("tcdt_doji_p160","Doji (50 pt) ORB · open +160 · buy",Doji,1.60,B,0.,"123","62%","$630","$559","$21,624","pr140"),
    orb!("tcdt_doji_p80","Doji (50 pt) ORB · open +80 · buy",Doji,0.80,B,0.,"222","60%","$665","$579","$36,950","pr140"),
    orb!("tcdt_doji_m80","Doji (50 pt) ORB · open −80 · sell",Doji,-0.80,S,0.,"232","58%","$690","$487","$44,749","pr140"),
    orb!("tcdt_doji_m160","Doji (50 pt) ORB · open −160 · sell",Doji,-1.60,S,0.,"127","60%","$670","$595","$20,575","pr140"),
    orb!("tcdt_nr4_p160","NR4 ORB · open +160 · buy",Nr4,1.60,B,0.,"145","62%","$1,093","$1,021","$42,224","pr111, pr145"),
    orb!("tcdt_nr4_p80","NR4 ORB · open +80 · buy",Nr4,0.80,B,0.,"233","57%","$1,060","$865","$54,550","pr111, pr145"),
    orb!("tcdt_nr4_m80","NR4 ORB · open −80 · sell",Nr4,-0.80,S,0.,"231","59%","$790","$961","$16,195","pr111, pr145"),
    orb!("tcdt_nr4_m160","NR4 ORB · open −160 · sell",Nr4,-1.60,S,0.,"140","61%","$750","$1,116","$2,395","pr111, pr145"),
    orb!("tcdt_dojinr4_p160","Doji & NR4 ORB · open +160 · buy",DojiNr4,1.60,B,0.,"62","65%","$545","$730","$5,724","pr150"),
    orb!("tcdt_dojinr4_p80","Doji & NR4 ORB · open +80 · buy",DojiNr4,0.80,B,0.,"105","58%","$673","$630","$13,375","pr150"),
    orb!("tcdt_dojinr4_m80","Doji & NR4 ORB · open −80 · sell",DojiNr4,-0.80,S,0.,"111","63%","$684","$432","$30,199","pr150"),
    orb!("tcdt_dojinr4_m160","Doji & NR4 ORB · open −160 · sell",DojiNr4,-1.60,S,0.,"66","67%","$582","$444","$15,875","pr150"),
    orb!("tcdt_idnr4_p160","ID/NR4 ORB · open +160 · buy",IdNr4,1.60,B,0.,"32","66%","$998","$1,040","$9,525","pr155"),
    orb!("tcdt_idnr4_p80","ID/NR4 ORB · open +80 · buy",IdNr4,0.80,B,0.,"55","55%","$997","$807","$9,750","pr155"),
    orb!("tcdt_idnr4_m80","ID/NR4 ORB · open −80 · sell",IdNr4,-0.80,S,0.,"54","56%","$778","$602","$8,895","pr155"),
    orb!("tcdt_idnr4_m160","ID/NR4 ORB · open −160 · sell",IdNr4,-1.60,S,0.,"36","53%","$722","$712","$1,620","pr155"),
    orb!("tcdt_nr7_p160","NR7 ORB · open +160 · buy",Nr7,1.60,B,0.,"84","67%","$881","$984","$21,800","pr112, pr160"),
    orb!("tcdt_nr7_p80","NR7 ORB · open +80 · buy",Nr7,0.80,B,0.,"132","61%","$918","$760","$35,625","pr112, pr160"),
    orb!("tcdt_nr7_m80","NR7 ORB · open −80 · sell",Nr7,-0.80,S,0.,"120","62%","$822","$762","$25,820","pr112, pr160"),
    orb!("tcdt_nr7_m160","NR7 ORB · open −160 · sell",Nr7,-1.60,S,0.,"84","60%","$716","$647","$13,795","pr112, pr160"),
    orb!("tcdt_ws4_p160","WS4 ORB · open +160 · sell",Ws4,1.60,S,0.,"129","46%","$1,247","$859","$13,425","pr109"),
    orb!("tcdt_ws4_p80","WS4 ORB · open +80 · sell",Ws4,0.80,S,0.,"244","45%","$961","$754","$4,624","pr109"),
    orb!("tcdt_ws4_m80","WS4 ORB · open −80 · sell",Ws4,-0.80,S,0.,"245","47%","$1,028","$646","$34,125","pr109"),
    orb!("tcdt_ws4_m160","WS4 ORB · open −160 · sell",Ws4,-1.60,S,0.,"134","49%","$1,255","$646","$38,899","pr109"),
    orb!("tcdt_ws7_p160","WS7 ORB · open +160 · sell",Ws7,1.60,S,0.,"75","44%","$1,709","$877","$19,550","pr110"),
    orb!("tcdt_ws7_p80","WS7 ORB · open +80 · sell",Ws7,0.80,S,0.,"137","46%","$1,186","$826","$13,574","pr110"),
    orb!("tcdt_ws7_m80","WS7 ORB · open −80 · sell",Ws7,-0.80,S,0.,"139","45%","$1,292","$730","$25,850","pr110"),
    orb!("tcdt_ws7_m160","WS7 ORB · open −160 · sell",Ws7,-1.60,S,0.,"77","48%","$1,670","$675","$34,799","pr110"),
    orb!("tcdt_bearhook_p160","Bear Hook ORB · open +160 · buy",BearHook,1.60,B,0.,"6","50%","$2,150","$166","$6,999","pr274"),
    orb!("tcdt_bearhook_p80","Bear Hook ORB · open +80 · buy",BearHook,0.80,B,0.,"11","55%","$1,391","$340","$6,649","pr274"),
    orb!("tcdt_bearhook_m80","Bear Hook ORB · open −80 · sell",BearHook,-0.80,S,0.,"16","75%","$780","$1,400","$3,830","pr274"),
    orb!("tcdt_bearhook_m160","Bear Hook ORB · open −160 · sell",BearHook,-1.60,S,0.,"10","80%","$721","$2,912","$0","pr274"),
    orb!("tcdt_bullhook_p160","Bull Hook ORB · open +160 · buy",BullHook,1.60,B,0.,"14","64%","$983","$460","$6,549","pr279"),
    orb!("tcdt_bullhook_p80","Bull Hook ORB · open +80 · buy",BullHook,0.80,B,0.,"20","55%","$1,140","$366","$9,250","pr279"),
    orb!("tcdt_bullhook_m80","Bull Hook ORB · open −80 · buy",BullHook,-0.80,B,0.,"16","63%","$542","$854","$300","pr279"),
    orb!("tcdt_bullhook_m160","Bull Hook ORB · open −160 · sell",BullHook,-1.60,S,0.,"8","50%","$775","$412","$1,450","pr279"),
    // Doji Lines and NR, Table B (printed p.135): yesterday's open-close difference < 50 pts and/or yesterday's range < 200 pts.
    orb!("tcdt_dr1_1","50 Doji · open +100 · buy",Doji,1.00,B,0.,"162","58%","$671","$551","$25,625","pr135"),
    orb!("tcdt_dr1_2","50 Doji; R1 < 200 · open +100 · buy",DojiR1,1.00,B,0.,"88","63%","$507","$346","$16,470","pr135"),
    orb!("tcdt_dr1_3","50 Doji; R1 < 200 · open +160 · buy",DojiR1,1.60,B,0.,"55","60%","$453","$430","$5,474","pr135"),
    orb!("tcdt_dr1_4","50 Doji; R1 < 200 · open −100 · sell",DojiR1,-1.00,S,0.,"100","64%","$479","$294","$18,550","pr135"),
    orb!("tcdt_dr1_5","50 Doji; R1 < 200 · open −160 · sell",DojiR1,-1.60,S,0.,"57","67%","$419","$351","$9,249","pr135"),
    orb!("tcdt_dr1_6","R1 < 200 · open −100 · sell",R1,-1.00,S,0.,"198","59%","$470","$409","$21,925","pr135"),
    orb!("tcdt_dr1_7","R1 < 200 · open −160 · sell",R1,-1.60,S,0.,"104","63%","$418","$414","$11,849","pr135"),
    orb!("tcdt_dr1_8","R1 < 200 · open +160 · buy",R1,1.60,B,0.,"106","58%","$534","$352","$16,724","pr135"),
    orb!("tcdt_dr1_9","R1 < 200 · open +100 · buy",R1,1.00,B,0.,"178","63%","$504","$320","$36,199","pr135"),
    // 2 Bar NR (printed p.173): +180 is as printed on the S&P table (the comparison table pr108 labels the same 75% row "+160").
    orb!("tcdt_2bnr_p180_h5","2 Bar NR ORB · open +180 · buy · 5 days",Bnr2,1.80,B,5.,"28","61%","$2,535","$1,397","$27,724","pr173"),
    orb!("tcdt_2bnr_p180_h2","2 Bar NR ORB · open +180 · buy · 2 days",Bnr2,1.80,B,2.,"28","64%","$1,536","$1,285","$14,799","pr173"),
    orb!("tcdt_2bnr_p180_h1","2 Bar NR ORB · open +180 · buy · 1 day",Bnr2,1.80,B,1.,"28","61%","$1,183","$777","$11,224","pr173"),
    orb!("tcdt_2bnr_p180_h0","2 Bar NR ORB · open +180 · buy · same day",Bnr2,1.80,B,0.,"28","75%","$721","$460","$11,924","pr173"),
    orb!("tcdt_2bnr_p80_h5","2 Bar NR ORB · open +80 · buy · 5 days",Bnr2,0.80,B,5.,"51","49%","$2,296","$1,800","$10,600","pr173"),
    orb!("tcdt_2bnr_p80_h2","2 Bar NR ORB · open +80 · buy · 2 days",Bnr2,0.80,B,2.,"51","61%","$1,408","$1,465","$14,375","pr173"),
    orb!("tcdt_2bnr_p80_h1","2 Bar NR ORB · open +80 · buy · 1 day",Bnr2,0.80,B,1.,"51","51%","$1,232","$1,060","$5,520","pr173"),
    orb!("tcdt_2bnr_p80_h0","2 Bar NR ORB · open +80 · buy · same day",Bnr2,0.80,B,0.,"51","59%","$839","$752","$9,375","pr173"),
    orb!("tcdt_2bnr_m80_h0","2 Bar NR ORB · open −80 · sell · same day",Bnr2,-0.80,S,0.,"67","58%","$812","$4,033 (as printed)","$20,375","pr173"),
    orb!("tcdt_2bnr_m80_h1","2 Bar NR ORB · open −80 · sell · 1 day",Bnr2,-0.80,S,1.,"67","60%","$1,128","$1,371","$8,125","pr173"),
    orb!("tcdt_2bnr_m80_h2","2 Bar NR ORB · open −80 · sell · 2 days",Bnr2,-0.80,S,2.,"67","55%","$1,228","$1,525","$299","pr173"),
    orb!("tcdt_2bnr_m80_h5","2 Bar NR ORB · open −80 · buy · 5 days",Bnr2,-0.80,B,5.,"67","52%","$2,235","$1,598","$27,099","pr173"),
    orb!("tcdt_2bnr_m160_h0","2 Bar NR ORB · open −160 · sell · same day",Bnr2,-1.60,S,0.,"45","62%","$635","$475","$9,724","pr173"),
    orb!("tcdt_2bnr_m160_h1","2 Bar NR ORB · open −160 · sell · 1 day",Bnr2,-1.60,S,1.,"45","51%","$1,222","$1,207","$1,549","pr173"),
    orb!("tcdt_2bnr_m160_h2","2 Bar NR ORB · open −160 · buy · 2 days",Bnr2,-1.60,B,2.,"45","53%","$1,533","$1,378","$7,850","pr173"),
    orb!("tcdt_2bnr_m160_h5","2 Bar NR ORB · open −160 · buy · 5 days",Bnr2,-1.60,B,5.,"45","51%","$2,659","$1,580","$26,275","pr173"),
    orb!("tcdt_3bnr_p160_h5","3 Bar NR ORB · open +160 · buy · 5 days",Bnr3,1.60,B,5.,"38","55%","$2,323","$1,529","$22,799","pr186"),
    orb!("tcdt_3bnr_p160_h2","3 Bar NR ORB · open +160 · buy · 2 days",Bnr3,1.60,B,2.,"38","61%","$2,103","$1,425","$26,999","pr186"),
    orb!("tcdt_3bnr_p160_h1","3 Bar NR ORB · open +160 · buy · 1 day",Bnr3,1.60,B,1.,"38","66%","$1,565","$948","$26,799","pr186"),
    orb!("tcdt_3bnr_p160_h0","3 Bar NR ORB · open +160 · buy · same day",Bnr3,1.60,B,0.,"38","74%","$914","$727","$18,324","pr186"),
    orb!("tcdt_3bnr_p80_h5","3 Bar NR ORB · open +80 · buy · 5 days",Bnr3,0.80,B,5.,"63","56%","$2,056","$1,987","$16,325","pr186"),
    orb!("tcdt_3bnr_p80_h2","3 Bar NR ORB · open +80 · buy · 2 days",Bnr3,0.80,B,2.,"63","62%","$1,879","$1,482","$37,725","pr186"),
    orb!("tcdt_3bnr_p80_h1","3 Bar NR ORB · open +80 · buy · 1 day",Bnr3,0.80,B,1.,"63","54%","$1,722","$1,063","$27,725","pr186"),
    orb!("tcdt_3bnr_p80_h0","3 Bar NR ORB · open +80 · buy · same day",Bnr3,0.80,B,0.,"63","62%","$989","$875","$17,575","pr186"),
    orb!("tcdt_3bnr_m80_h0","3 Bar NR ORB · open −80 · sell · same day",Bnr3,-0.80,S,0.,"64","58%","$839","$685","$12,550","pr186"),
    orb!("tcdt_3bnr_m80_h1","3 Bar NR ORB · open −80 · sell · 1 day",Bnr3,-0.80,S,1.,"64","53%","$1,333","$1,467","$1,300","pr186"),
    orb!("tcdt_3bnr_m80_h2","3 Bar NR ORB · open −80 · buy · 2 days",Bnr3,-0.80,B,2.,"64","48%","$1,827","$1,452","$8,724","pr186"),
    orb!("tcdt_3bnr_m80_h5","3 Bar NR ORB · open −80 · buy · 5 days",Bnr3,-0.80,B,5.,"64","56%","$2,078","$1,736","$26,199","pr186"),
    orb!("tcdt_3bnr_m160_h0","3 Bar NR ORB · open −160 · sell · same day",Bnr3,-1.60,S,0.,"42","67%","$637","$585","$9,649","pr186"),
    orb!("tcdt_3bnr_m160_h1","3 Bar NR ORB · open −160 · sell · 1 day",Bnr3,-1.60,S,1.,"42","60%","$1,260","$1,447","$6,899","pr186"),
    orb!("tcdt_3bnr_m160_h2","3 Bar NR ORB · open −160 · buy · 2 days",Bnr3,-1.60,B,2.,"42","50%","$1,690","$1,565","$2,625","pr186"),
    orb!("tcdt_3bnr_m160_h5","3 Bar NR ORB · open −160 · buy · 5 days",Bnr3,-1.60,B,5.,"42","55%","$2,480","$1,858","$22,125","pr186"),
];

// ---------------------------------------------------------------- printed S&P 1982-88 tables (open-entry and ORB tables)
/// (pattern, B/S, trades, win%, avg win $, avg loss $, total $). Pattern: leftmost = oldest close vs the close before it,
/// last sign = today's open vs yesterday's close (printed p.35, p.51). Table A printed p.68 (rows 29-60 = p.52).
static TABLE_A: &[(&str, char, u32, u32, u32, u32, i32)] = &[
    ("++",'S',427,53,842,894,12294),("-+",'B',376,54,745,750,20024),("+-",'B',371,52,829,751,24625),("--",'B',326,57,873,1144,4249),
    ("---",'S',121,40,1710,692,33950),("--+",'S',166,51,879,618,23150),("-+-",'S',171,50,883,836,3200),("-++",'B',221,49,896,844,1405),
    ("+--",'B',202,56,974,840,37174),("+-+",'B',203,58,838,630,43824),("++-",'B',194,53,826,632,27625),("+++",'S',198,55,831,903,8425),
    ("++++",'B',79,56,964,963,8725),("+++-",'B',112,54,655,579,10400),("++-+",'B',95,54,750,752,5124),("++--",'B',101,57,1098,776,30349),
    ("+-++",'B',120,47,850,695,4655),("+-+-",'S',103,58,944,866,19399),("+--+",'S',90,54,893,650,17125),("+---",'S',76,42,1044,655,4600),
    ("----",'S',44,56,3126,750,29000),("---+",'S',71,45,913,609,5450),("--+-",'B',68,63,805,737,16199),("--++",'S',96,49,1047,950,2674),
    ("-+++",'S',116,62,775,816,19875),("-++-",'B',80,51,1092,708,17175),("-+-+",'B',101,61,847,479,33875),("-+--",'B',99,57,845,919,7850),
    ("+++++",'B',30,50,890,875,224),("++++-",'B',53,66,691,425,16550),("+++-+",'S',29,52,765,764,775),("+++--",'B',41,56,866,994,2024),
    ("++-++",'B',44,48,622,555,300),("++-+-",'S',38,63,1185,776,17574),("++--+",'B',31,58,791,578,6724),("++---",'S',38,47,1201,712,7374),
    ("+-+++",'S',40,60,803,525,12525),("+-++-",'S',38,53,861,686,4875),("+-+-+",'B',47,60,803,525,12525),("+-+--",'B',53,57,779,755,6000),
    ("+--++",'S',46,54,1021,995,4625),("+--+-",'B',28,64,670,532,6750),("+---+",'S',32,44,948,423,5650),("+----",'S',30,40,3425,806,26574),
    ("-++++",'B',36,56,1147,951,7725),("-+++-",'S',34,56,461,555,449),("-++-+",'B',43,60,674,785,4174),("-++--",'B',41,59,1363,658,21573),
    ("-+-++",'B',54,46,896,761,325),("-+-+-",'S',42,52,796,750,514),("-+--+",'S',44,70,991,417,25300),("-+---",'S',23,39,661,389,500),
    ("--+++",'S',50,62,908,857,11550),("--++-",'B',31,55,1758,571,21899),("--+-+",'B',32,63,861,450,11824),("--+--",'S',28,46,1071,821,1600),
    ("---++",'B',39,56,1010,1033,4650),("---+-",'B',25,64,1071,1069,7525),("----+",'B',26,58,776,690,4050),("-----",'B',10,70,546,600,2025),
];
/// Table B printed p.69: as Table A but Day 2 (yesterday) had an expanding range relative to the day before.
static TABLE_B: &[(&str, char, u32, u32, u32, u32, i32)] = &[
    ("++",'S',193,59,803,721,34599),("-+",'B',185,55,808,742,19249),("+-",'S',179,49,862,692,11349),("--",'S',159,40,1510,891,14862),
    ("---",'S',63,40,2400,610,36799),("--+",'S',84,55,880,663,15325),("-+-",'S',83,52,1017,762,13249),("-++",'S',91,58,870,682,20174),
    ("+--",'B',93,60,1054,953,23774),("+-+",'B',97,62,910,572,33399),("++-",'B',91,54,634,729,450),("+++",'S',95,58,718,760,9074),
    ("++++",'B',41,51,907,842,2200),("+++-",'B',54,59,583,672,3875),("++-+",'B',47,66,791,775,12124),("++--",'B',41,66,1110,1117,14324),
    ("+-++",'S',58,59,691,638,8175),("+-+-",'S',54,61,1177,804,21949),("+--+",'S',42,52,1037,691,9000),("+---",'S',36,47,1408,586,13448),
    ("----",'S',23,30,5371,671,26849),("---+",'S',38,55,802,660,5624),("--+-",'B',29,66,715,490,8700),("--++",'S',31,58,1223,709,12799),
    ("-+++",'S',53,64,659,598,11050),("-++-",'S',37,54,791,729,3424),("-+-+",'B',47,57,992,427,18249),("-+--",'B',51,57,1002,854,10274),
    ("-----",'S',6,33,1587,518,1100),("----+",'S',15,60,911,758,3649),("---+-",'B',19,63,704,614,4150),("---++",'S',12,58,1375,965,4799),
    ("--+--",'B',23,65,831,993,4524),("--+-+",'B',24,54,867,502,5749),("--++-",'S',18,56,777,859,899),("--+++",'S',27,67,738,400,9700),
    ("-+---",'S',13,46,1008,410,3175),("-+--+",'S',21,71,1075,545,12850),("-+-+-",'S',28,57,979,935,4449),("-+-++",'S',26,58,735,618,4224),
    ("-++--",'B',19,74,1494,100,20425),("-++-+",'B',28,64,702,910,3550),("-+++-",'B',29,55,618,598,2125),("-++++",'B',16,63,940,770,4775),
    ("+++++",'S',24,58,873,925,2974),("++++-",'B',25,64,548,780,1750),("+++-+",'B',17,65,956,570,7099),("+++--",'S',22,41,1683,696,6100),
    ("++-++",'S',28,54,575,655,100),("++-+-",'S',26,65,1363,630,17500),("++--+",'B',21,67,753,957,3849),("++---",'S',25,40,1537,638,5799),
    ("+-+++",'S',25,60,601,777,1250),("+-++-",'S',19,53,805,613,2524),("+-+-+",'B',22,59,1167,336,12149),("+-+--",'B',27,52,1185,715,7300),
    ("+--++",'S',19,58,1127,550,8000),("+--+-",'B',10,70,735,200,4549),("+---+",'S',23,52,720,606,1975),("+----",'S',17,29,6885,722,25749),
];
/// S&C V.7:10 (1989) "Trading close-to-close patterns", Figure 1 (T-bond futures 1978-87): (pattern, B/S, trades).
/// Every sign is a close vs the close before it; entry on the last close of the pattern, exit on the next close.
static TABLE_CC: &[(&str, char, u32)] = &[
    ("++",'S',517), ("-+",'S',555), ("+-",'S',561), ("--",'B',564), ("+++",'S',239), ("++-",'S',272), ("+-+",'S',259), ("+--",'B',287), ("---",'B',267), ("--+",'S',288), ("-++",'S',262), ("-+-",'S',281), ("++++",'S',110), ("+++-",'S',127), ("++-+",'S',131), ("++--",'B',135), ("+-++",'B',123), ("+-+-",'S',131), ("+--+",'B',153), ("+---",'S',130), ("----",'B',131), ("---+",'S',131), ("--+-",'B',148), ("--++",'S',132), ("-+--",'S',146), ("-+-+",'B',126), ("-++-",'B',138), ("-+++",'S',122), ("+++++",'S',52), ("++++-",'S',58), ("+++-+",'S',59), ("+++--",'B',64), ("++-++",'B',62), ("++-+-",'S',64), ("++--+",'B',76), ("++---",'S',62), ("+-+++",'S',63), ("+-++-",'B',62), ("+-+-+",'S',53), ("+-+--",'B',77), ("+--++",'S',70), ("+--+-",'B',81), ("+---+",'S',55), ("+----",'B',71), ("-----",'B',57), ("----+",'S',73), ("---+-",'B',65), ("---++",'S',61), ("--+--",'S',67), ("--+-+",'B',72), ("--++-",'S',75), ("--+++",'S',53), ("-+---",'S',65), ("-+--+",'B',74), ("-+-+-",'S',66), ("-+-++",'B',60), ("-++--",'S',68), ("-++-+",'B',68), ("-+++-",'S',64), ("-++++",'B',55),
];
/// Table E printed p.72: range condition on Day 2 (0 control, 1 NR, 2 NR4, 3 RE, 4 RE4) x two-day pattern.
static TABLE_E: &[(u8, &str, char, u32, u32, u32, u32, i32)] = &[
    (2,"++",'B',114,48,1149,862,12354),(1,"++",'B',227,52,1023,871,25829),(0,"++",'S',427,53,842,894,12294),(3,"++",'S',193,59,803,721,34599),(4,"++",'S',95,61,818,793,15774),
    (2,"-+",'S',74,54,856,886,4124),(1,"-+",'B',186,53,689,691,8124),(0,"-+",'B',376,54,745,750,20024),(3,"-+",'B',185,55,808,742,19249),(4,"-+",'B',97,52,847,785,5425),
    (2,"+-",'B',101,51,1132,700,24550),(1,"+-",'B',188,52,919,640,30925),(0,"+-",'B',371,52,829,751,24625),(3,"+-",'S',179,49,862,692,11349),(4,"+-",'B',80,63,696,777,11500),
    (2,"--",'B',81,48,1059,942,1750),(1,"--",'B',163,56,859,840,17650),(0,"--",'B',326,57,873,1144,4249),(3,"--",'S',159,40,1510,891,14862),(4,"--",'S',86,36,1812,830,10524),
];
/// S&P gaps and ORB, printed pp.244-248: (gap pts, +1 up / -1 down, signed level, B/S, trades, win, avg win, avg loss, gross).
static GAPS: &[(f64, f64, f64, char, u32, u32, u32, u32, i32)] = &[
    (0.2,1.,1.6,'B',197,61,873,776,46674),(0.2,1.,0.8,'B',361,53,873,715,48400),(0.2,1.,-0.8,'S',394,50,817,676,29220),(0.2,1.,-1.6,'S',233,52,814,798,9120),
    (0.2,-1.,1.6,'B',205,50,1040,1040,574),(0.2,-1.,0.8,'B',345,57,845,1003,16225),(0.2,-1.,-0.8,'S',276,50,1187,911,35924),(0.2,-1.,-1.6,'S',169,49,1467,1070,27200),
    (0.4,1.,1.6,'B',142,58,887,822,23449),(0.4,1.,0.8,'B',249,51,919,721,27075),(0.4,1.,-0.8,'S',280,52,865,760,22795),(0.4,1.,-1.6,'S',180,49,881,840,1895),
    (0.4,-1.,1.6,'B',159,52,1139,1212,2399),(0.4,-1.,0.8,'B',251,59,953,1240,11125),(0.4,-1.,-0.8,'S',199,50,1372,1070,31274),(0.4,-1.,-1.6,'S',127,50,1677,1267,24575),
    (0.6,1.,1.6,'B',103,62,953,878,26749),(0.6,1.,0.8,'B',169,54,1022,764,33375),(0.6,1.,-0.8,'S',176,53,956,907,15570),(0.6,1.,-1.6,'S',125,51,973,966,3304),
    (0.6,-1.,1.6,'S',103,49,1572,1389,4975),(0.6,-1.,0.8,'B',152,56,1183,1458,2875),(0.6,-1.,-0.8,'S',127,49,1835,1200,35824),(0.6,-1.,-1.6,'S',89,48,2161,1467,25425),
    (0.8,1.,1.6,'B',74,61,963,927,16474),(0.8,1.,0.8,'B',116,53,1061,843,20250),(0.8,1.,-0.8,'S',118,56,1076,954,21419),(0.8,1.,-1.6,'S',89,48,1140,956,5020),
    (0.8,-1.,1.6,'S',82,46,1906,1591,2425),(0.8,-1.,0.8,'B',106,60,1430,2066,4775),(0.8,-1.,-0.8,'S',97,56,1981,1562,39850),(0.8,-1.,-1.6,'S',74,53,2265,1669,29925),
    (1.0,1.,1.6,'B',61,64,983,931,17849),(1.0,1.,0.8,'B',88,59,1106,881,25800),(1.0,1.,-0.8,'S',89,54,1177,1130,10170),(1.0,1.,-1.6,'S',68,49,1219,1100,1745),
    (1.0,-1.,1.6,'S',62,44,2314,1730,1950),(1.0,-1.,0.8,'S',80,43,2324,1663,2499),(1.0,-1.,-0.8,'S',78,54,2160,1757,27475),(1.0,-1.,-1.6,'S',62,47,2637,1744,18900),
    (1.5,1.,1.6,'B',26,50,1700,838,11199),(1.5,1.,0.8,'B',36,58,1401,1081,13200),(1.5,1.,-0.8,'S',42,55,1604,1616,6194),(1.5,1.,-1.6,'S',36,47,1697,1493,470),
    (1.5,-1.,1.6,'S',39,46,3073,2365,5650),(1.5,-1.,0.8,'S',51,49,2523,2264,4199),(1.5,-1.,-0.8,'S',48,54,2486,2295,14150),(1.5,-1.,-1.6,'S',38,53,2765,2563,9150),
    (2.0,1.,1.6,'B',16,56,2047,760,13099),(2.0,1.,0.8,'B',22,59,1767,1286,11400),(2.0,1.,-0.8,'B',24,58,1891,2362,2855),(2.0,1.,-1.6,'B',20,65,1833,2867,3754),
    (2.0,-1.,1.6,'S',27,44,4045,2461,10125),(2.0,-1.,0.8,'S',35,49,3339,2493,11899),(2.0,-1.,-0.8,'S',34,56,3144,2896,16299),(2.0,-1.,-1.6,'S',27,63,3086,3790,14575),
    (2.5,1.,1.6,'B',11,64,2496,1187,12724),(2.5,1.,0.8,'B',15,53,2575,1317,11375),(2.5,1.,-0.8,'B',18,61,2080,2010,8805),(2.5,1.,-1.6,'B',16,69,2014,2320,10554),
    (2.5,-1.,1.6,'S',17,53,4991,3828,14300),(2.5,-1.,0.8,'S',23,57,4142,3417,19674),(2.5,-1.,-0.8,'S',27,59,3510,3565,16950),(2.5,-1.,-1.6,'S',24,62,3338,4119,13000),
    (3.0,1.,1.6,'B',10,60,2862,1187,12424),(3.0,1.,0.8,'B',14,50,2842,1317,10675),(3.0,1.,-0.8,'B',16,63,2140,2025,9255),(3.0,1.,-1.6,'B',14,71,2028,2518,10204),
    (3.0,-1.,1.6,'S',12,58,5975,5890,12375),(3.0,-1.,0.8,'S',15,60,5372,5258,16800),(3.0,-1.,-0.8,'S',20,55,4193,4111,9125),(3.0,-1.,-1.6,'S',18,56,4202,4503,6000),
    (4.0,1.,1.6,'B',6,67,3850,1862,11674),(4.0,1.,0.8,'B',9,44,4250,1685,8575),(4.0,1.,-0.8,'B',8,50,4237,2918,5275),(4.0,1.,-1.6,'B',7,43,4866,2518,4525),
    (4.0,-1.,1.6,'S',9,67,6804,8616,14975),(4.0,-1.,0.8,'S',10,60,7204,6787,16075),(4.0,-1.,-0.8,'S',12,58,5796,6125,9950),(4.0,-1.,-1.6,'S',12,50,6345,5487,5150),
];
/// Tri-segment (3 parts) S&P table printed p.238 and 5-segment ("quadrant") S&P tables printed pp.231-232:
/// (segment of yesterday's open, segment of yesterday's close, signed level, B/S, trades, win, avg win, avg loss, gross); 1 = bottom.
type Seg = (u8, u8, f64, char, u32, u32, u32, u32, i32);
static TRISEG: &[Seg] = &[
    (3,3,0.8,'S',41,49,838,592,4324),(3,3,-0.8,'S',41,61,1177,628,19350),(3,2,0.8,'S',115,57,872,723,22125),(3,2,-0.8,'S',83,45,1181,690,11950),
    (3,1,0.8,'B',186,53,854,952,1675),(3,1,-0.8,'S',200,43,1099,723,10275),(2,3,0.8,'B',98,54,815,655,13725),(2,3,-0.8,'S',111,56,627,621,8470),
    (2,2,0.8,'B',88,61,845,736,20600),(2,2,-0.8,'B',87,51,832,841,474),(2,1,0.8,'B',86,57,758,511,18250),(2,1,-0.8,'S',83,46,959,626,8250),
    (1,3,0.8,'B',235,52,896,866,13175),(1,3,-0.8,'B',223,49,875,796,4599),(1,2,0.8,'B',93,56,611,621,6300),(1,2,-0.8,'S',83,52,733,631,6300),
    (1,1,0.8,'B',31,74,560,678,7475),(1,1,-0.8,'B',36,58,570,610,2825),
];
static QUAD: &[Seg] = &[
    (5,5,0.8,'B',6,67,1206,350,4125),(5,5,-0.8,'S',8,50,925,600,925),(5,4,0.8,'S',26,46,983,612,3224),(5,4,-0.8,'S',21,43,1291,466,6025),
    (5,3,0.8,'B',52,54,808,873,1675),(5,3,-0.8,'S',39,46,1404,800,8474),(5,2,0.8,'S',56,43,1943,1020,14000),(5,2,-0.8,'S',56,46,2146,695,34950),
    (5,1,0.8,'B',97,54,693,546,11499),(5,1,-0.8,'B',114,57,665,780,4974),(4,5,0.8,'S',25,56,739,402,5924),(4,5,-0.8,'S',30,53,815,537,5525),
    (4,4,0.8,'B',29,59,695,714,3250),(4,4,-0.8,'S',22,59,767,491,5550),(4,3,0.8,'B',35,63,1188,748,16425),(4,3,-0.8,'B',28,43,1497,787,5374),
    (4,2,0.8,'B',45,47,670,542,1050),(4,2,-0.8,'B',34,53,623,520,2900),(4,1,0.8,'B',43,56,1018,511,14725),(4,1,-0.8,'B',39,59,882,542,11624),
    (3,5,0.8,'B',42,55,865,586,8750),(3,5,-0.8,'S',46,70,717,500,15975),(3,4,0.8,'B',29,69,785,600,10300),(3,4,-0.8,'B',22,59,555,444,3225),
    (3,3,0.8,'S',37,49,894,788,1124),(3,3,-0.8,'S',41,56,946,444,13774),(3,2,0.8,'B',36,58,790,496,9150),(3,2,-0.8,'B',37,54,721,469,6450),
    (3,1,0.8,'B',27,63,711,382,8275),(3,1,-0.8,'S',27,44,1893,461,15800),(2,5,0.8,'B',58,53,1275,502,25975),(2,5,-0.8,'B',50,52,1654,517,30579),
    (2,4,0.8,'B',33,67,710,1059,3975),(2,4,-0.8,'S',39,62,878,543,12925),(2,3,0.8,'B',21,52,679,700,475),(2,3,-0.8,'S',25,60,985,437,10400),
    (2,2,0.8,'B',23,61,653,950,600),(2,2,-0.8,'S',22,59,1021,580,8049),(2,1,0.8,'S',14,36,775,347,750),(2,1,-0.8,'B',19,53,440,391,874),
    (1,5,0.8,'S',138,53,932,783,17149),(1,5,-0.8,'S',134,57,830,659,24874),(1,4,0.8,'B',55,58,778,822,6000),(1,4,-0.8,'B',57,49,799,602,4900),
    (1,3,0.8,'B',41,59,582,497,5525),(1,3,-0.8,'S',34,68,518,909,1925),(1,2,0.8,'B',22,64,605,406,5225),(1,2,-0.8,'B',15,67,855,705,5024),
    (1,1,0.8,'B',9,78,860,925,4175),(1,1,-0.8,'S',13,31,993,266,1574),
];

// ---------------------------------------------------------------- rules text
const NOSTOP: &str = "X|\"Stops were not used and if the market traded both above and below the opening range the given amount, a buy and a sell could be registered.\" (printed p.123) Each level is its own test.";
const ORB_E: &str = "E|ORB: \"A trade taken with entry at a predetermined level above or below the open\" (printed p.139). Buy (sell) on a stop/limit at today's open ± the level when the day's range reaches it; fill at the level. B/S as printed (\"determined by Gross Profits\").";
const ORB_X: &str = "X|\"Exit on the close of the same day of entry\" (printed p.123); for the 2/3 Bar NR tables the close 1, 2 or 5 days after entry (\"No. of days in trade\", printed pp.173, 186).";
const LVL: &str = "F|Levels are S&P 500 futures points as printed (80 points = 0.80 index points; printed p.17: open 271.00 to 272.90 = \"190 points\"). level_mode = 1 [labelled option, not the tested default]: use the book's Chapter 1 Stretch = average over the previous 10 days of |open − the extreme closest to the open| (printed p.1, p.286) × stretch_mult.";
const R_CTL: &[&str] = &["F|Control group: any day (\"made no reference to range size\", printed pp.20, 105)", ORB_E, ORB_X, NOSTOP, LVL];
const R_ID: &[&str] = &["F|Inside Day (Day 2): \"the Inside Day's high is less than the previous day's high and the Inside Day's low is greater than the previous day's low\" (printed p.123)", ORB_E, ORB_X, NOSTOP, LVL];
const R_DOJI: &[&str] = &["F|Doji (Day 2): \"The difference between the open and close of yesterday is less than\" the doji value (\"50 Doji\" = 50 points = 0.50; printed p.132, p.139)", ORB_E, ORB_X, NOSTOP, LVL];
const R_NR4: &[&str] = &["F|NR4 (Day 2): \"a daily range that is narrower than the previous three days' daily ranges compared individually\" (printed p.105, p.285)", ORB_E, ORB_X, NOSTOP, LVL];
const R_DNR4: &[&str] = &["F|Day 2 is both a Doji (open−close < 50 pts) and an NR4 (\"a day with a narrower daily range than any of the previous 3 day's daily ranges\", printed p.149)", ORB_E, ORB_X, NOSTOP, LVL];
const R_IDNR4: &[&str] = &["F|Day 2 is both an Inside Day and an NR4 (printed p.153)", ORB_E, ORB_X, NOSTOP, LVL];
const R_NR7: &[&str] = &["F|NR7 (Day 2): \"a daily range that is narrower than the previous six day's daily ranges compared individually\" (printed p.106, p.112)", ORB_E, ORB_X, NOSTOP, LVL];
const R_WS4: &[&str] = &["F|WS4 (Day 2): \"a day with a daily range that is larger than any of the previous three day's ranges\" (printed p.109)", ORB_E, ORB_X, NOSTOP, LVL];
const R_WS7: &[&str] = &["F|WS7 (Day 2): \"a day with a daily range that is larger than any of the previous six day's ranges\" (printed p.110)", ORB_E, ORB_X, NOSTOP, LVL];
const R_BEAR: &[&str] = &["F|Bear Hook (Day 2): \"a day in which the open is below the previous day's low and the close is above the previous day's close with a narrow range relative to the previous day\" (printed p.271)", ORB_E, ORB_X, NOSTOP, LVL];
const R_BULL: &[&str] = &["F|Bull Hook (Day 2): \"a higher open than the previous day's high followed by a lower close with a narrowing daily range\"; glossary: open above the previous day's high, close below the previous day's close, narrow range (printed pp.279, 284)", ORB_E, ORB_X, NOSTOP, LVL];
const R_DR1: &[&str] = &["F|\"50 Doji\": yesterday's open−close difference less than 50 points; \"R1 less than 200\": yesterday's range less than 200 points (printed p.132, Table B p.135)", ORB_E, ORB_X, NOSTOP, LVL];
const R_2BNR: &[&str] = &["F|2 Bar NR (Day 2 = last day of the pair): \"the narrowest range from high to low of any two day period relative to any two day period within the previous twenty market days\" (printed p.166, p.285). Compared strictly with every two-day window lying inside the previous 20 days.", ORB_E, ORB_X, NOSTOP, LVL];
const R_3BNR: &[&str] = &["F|3 Bar NR (Day 2 = last day of the three): \"the narrowest range from high to low of any three day period relative to any three day period within the previous 20 market days\" (printed p.180, p.285). Compared strictly with every three-day window inside the previous 20 days.", ORB_E, ORB_X, NOSTOP, LVL];
const R_OC: &[&str] = &["F|Table A (printed p.68; 5-day rows also p.52): pattern of closes 2-5 days back (+ up close, − down close vs the prior day) and today's open vs yesterday's close (last sign). row = the book's pattern number.",
    "E|\"A trade is taken on Day 1's open and exited on the close of Day 1\" (printed p.67). B/S as printed for the row (side = 0); side = ±1 overrides.", "X|Exit on the close of Day 1. No stops (printed p.36)."];
const R_OCRE: &[&str] = &["F|Table B (printed p.69): \"same as Table A except Day 2 had an expanding daily range relative to the previous day\" (printed p.67). row = the book's pattern number.",
    "E|Buy/sell on Day 1's open as printed (side = 0); side = ±1 overrides.", "X|Exit on the close of Day 1."];
const R_CC: &[&str] = &["F|Figure 1 of Crabel, \"Trading close-to-close patterns\" (Stocks & Commodities V.7:10, 1989, p.372-374): all two- to five-day patterns of closes, + = a higher close and - = a lower close than the close before; row = the figure's pattern number (1-60).",
    "E|Enter on the close of the last day of the pattern as printed (B buy, S sell; side = ±1 overrides).", "X|Exit on the next day's close (\"exited on the next day's close. Stops were not used\")."];
const R_OCNR: &[&str] = &["F|Table E (printed p.72): yesterday's close direction and today's open direction, with yesterday being NR4 / NR (range below the day before) / control / RE (range above the day before) / RE4 (range above each of the previous three).",
    "E|Entry on today's open, B/S as printed (side = 0).", "X|Exit on today's close."];
const R_GAP: &[&str] = &["F|S&P Gaps and ORB (printed pp.243-248): today's open at least `gap` points above (gap_dir +1) or below (−1) yesterday's close. Gaps of 20, 40, 60, 80, 100, 150, 200, 250, 300, 400 points. The tables' trade counts prove the gap classes are cumulative (\"at least\"), not buckets: the +80 rows sum to 2,147 trades, more than the ~1,700 days of 1982-88.",
    "E|ORB of 80 or 160 points above / below the open, B/S as printed for the row.", "X|Exit on today's close (\"exit on the close\" as in all ORB tables).", LVL];
const R_TRI: &[&str] = &["F|Tri-segment study (printed p.235, S&P table p.238): yesterday's range divided into 3 equal segments (1 bottom, 3 top); the segments of yesterday's open and close select the row.",
    "E|\"The trade was then taken at the indicated points off the open\" (± 80 points), B/S as printed.", "X|\"exited on the close of the same day\" (printed p.235).", LVL];
const R_QUAD: &[&str] = &["F|5 Segment study (printed p.227, S&P tables pp.231-232): yesterday's range divided \"into 5 equal segments of price\", 1 = bottom; the segments of yesterday's open and close select the row.",
    "E|ORB taken on a move above and below the open (± 80 points), B/S as printed.", "X|Exit on the close of the same day.", LVL];
const R_P1: &[&str] = &["F|\"Pattern 1: an inside day followed by a sale(s) on a lower open or buy (b) on a higher open\" (printed p.115)", "E|\"Entry is on the open\"", "X|\"with an exit on the same day's close with no stop\""];
const R_P2: &[&str] = &["F|\"Pattern 2a is defined as an ID with a higher close than the previous day followed by a higher open\" (buy); \"Pattern 2b ... an ID with a lower close followed by a lower open\" (sale) (printed p.115)", "E|Entry on the open", "X|Exit on the close; \"stops were not used\""];
const R_P3: &[&str] = &["F|\"On Day 1 an open lower than Day 2's mid-range and lower than Day 2's close is necessary. Day 2 must be inside of Day 3. Day 3 must have a higher low than Day 4.\" (printed p.115)", "E|\"A sale is made on the open of Day 1\"", "X|\"with exit on the close of Day 1\""];
const R_P4: &[&str] = &["F|\"Pattern 4 is similar to Pattern 3 with opposite parameters. The only exception is the open on Day 1 need only be higher, not above mid-range\": Day 1 open higher than Day 2's close, Day 2 inside Day 3, Day 3 lower high than Day 4 (printed p.115)", "E|Buy on the open of Day 1", "X|Exit on the close of Day 1; no stops"];
const R_UT: &[&str] = &["F|Upthrust (printed p.261, p.287): \"a price move above a pivot top followed by a wide spread reversal to the downside: 1) closes below the two previous days' closings 2) close below the pivot top 3) close below the opening and the mid-range of the day 4) daily range greater than the previous day's range\". Pivot top: \"a day in which the high is above the high of the preceding day and the high of the following day\" (printed p.284).",
    "F|Diagram 1-5: the pivot top is on Day 3 / 4 / 5 / 6 / 7 (Day 1 = upthrust day); intervening days are unconstrained (\"Day 2 could have been above the pivot also\", \"Day 2 and 3 allowed to vary anywhere\"). Day 1's high exceeds the pivot high.",
    "E|\"The test was for a short sale taken on the close of Day 1\"", "X|\"exited three days later on the close\"; \"Stops were not used\""];

const PER: &str = "S&P 500 futures 1982-88 (book); our check: SPX cash index 1983-88";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let mut v: Vec<StrategyDef> = ORBS.iter().map(|o| {
        let (group, rules): (&'static str, &'static [&'static str]) = match o.pat {
            Pat::Any => ("Ch 3 · Moves away from the open (control)", R_CTL), Pat::Id => ("Ch 17 · Inside Day & ORB", R_ID),
            Pat::Doji => if o.id.starts_with("tcdt_dr1") { ("Ch 18 · Doji Lines & NR", R_DR1) } else { ("Ch 19 · Doji & ORB", R_DOJI) },
            Pat::Nr4 => ("Ch 20 · NR4 & ORB", R_NR4), Pat::DojiNr4 => ("Ch 21 · Doji & NR4 with ORB", R_DNR4),
            Pat::IdNr4 => ("Ch 22 · ID/NR4 & ORB", R_IDNR4), Pat::Nr7 => ("Ch 23 · NR7 & ORB", R_NR7),
            Pat::Ws4 => ("Ch 15 · Contraction/Expansion (WS4)", R_WS4), Pat::Ws7 => ("Ch 15 · Contraction/Expansion (WS7)", R_WS7),
            Pat::BearHook => ("Bear Hook days & ORB", R_BEAR), Pat::BullHook => ("Bull Hook days & ORB", R_BULL),
            Pat::DojiR1 | Pat::R1 => ("Ch 18 · Doji Lines & NR", R_DR1), Pat::Bnr2 => ("Ch 24 · 2 Bar NR & ORB", R_2BNR), Pat::Bnr3 => ("Ch 25 · 3 Bar NR & ORB", R_3BNR),
        };
        let dir = if o.side > 0.0 { "Long" } else { "Short" };
        let claim = Some(Claim { trades: if o.t.is_empty() { "not printed" } else { o.t }, win: o.w,
            avg: if o.t.is_empty() { "% winners only (printed pp.20, 108)" } else { o.avg },
            hold: if o.hold == 0.0 { "exit same-day close" } else if o.hold == 1.0 { "exit close 1 day after entry" } else if o.hold == 2.0 { "exit close 2 days after entry" } else { "exit close 5 days after entry" },
            period: PER });
        d(o.id, o.name, group, dir, Status::Live, rules,
          if o.hold == 0.0 { "Book: S&P 500 futures, daily OHLC, 1982-88, one contract, no stops, no slippage or commission; avg win/loss in $ per contract ($500 per index point). Run here on cash indices/ETFs (vault convention); the cash S&P open before ~1990 is often yesterday's close, so 1983-88 SPX counts differ from the futures. Default level = the printed S&P points (instrument swap only); on NIFTY these points are tiny relative to price, see level_mode. Overlapping multi-day trades are each counted, as the book's equal trade counts across holding periods show. Intraday wave 1: run on real 1-minute bars by default (p.path=1): fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book]." } else { "Book: S&P 500 futures, daily OHLC, 1982-88, one contract, no stops, no slippage or commission; avg win/loss in $ per contract ($500 per index point). Run here on cash indices/ETFs (vault convention); the cash S&P open before ~1990 is often yesterday's close, so 1983-88 SPX counts differ from the futures. Default level = the printed S&P points (instrument swap only); on NIFTY these points are tiny relative to price, see level_mode. Overlapping multi-day trades are each counted, as the book's equal trade counts across holding periods show." },
          claim)
    }).collect();
    v.push(d("tcdt_oc_pattern", "Open-to-close patterns (Table A)", "Ch 8/11 · Price Pattern Studies IV · S&P open to close", "Long/Short", Status::Live, R_OC,
        "60 printed S&P rows (2- to 5-day patterns); param row = book pattern number (default 1 = \"++\" S, 427 trades, 53%). Equal closes/opens match neither + nor − [book silent]. The text example on p.51 calls pattern 30 a sale while the table marks it B; the table is used.",
        Some(Claim { trades: "427 (row 1)", win: "53%", avg: "avg win $842 / avg loss $894, total $12,294 (row 1)", hold: "exit same-day close", period: PER })));
    v.push(d("tcdt_oc_rangeexp", "Range expansion & open-to-close (Table B)", "Ch 11 · Range expansion S&P", "Long/Short", Status::Live, R_OCRE,
        "60 printed S&P rows; param row = book pattern number (default 1 = \"++\" S, 193 trades, 59%).",
        Some(Claim { trades: "193 (row 1)", win: "59%", avg: "avg win $803 / avg loss $721, total $34,599 (row 1)", hold: "exit same-day close", period: PER })));
    v.push(d("tcdt_cc_pattern", "Close-to-close patterns (S&C 1989, Figure 1)", "S&C article · T-bond close to close", "Long/Short", Status::Live, R_CC,
        "60 printed T-bond rows (1978-87); param row = the figure's pattern number (default 1 = \"++\" S, 517 trades, 52%). Equal closes match neither + nor - [article silent]. The article tested T-bond futures only; index, ETF and stock runs are a declared application.",
        Some(Claim { trades: "517 (row 1)", win: "52%", avg: "gross profit $12,791 (row 1)", hold: "exit next day's close", period: "T-bond futures 1978-87" })));
    v.push(d("tcdt_oc_nr_re", "NR4 / NR / RE / RE4 two-day patterns (Table E)", "Ch 11 · Range expansion S&P", "Long/Short", Status::Live, R_OCNR,
        "20 printed S&P rows in book order (NR4, NR, Control, RE, RE4 for ++, −+, +−, −−); default row 1 = NR4 \"++\" B.",
        Some(Claim { trades: "114 (row 1)", win: "48%", avg: "avg win $1149 / avg loss $862, gross $12,354 (row 1)", hold: "exit same-day close", period: PER })));
    v.push(d("tcdt_gap_orb", "S&P Gaps & ORB", "S&P Gaps and Opening Range Breakout", "Long/Short", Status::Live, R_GAP,
        "80 printed rows: row = 8 × gap class (20,40,60,80,100,150,200,250,300,400) + 4 if gap down + level (0:+160, 1:+80, 2:−80, 3:−160) + 1. Default row 1 = gap up ≥ 20, open +160, buy. Intraday wave 1: run on real 1-minute bars by default (p.path=1): fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].",
        Some(Claim { trades: "197 (row 1)", win: "61%", avg: "avg win $873 / avg loss $776, gross $46,674 (row 1)", hold: "exit same-day close", period: PER })));
    v.push(d("tcdt_triseg_orb", "Tri-segment open/close & ORB", "Open to close tri-segment studies", "Long/Short", Status::Live, R_TRI,
        "18 printed S&P rows (O/3 C/3 +80, O/3 C/3 −80, O/3 C/2 +80 ...). Prices exactly on a boundary go to the upper segment; the top value belongs to the top segment [book silent]. Intraday wave 1: run on real 1-minute bars by default (p.path=1): fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].",
        Some(Claim { trades: "41 (row 1)", win: "49%", avg: "avg win $838 / avg loss $592, gross $4,324 (row 1)", hold: "exit same-day close", period: PER })));
    v.push(d("tcdt_quad_orb", "5-segment open/close & ORB", "5 segment study and ORB", "Long/Short", Status::Live, R_QUAD,
        "50 printed S&P rows (O/5 C/5 +80, O/5 C/5 −80, O/5 C/4 +80 ...). Boundary convention as tri-segment [book silent]. Intraday wave 1: run on real 1-minute bars by default (p.path=1): fills on the first minute that reaches the level, at the level or at that minute's open when it opened through it; out on the official daily close. Instrument: SPX only, because the levels are the book's fixed S&P points (no conversion to SPY or NSE prices is invented); from 2014, where SPX daily opens are real, to 2026-08. Days without minute data make no trade. p.path=0 = the old daily-bar reading [Not from the book].",
        Some(Claim { trades: "6 (row 1)", win: "67%", avg: "avg win $1,206 / avg loss $350, gross $4,125 (row 1)", hold: "exit same-day close", period: PER })));
    let idnote = "Book: S&P futures 1982-87 daily OHLC, entry on the open, exit on the close, no stop. Higher/lower open = vs yesterday's (Day 2's) close (the book's open-sign convention, printed p.35).";
    v.push(d("tcdt_idp1", "Inside Day pattern 1 · open direction", "Ch 16 · Inside Days in the S&P", "Long/Short", Status::Live, R_P1, idnote,
        Some(Claim { trades: "not printed", win: "68%", avg: "profits of $18,000 after $18 commission", hold: "exit same-day close", period: "S&P futures 1982-87" })));
    v.push(d("tcdt_idp2", "Inside Day pattern 2a/2b · close & open agree", "Ch 16 · Inside Days in the S&P", "Long/Short", Status::Live, R_P2, idnote,
        Some(Claim { trades: "44", win: "74%", avg: "net profit $14,914", hold: "exit same-day close", period: "S&P futures 1982-87" })));
    v.push(d("tcdt_idp3", "Inside Day pattern 3 · sale", "Ch 16 · Inside Days in the S&P", "Short", Status::Live, R_P3, idnote,
        Some(Claim { trades: "10", win: "80%", avg: "winners five times the size of losers", hold: "exit same-day close", period: "S&P futures 1982-87" })));
    v.push(d("tcdt_idp4", "Inside Day pattern 4 · buy", "Ch 16 · Inside Days in the S&P", "Long", Status::Live, R_P4, idnote,
        Some(Claim { trades: "11", win: "91%", avg: "avg winner $860 / avg loser $820", hold: "exit same-day close", period: "S&P futures 1982-87" })));
    let utn = "Book tested T-Bond futures 1978-87 only (no S&P result); coded for index use as an instrument swap. Signals within 3 days of each other are each counted (occurrences).";
    for (id, name, t, w, a) in [
        ("tcdt_ut1", "Upthrust · pivot on Day 3", "8", "87%", "avg winner $940 / avg loser $625"),
        ("tcdt_ut2", "Upthrust · pivot on Day 4", "11", "55%", "avg winner $869 / avg loser $587"),
        ("tcdt_ut3", "Upthrust · pivot on Day 5", "17", "59%", "avg winner $906 / avg loser $941"),
        ("tcdt_ut4", "Upthrust · pivot on Day 6", "20", "70%", "avg winner $1,081 / avg loser $1,192"),
        ("tcdt_ut5", "Upthrust · pivot on Day 7", "14", "36%", "avg win $918 / avg loss $805 (the long side won 64%)"),
    ] {
        v.push(d(id, name, "Upthrust and Spring reversals", "Short", Status::Live, R_UT, utn,
            Some(Claim { trades: t, win: w, avg: a, hold: "short on Day 1 close, exit close 3 days later", period: "T-Bond futures 1978-87" })));
    }
    // Seen, not coded: reasons are the book's own text.
    v.push(d("tcdt_orb_stretch", "ORB I · Stretch with protective opposite stop", "Ch 1 · Opening Range Breakout I", "Long/Short",
        Status::NotTestable("no exit rule is stated: the position is managed by moving stops to break even within an hour and trailing them, for a possible 2-3 day run \"only if a substantial profit is realized by the end of the session\" (printed pp.2, 6) - judgement; the opening range is the first 30 seconds (needs tick data)"),
        &["E|\"a buy stop is placed that amount [the Stretch] above the high of the opening range and a sell stop the same amount below the low of the opening range. The first stop that is traded is the position and the other stop is used as a protective stop.\" (printed p.1)",
          "F|\"The Stretch is determined by looking at the previous ten days and averaging the sum of the differences between the open for each day and the closest extreme to the open on each day\"",
          "X|Not stated mechanically (break-even within one hour, trailing stops, 3/8-1/2 retracement re-entries: printed p.6)"],
        "The Stretch itself is available as level_mode = 1 on every coded ORB test.", None));
    v.push(d("tcdt_orbp", "ORB Preference (one direction only)", "Ch 1 · Opening Range Breakout I", "Long/Short",
        Status::NotTestable("the direction comes from the trader's \"clear bias\" - judgement"), &["E|\"only the order entered is the stop in the direction of the entry\" when a bias exists (printed pp.1, 284)"], "", None));
    v.push(d("tcdt_early_entry", "Early Entry type 1 / type 2", "Ch 2 · ORB II Early Entry", "Long/Short",
        Status::NotTestable("needs the first five-minute bar of each session (we have only first-15-minute NIFTY series) and the trader's judgement of \"ideal action\" within 5-10 minutes; no exit rule"),
        &["F|EE1: first five-minute range larger than the average of the preceding ten days' first five-minute ranges, open at one extreme, close at the other, second five minutes an equal thrust (printed pp.9-14, 283)",
          "F|EE2: first five-minute range bigger than the previous twenty days' first five-minute periods"], "", None));
    v.push(d("tcdt_id_retrace", "Inside Day patterns 5-8 · counter move to yesterday's close", "Ch 16 · Inside Days in the S&P", "Long/Short",
        Status::NotTestable("the book gives only the chance of closing beyond Day 2's close (62/69/59/67%); entry is on a counter move \"observed for a loss of momentum\" - judgement"), &["F|Patterns 5-8 (printed p.116)"], "", None));
    v.push(d("tcdt_200pt_move", "200-point move off the open", "Ch 16 · Inside Days in the S&P", "Long/Short",
        Status::NotTestable("\"enter in the direction of the initial trend on any low momentum move back to the open\" - momentum judgement; only the 90%/88% statistic is printed"), &["F|S&P 1982-88: after a 200-point move above (below) the open the market closed above (below) the open 90% (88%) of the time (printed p.116)"], "", None));
    v.push(d("tcdt_ce_patterns", "Two/three-day patterns with Contraction/Expansion & ORB; daily bias (Gold)", "Ch 26-28", "Long/Short",
        Status::NotTestable("the book prints these only for T-Bonds, Yen, D-Mark, Gold and Beans with entry levels in tics/cents; no S&P/index level is given, so an index version would need an invented level. The building blocks (NR/NR4/NR7/WS/WS4/WS7 days, 2-day patterns, ORB) are coded in the S&P tests above"),
        &["F|NR7/NR4/NR/control/WS/WS4/WS7 on Day 2 × yesterday's close direction × today's open direction, ORB off the open (printed pp.191-218)"], "", None));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("tcdt_") { return None; }
    let fam = |f: &'static str, n: &'static str| (f, n);
    let (f, n) = match id.split('_').nth(1)? {
        "ctl" => fam("tcdt_ctl", "Crabel · Control ORB (any day)"), "id" => fam("tcdt_id", "Crabel · Inside Day ORB"),
        "doji" => fam("tcdt_doji", "Crabel · Doji ORB"), "nr4" => fam("tcdt_nr4", "Crabel · NR4 ORB"),
        "dojinr4" => fam("tcdt_dojinr4", "Crabel · Doji & NR4 ORB"), "idnr4" => fam("tcdt_idnr4", "Crabel · ID/NR4 ORB"),
        "nr7" => fam("tcdt_nr7", "Crabel · NR7 ORB"), "ws4" => fam("tcdt_ws4", "Crabel · WS4 ORB"), "ws7" => fam("tcdt_ws7", "Crabel · WS7 ORB"),
        "bearhook" => fam("tcdt_bearhook", "Crabel · Bear Hook ORB"), "bullhook" => fam("tcdt_bullhook", "Crabel · Bull Hook ORB"),
        "dr1" => fam("tcdt_dr1", "Crabel · Doji Lines & NR"), "2bnr" => fam("tcdt_2bnr", "Crabel · 2 Bar NR ORB"), "3bnr" => fam("tcdt_3bnr", "Crabel · 3 Bar NR ORB"),
        "oc" => fam("tcdt_oc", "Crabel · Open-to-close pattern tables"), "cc" => fam("tcdt_cc", "Crabel · Close-to-close patterns (S&C)"), "gap" | "triseg" | "quad" => fam("tcdt_openloc", "Crabel · Gap / segment ORB tables"),
        "idp1" | "idp2" | "idp3" | "idp4" | "id_retrace" | "200pt" => fam("tcdt_idpat", "Crabel · Inside Day open patterns"),
        "ut1" | "ut2" | "ut3" | "ut4" | "ut5" => fam("tcdt_ut", "Crabel · Upthrust"),
        _ => fam("tcdt_other", "Crabel · ORB concepts (not testable)"),
    };
    let label = ORBS.iter().find(|o| o.id == id).map(|o| o.name).or_else(|| catalogue_name(id)).unwrap_or("");
    Some((f, n, label))
}
fn catalogue_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "tcdt_oc_pattern" => "Table A", "tcdt_cc_pattern" => "Close-to-close (S&C 1989)", "tcdt_oc_rangeexp" => "Table B (range expansion)", "tcdt_oc_nr_re" => "Table E (NR4/NR/RE/RE4)",
        "tcdt_gap_orb" => "Gaps", "tcdt_triseg_orb" => "Tri-segment", "tcdt_quad_orb" => "5-segment",
        "tcdt_idp1" => "Pattern 1", "tcdt_idp2" => "Pattern 2a/2b", "tcdt_idp3" => "Pattern 3", "tcdt_idp4" => "Pattern 4",
        "tcdt_ut1" => "Pivot Day 3", "tcdt_ut2" => "Pivot Day 4", "tcdt_ut3" => "Pivot Day 5", "tcdt_ut4" => "Pivot Day 6", "tcdt_ut5" => "Pivot Day 7",
        _ => return None,
    })
}

fn level_params(v: &mut Vec<ParamDef>) {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    v.push(p("level_mode", "Level: 0 = printed S&P points (book tests) · 1 = Stretch × mult (book Ch 1, option)", 0., 0., 1., 1.));
    v.push(p("stretch_mult", "Stretch multiple (level_mode 1)", 1., 0.25, 4., 0.25));
    v.push(p("stretch_n", "Stretch lookback days (book 10)", 10., 3., 30., 1.));
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if let Some(o) = ORBS.iter().find(|o| o.id == id) {
        let mut v = vec![
            p("pts", "Entry level off the open (index points; book S&P value)", o.lvl.abs(), 0., 500., 0.05),
            p("side", "Side: 1 buy, −1 sell (book's printed B/S)", o.side, -1., 1., 2.),
            p("hold", "Exit: close N days after the entry day (0 = same day)", o.hold, 0., 10., 1.),
        ];
        match o.pat {
            Pat::Doji | Pat::DojiNr4 | Pat::DojiR1 => v.push(p("doji", "Doji: yesterday's |open − close| below (points)", 0.50, 0., 50., 0.05)),
            _ => {}
        }
        if matches!(o.pat, Pat::DojiR1 | Pat::R1) { v.push(p("r1", "R1: yesterday's range below (points)", 2.00, 0., 200., 0.1)); }
        if matches!(o.pat, Pat::Bnr2 | Pat::Bnr3) { v.push(p("look", "Narrowest within the previous N market days", 20., 10., 40., 1.)); }
        level_params(&mut v);
        if o.hold == 0.0 { v.push(p("path", "Same-day ORB fill: 1 = real 1-minute bars (fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] daily bars (fill at the level)", 1., 0., 1., 1.)); } // intraday wave 1
        return Some(v);
    }
    let rows = |n: f64| vec![p("row", "Book table row", 1., 1., n, 1.), p("side", "Side: 0 = as printed, 1 buy, −1 sell", 0., -1., 1., 1.)];
    Some(match id {
        "tcdt_oc_pattern" | "tcdt_oc_rangeexp" | "tcdt_cc_pattern" => rows(60.),
        "tcdt_oc_nr_re" => rows(20.),
        "tcdt_gap_orb" | "tcdt_triseg_orb" | "tcdt_quad_orb" => {
            let mut v = rows(match id { "tcdt_gap_orb" => 80., "tcdt_triseg_orb" => 18., _ => 50. });
            level_params(&mut v);
            v.push(p("path", "Same-day ORB fill: 1 = real 1-minute bars (fills at the level or the minute's open; days without minute data make no trade), 0 = [Not from the book] daily bars (fill at the level)", 1., 0., 1., 1.)); // intraday wave 1
            v
        }
        "tcdt_idp1" | "tcdt_idp2" | "tcdt_idp3" | "tcdt_idp4" => vec![],
        "tcdt_ut1" | "tcdt_ut2" | "tcdt_ut3" | "tcdt_ut4" | "tcdt_ut5" => vec![p("hold", "Exit: close N days after the upthrust day", 3., 1., 10., 1.)],
        _ => return None,
    })
}

// ---------------------------------------------------------------- pattern helpers (all on bar j = Day 2 unless noted)
fn rng(x: &Ctx, j: usize) -> f64 { x.h[j] - x.l[j] }
fn inside(x: &Ctx, j: usize) -> bool { j >= 1 && x.h[j] < x.h[j - 1] && x.l[j] > x.l[j - 1] }
fn narrower(x: &Ctx, j: usize, n: usize) -> bool { j >= n && (1..=n).all(|k| rng(x, j) < rng(x, j - k)) }
fn wider(x: &Ctx, j: usize, n: usize) -> bool { j >= n && (1..=n).all(|k| rng(x, j) > rng(x, j - k)) }
fn nbar_range(x: &Ctx, end: usize, n: usize) -> f64 {
    let s = end + 1 - n;
    x.h[s..=end].iter().cloned().fold(f64::MIN, f64::max) - x.l[s..=end].iter().cloned().fold(f64::MAX, f64::min)
}
/// Narrowest n-day range ending at j vs every n-day window lying inside the previous `look` days (j-look ..= j-1).
fn nbar_nr(x: &Ctx, j: usize, n: usize, look: usize) -> bool {
    if j < look + n { return false; }
    let r = nbar_range(x, j, n);
    (j - look + n - 1..j).all(|m| r < nbar_range(x, m, n))
}
fn sign(a: f64, b: f64) -> Option<char> { if a > b { Some('+') } else if a < b { Some('-') } else { None } }
/// The book's pattern string ending today (bar i): closes of the previous len-1 days vs their prior day, then today's open vs yesterday's close.
fn pattern_ok(x: &Ctx, i: usize, pat: &str) -> bool {
    let n = pat.len();
    if i < n { return false; }
    pat.chars().enumerate().all(|(k, ch)| {
        let s = if k + 1 == n { sign(x.o[i], x.c[i - 1]) } else { let j = i - (n - 1) + k; sign(x.c[j], x.c[j - 1]) };
        s == Some(ch)
    })
}
fn segment(lo: f64, hi: f64, p: f64, parts: u8) -> Option<u8> {
    let r = hi - lo;
    if !(r > 0.0) { return None; }
    Some((((p - lo) / r * parts as f64).floor() as i64 + 1).clamp(1, parts as i64) as u8)
}
/// Book Ch 1 Stretch on bar i: mean over the previous n days of |open − the extreme closest to the open|.
fn stretch(x: &Ctx, i: usize, n: usize) -> f64 {
    if i < n { return f64::NAN; }
    (i - n..i).map(|j| (x.h[j] - x.o[j]).min(x.o[j] - x.l[j])).sum::<f64>() / n as f64
}

fn pat_ok(x: &Ctx, pat: Pat, j: usize, doji: f64, r1: f64, look: usize) -> bool {
    let dj = |j: usize| (x.o[j] - x.c[j]).abs() < doji;
    match pat {
        Pat::Any => true,
        Pat::Id => inside(x, j),
        Pat::Doji => dj(j),
        Pat::Nr4 => narrower(x, j, 3),
        Pat::DojiNr4 => dj(j) && narrower(x, j, 3),
        Pat::IdNr4 => inside(x, j) && narrower(x, j, 3),
        Pat::Nr7 => narrower(x, j, 6),
        Pat::Ws4 => wider(x, j, 3),
        Pat::Ws7 => wider(x, j, 6),
        Pat::BearHook => j >= 1 && x.o[j] < x.l[j - 1] && x.c[j] > x.c[j - 1] && rng(x, j) < rng(x, j - 1),
        Pat::BullHook => j >= 1 && x.o[j] > x.h[j - 1] && x.c[j] < x.c[j - 1] && rng(x, j) < rng(x, j - 1),
        Pat::DojiR1 => dj(j) && rng(x, j) < r1,
        Pat::R1 => rng(x, j) < r1,
        Pat::Bnr2 => nbar_nr(x, j, 2, look),
        Pat::Bnr3 => nbar_nr(x, j, 3, look),
    }
}

fn trade(x: &Ctx, dir: f64, i: usize, px: f64, hold: usize, to: usize, kind: &'static str, why: &str) -> Trade {
    let e = i + hold;
    let open = e > to;
    let exit_i = e.min(to);
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i, exit_px: x.c[exit_i],
        reason: if open { "open".into() } else if hold == 0 { format!("{why} · same-day close") } else { format!("{why} · close {hold} day(s) after entry") },
        entry_kind: kind, signal_i: if i > 0 { i - 1 } else { 0 }, open, exits: vec![] }
}

/// ORB fill on bar i at open + signed level: touched when the day's range reaches it; fills at the level.
fn orb_fill(x: &Ctx, i: usize, lvl: f64, mode: f64, mult: f64, sn: usize) -> Option<f64> {
    let off = if mode >= 0.5 { let s = stretch(x, i, sn); if !s.is_finite() { return None; } lvl.signum() * mult * s } else { lvl };
    let px = x.o[i] + off;
    let hit = if off >= 0.0 { x.h[i] >= px } else { x.l[i] <= px };
    hit.then_some(px)
}

// ---- intraday wave 1
/// The same-day ORB tests that fill on real 1-minute bars (admitted on SPX with the printed S&P points).
pub fn same_day_orb(id: &str) -> bool { ORBS.iter().any(|o| o.id == id && o.hold == 0.0) || matches!(id, "tcdt_gap_orb" | "tcdt_triseg_orb" | "tcdt_quad_orb") }
/// ORB fill on real 1-minute bars: the level = today's (daily) open + the signed offset; the first minute that reaches it
/// fills, at the level or at that minute's open when it opened beyond it (buy stops and sell limits above the open alike).
/// None: no minute data that day, or never reached.
fn orb_fill_m(x: &Ctx, i: usize, lvl: f64, mode: f64, mult: f64, sn: usize) -> Option<f64> {
    let off = if mode >= 0.5 { let s = stretch(x, i, sn); if !s.is_finite() { return None; } lvl.signum() * mult * s } else { lvl };
    let b = x.minutes?.day(&x.r[i].date)?;
    let lv = crate::intraday::Level { up: off >= 0.0, px: x.o[i] + off };
    crate::intraday::stop_entry_day(&b, 0, &[lv], &|_, _| None).map(|(_, px, _)| px)
}
// ---- end intraday wave 1

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("tcdt_") { return None; }
    let defs = params(id)?;
    if to >= x.r.len() || from > to { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let mut out = Vec::new();
    let start = from.max(1);
    if let Some(o) = ORBS.iter().find(|o| o.id == id) {
        let (pts, side, hold) = (g("pts"), if g("side") >= 0.0 { 1.0 } else { -1.0 }, g("hold").max(0.0) as usize);
        let lvl = o.lvl.signum() * pts;
        let (doji, r1, look) = (g("doji"), g("r1"), g("look").max(1.0) as usize);
        let (mode, mult, sn) = (g("level_mode"), g("stretch_mult"), g("stretch_n").max(1.0) as usize);
        let minute = hold == 0 && o.hold == 0.0 && g("path") >= 0.5; // intraday wave 1: same-day rows only
        if minute && x.minutes.is_none() { return Some(out); }
        for i in start..=to {
            if !pat_ok(x, o.pat, i - 1, doji, r1, look) { continue; }
            if let Some(px) = if minute { orb_fill_m(x, i, lvl, mode, mult, sn) } else { orb_fill(x, i, lvl, mode, mult, sn) } {
                let kind = if (lvl >= 0.0) == (side > 0.0) { "stop" } else { "limit" };
                out.push(trade(x, side, i, px, hold, to, kind, o.name));
            }
        }
        return Some(out);
    }
    let row = (g("row").round().max(1.0) as usize).saturating_sub(1);
    let over = g("side");
    let side_of = |c: char| if over > 0.5 { 1.0 } else if over < -0.5 { -1.0 } else if c == 'B' { 1.0 } else { -1.0 };
    let (mode, mult, sn) = (g("level_mode"), g("stretch_mult"), g("stretch_n").max(1.0) as usize);
    // intraday wave 1: the gap / segment ORB tables fill on real 1-minute bars by default
    let minute = matches!(id, "tcdt_gap_orb" | "tcdt_triseg_orb" | "tcdt_quad_orb") && g("path") >= 0.5;
    if minute && x.minutes.is_none() { return Some(out); }
    let orb_fill = |x: &Ctx, i: usize, lvl: f64, mode: f64, mult: f64, sn: usize| if minute { orb_fill_m(x, i, lvl, mode, mult, sn) } else { orb_fill(x, i, lvl, mode, mult, sn) };
    match id {
        "tcdt_oc_pattern" | "tcdt_oc_rangeexp" => {
            let t = if id == "tcdt_oc_pattern" { TABLE_A } else { TABLE_B };
            let (pat, bs, ..) = *t.get(row)?;
            for i in start..=to {
                if id == "tcdt_oc_rangeexp" && !(i >= 2 && rng(x, i - 1) > rng(x, i - 2)) { continue; }
                if pattern_ok(x, i, pat) { out.push(trade(x, side_of(bs), i, x.o[i], 0, to, "open", pat)); }
            }
        }
        "tcdt_cc_pattern" => {
            let (pat, bs, _) = *TABLE_CC.get(row)?;
            let n = pat.len();
            for i in start..=to {
                if i < n { continue; }
                let ok = pat.chars().enumerate().all(|(k, ch)| { let j = i + 1 - n + k; sign(x.c[j], x.c[j - 1]) == Some(ch) });
                if ok { let mut t = trade(x, side_of(bs), i, x.c[i], 1, to, "close", pat); t.signal_i = i; out.push(t); }
            }
        }
        "tcdt_oc_nr_re" => {
            let (cond, pat, bs, ..) = *TABLE_E.get(row)?;
            for i in start..=to {
                let j = i - 1;
                let ok = match cond { 0 => true, 1 => j >= 1 && rng(x, j) < rng(x, j - 1), 2 => narrower(x, j, 3), 3 => j >= 1 && rng(x, j) > rng(x, j - 1), _ => wider(x, j, 3) };
                if ok && pattern_ok(x, i, pat) { out.push(trade(x, side_of(bs), i, x.o[i], 0, to, "open", pat)); }
            }
        }
        "tcdt_gap_orb" => {
            let (gap, gdir, lvl, bs, ..) = *GAPS.get(row)?;
            for i in start..=to {
                let gp = (x.o[i] - x.c[i - 1]) * gdir;
                if gp < gap - 1e-9 { continue; }
                if let Some(px) = orb_fill(x, i, lvl, mode, mult, sn) { out.push(trade(x, side_of(bs), i, px, 0, to, "stop", "gap ORB")); }
            }
        }
        "tcdt_triseg_orb" | "tcdt_quad_orb" => {
            let (t, parts) = if id == "tcdt_triseg_orb" { (TRISEG, 3u8) } else { (QUAD, 5u8) };
            let (so, sc, lvl, bs, ..) = *t.get(row)?;
            for i in start..=to {
                let j = i - 1;
                if segment(x.l[j], x.h[j], x.o[j], parts) != Some(so) || segment(x.l[j], x.h[j], x.c[j], parts) != Some(sc) { continue; }
                if let Some(px) = orb_fill(x, i, lvl, mode, mult, sn) { out.push(trade(x, side_of(bs), i, px, 0, to, "stop", "segment ORB")); }
            }
        }
        "tcdt_idp1" | "tcdt_idp2" | "tcdt_idp3" | "tcdt_idp4" => {
            for i in start.max(3)..=to {
                let j = i - 1;
                let dir = match id {
                    "tcdt_idp1" if inside(x, j) => sign(x.o[i], x.c[j]).map(|s| if s == '+' { 1.0 } else { -1.0 }),
                    "tcdt_idp2" if inside(x, j) => match (sign(x.c[j], x.c[j - 1]), sign(x.o[i], x.c[j])) { (Some('+'), Some('+')) => Some(1.0), (Some('-'), Some('-')) => Some(-1.0), _ => None },
                    "tcdt_idp3" if inside(x, j) && x.l[j - 1] > x.l[j - 2] && x.o[i] < (x.h[j] + x.l[j]) / 2.0 && x.o[i] < x.c[j] => Some(-1.0),
                    "tcdt_idp4" if inside(x, j) && x.h[j - 1] < x.h[j - 2] && x.o[i] > x.c[j] => Some(1.0),
                    _ => None,
                };
                if let Some(d) = dir { out.push(trade(x, d, i, x.o[i], 0, to, "open", "inside-day pattern")); }
            }
        }
        _ if id.starts_with("tcdt_ut") => {
            let k: usize = id[7..].parse::<usize>().ok()? + 2; // pivot on Day k (Day 1 = upthrust day)
            let hold = g("hold").max(1.0) as usize;
            for i in start.max(k + 1)..=to {
                let pv = i + 1 - k;
                let pivot = x.h[pv] > x.h[pv - 1] && x.h[pv] > x.h[pv + 1];
                let ut = x.h[i] > x.h[pv] && x.c[i] < x.c[i - 1] && x.c[i] < x.c[i - 2] && x.c[i] < x.h[pv]
                    && x.c[i] < x.o[i] && x.c[i] < (x.h[i] + x.l[i]) / 2.0 && rng(x, i) > rng(x, i - 1);
                if pivot && ut {
                    let mut t = trade(x, -1.0, i, x.c[i], hold, to, "close", "upthrust");
                    t.signal_i = i;
                    out.push(t);
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn nr7_orb_buys_at_open_plus_level_and_exits_on_the_close() {
        // six wide days, an NR7 day (range 1), then a day that trades 0.80 above its open and closes higher
        let mut v = vec![(100., 105., 95., 100.); 6];
        v.push((100., 100.5, 99.5, 100.));
        v.push((100., 102., 99.9, 101.5));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tcdt_nr7_p80", &x, 0, 7, &Params::from([("path".to_string(), 0.0)])).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].dir), (7, 100.8, 7, 101.5, 1.0));
        // the level is not reached -> no trade
        let mut v2 = v.clone(); v2[7] = (100., 100.7, 99.0, 99.5);
        let r2 = bars(&v2); let x2 = Ctx::new(&r2, &r2, &[]);
        assert!(run("tcdt_nr7_p80", &x2, 0, 7, &Params::from([("path".to_string(), 0.0)])).unwrap().is_empty());
    }
    // ---- intraday wave 2a block: open-to-close inside-day patterns (printed p.115), hand-checked
    #[test]
    fn w2a_inside_day_patterns_open_to_close() {
        // bar 3 inside bar 2 with a higher close; bar 4 opens above bar 3's close
        let r = bars(&[(10., 12., 8., 11.), (11., 13., 9., 12.), (12., 14., 10., 13.), (13., 13.5, 10.5, 13.2), (13.5, 14., 13., 13.8)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tcdt_idp1", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 4, 13.5, 4, 13.8));
        assert_eq!(run("tcdt_idp2", &x, 0, 4, &Params::new()).unwrap().len(), 1);
        assert!(run("tcdt_idp4", &x, 0, 4, &Params::new()).unwrap().is_empty()); // bar 2's high is not below bar 1's
        assert!(super::reviewed("tcdt_idp3") && super::reviewed("tcdt_oc_nr_re"));
    }
    // ---- end intraday wave 2a block ----
    #[test]
    fn pattern_strings_and_segments() {
        let r = bars(&[(10., 11., 9., 10.), (10., 12., 9., 11.), (11., 12., 9., 10.), (9.5, 10., 9., 9.8)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(pattern_ok(&x, 3, "+--")); // close up, close down, open below yesterday's close
        assert!(!pattern_ok(&x, 3, "+-+"));
        assert_eq!(segment(0., 9., 9., 3), Some(3));
        assert_eq!(segment(0., 9., 3., 3), Some(2));
        assert_eq!(segment(0., 9., 0., 5), Some(1));
    }
    #[test]
    fn every_live_strategy_has_params_and_runs() {
        let r = bars(&(0..60).map(|i| { let b = 100. + (i as f64 * 0.7).sin() * 3.; (b, b + 1. + (i % 5) as f64 * 0.3, b - 1., b + 0.2) }).collect::<Vec<_>>());
        let x = Ctx::new(&r, &r, &[]);
        for d in catalogue() {
            if d.status == Status::Live { assert!(run(d.id, &x, 0, 59, &Params::new()).is_some(), "{}", d.id); }
        }
    }
}
