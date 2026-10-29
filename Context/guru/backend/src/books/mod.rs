//! One module per book. Each exposes catalogue / params / run / family_of; this file just chains them.
use crate::connors::{ParamDef, Params, StrategyDef, Ctx, Trade};
pub mod joe_ross_math;
pub mod elcim_account;
pub mod hbiab_reviewed;
pub mod lw_admit; // Larry Williams block

macro_rules! books {
    ($($m:ident),*) => {
        $(pub mod $m;)*
        pub fn catalogue() -> Vec<StrategyDef> {
            let mut v = Vec::new();
            $(v.extend($m::catalogue());)*
            v
        }
        pub fn params(id: &str) -> Option<Vec<ParamDef>> {
            None$(.or_else(|| $m::params(id)))*
        }
        pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
            None$(.or_else(|| $m::run(id, x, from, to, p)))*
        }
        pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
            None$(.or_else(|| $m::family_of(id)))*
        }
    };
}

books!(ws5, ats, coats, ss, hfm, lrd, oiss, otvt, rsi25, spmt, btp, ptjdt, ptjmt, ptjopt, pctb, vixr, cwin, alpha, joe_ross, joe_ross_rules, chande, sofmt, jsgsta, jsats, nmwbk, radge, covel, ststw, hpetf, hmrw, tharp, afactor, r3r4, mmft, lwlts, lwdg2, tcdt, tcsp, jt, kftsm, lbrtt, jc5dm, jchr1, jchr2, mhart, mhpst, mhstp, strmm, strtw, apta, apmacd, pbwts, putg, lebtc, lebcafm, stad, kets, jb624, jbudt, jb30d, jbsmsw, jbssf, jbcdt, ehrs, ehmd, ehmtc, tdperl, chbta2, chntt, bab80, baboops, rpdt, rpeo, hsta, hcta, jkbwts, jk36k, jkbk, jkzap, wdowf, wdnbp, wdgig, rgcts, rgtsec, rgstt, rgsh, gadm, aqrtf, lpgap, jmls, bwntd, bwtc1, bwtc2, stad2, apwms, apsmts, apoi, lwicw, lwmdc, lwpro, lwrsr, lwmds, lwhtb, kfsc, kfbpm, kfst, tdnsta, tdnmtt, dlost, dlstfl, mbhfe, mbsot, mkits, mkfve, rwmts, hbqts, hbiab, dmast, dmst2, clfs, clomw, wwnc, bobb, elcim, eltfl, khtwg, cktwo, lwstc, lwppt, lwsss, lwmil, lwic6, lwcot, pstc, prap1, prap2, gb10, ecqt, rwtlc, swsp, dgmf, acatr, arebta, lbrvb, dgstt, mcmts, csadx, lg3k, aanwr, aanto, lgvp, lgtt30, lgsc, kpss, maetf, mahrs, lkqts, lkcpt, nftm, nfhp, nfgu, nfmt, nfpk, acbff, cbpst, chpbs, jhs35, jsdow, lbi12, mzwws, ndbrm, rjspr, lwdtfo, lwcmc, lwad, lebsp, rjmil, rjtg, jhbct, wbmdd, tbpk, svmws, svtv2, svtvc, dvetf, bergsth, pstc08, tdwb, elee, elsss, dggw, dl10b, kfcd, dmtad, ramtm, jbcdt2, jbeft, jbhsms, jbtywr, jbtifm, jbhpp, kfgei, lebfut, lebws, tbabc, tbsyn, tdscal, prpr, elsgee, elsgss, jkofc, dgdwb, dgppt, dgmtt, ndtci, bllstr, stwds, cjlmom, masmt, sglr, lydetf, fejmma, mpts, mpmm, jtdpls, jswct, awab, cgtdd, gfstsb, hkmmts, palets, sbma, lsgst, tkttl, zsms, kdta, vsbrw, jtwl, sbcft, rbhetf, ckbtm, phbt, lydpes, hkcgdt, mpasi, jbcaf, cgift, elmts4, capttm, lebtcb, dgnl, mjthir, pwabal, jwbent, fcfail, ubvma, kfaber, gcplad, ddrvi, aatt, rhlss, mcstis, cmeod, mjzz, rcmm, sprfs, mlhpt, jscgf, mapsl, kwcci, stgdg, gh52w, cghms, lswtv, ltan1, ltrsg, cpgap, atm00, atm01, atm02, atm03, atm04, atm10, atmmisc, ftm, ctcn, mrina, tsx, atmcsa, atmrc, lwnl, wmny, sfo, ftm2, ltan2, stkch, ataa, t2w, sctw, sixpk, ts247, bwtm, mjlbb, fbs50, fsi23, mfcat, ltwr, tvax, pdmb, msftp, sl2040, mbbb2, msws, mstt, rarsys, ajmisc, tscom, alfar, andro, axsym, barra, ltcoat, pwadcs, gunsl, ginyd, jcurt, dtrend, jcfeed, alfrd, ltsfut, qsfund, rvecl, sktbw, rrsim, dtwxp, tchnl, stfun, jhsto, lwinb, lwfc10, bfmmx, tcsyn, zstrt, tttt1, msmom, ccl08, tilma, bcsts, bfmm, rinadz, tcrsi, jrfsp, bpqst, wmmts, phbttk, vdso, mslib, mscvl, mssta, mxswg, jlmms, bfm2c, tltdr, rrlft, mjmcr, hwdti, mhnats, upg18, msadd, mptae, mpcse, mpsel, gregoriou, mccic, fzhci, fontanills, mtpos, fischer, apitc, minervini, jperson, oneil, bigalow, murphy, pesavento, vanvliet, carney, bickford, mdarcher, gbayer, skdab, metwo, csmlt, kllien, rwarms, darvas, kmoneil, ebdtf, gangell, jaugen, rdwyck, habell, snison, jkennedy, kdittmann, jtduffy, swirrick, rborowski, rmerriman, djhta, jlangham, cbsiws, gmaclean, mlink, mmcrae, pvelez, pcapra, jcarter, bstacm, epfpp, wtdfg, wtdtt);
