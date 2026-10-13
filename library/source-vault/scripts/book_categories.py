#!/usr/bin/env python3
"""Evidence-qualified subject tags. Categories never certify rules or performance."""
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[3]
CATEGORIES=[{'id':'rule_based','label':'Rule-based'}, {'id':'psychology','label':'Psychology'},
            {'id':'money_management','label':'Money management'}]
# These new catalogue entries have bibliographic/cover evidence, but their
# chapter content and complete rule procedures have not yet been reviewed.
READING_ONLY_ADDITIONS=[
 {'id':'p_intraday_capra','title':'Intraday Trading Techniques','author_id':'pristine'},
 {'id':'p_tools_tactics','title':'Tools & Tactics for the Master Day Trader','author_id':'pristine'},
 {'id':'p_tpm1','title':'Trading the Pristine Method — Part 1','author_id':'pristine'},
]
CURATED={
 'stc9':(['rule_based'],'reviewed','Source slides define specific SMA/holding and RSI entry/exit predicates; review is bounded to those methods.',
         ['library/connors/VERIFICATION-STATUS.md','library/connors/STOCK-RSI-PULLBACK-AUDIT.md']),
 'ch_bta':(['rule_based'],'reviewed','The supplied partial edition has a source-audited 65SMA/three-close reversal sequence. Other clauses and original contract studies remain unresolved.',
           ['library/chande/README.md']),
 'champion':(['psychology','money_management'],'reviewed','Actually checked contents/introduction explicitly cover trading psychology, position sizing and risk limits; no full deterministic system is inferred.',
             ['library/source-vault/import-manifest-2026-10-13.json; supplied PDF2–8']),
 'wizard':(['money_management'],'reviewed','Actually checked contents/foreword describe controlling risk. This is a risk-management subject tag, not an audited complete SEPA stock-selection system.',
           ['library/source-vault/import-manifest-2026-10-13.json; supplied PDF9–16']),
 'tharp':(['rule_based','money_management'],'catalogue-description','Existing description specifies random entry/ATR trailing stop, a 55/21 breakout and position-sizing options; no whole-book source audit is asserted.',
          ['library/source-vault/book-coverage.json; catalogue tharp']),
 'strmm':(['rule_based','money_management'],'catalogue-description','Recorded title explicitly combines trading systems and money management; description identifies eight systems. Source fidelity is not established by this subject classification.',
          ['library/source-vault/book-coverage.json; catalogue strmm']),
 'rwtlc':(['rule_based'],'catalogue-description','Existing catalogue explicitly describes mechanical systems; the subject tag does not adopt any performance claim.',
          ['library/source-vault/book-coverage.json; catalogue rwtlc']),
 'rwmts':(['rule_based'],'catalogue-description','Recorded title explicitly says Mechanical Trading Systems. Existing performance comparisons are not treated as verification evidence.',
          ['library/source-vault/book-coverage.json; catalogue rwmts']),
 'mhstp':(['rule_based'],'catalogue-description','The existing card identifies price-pattern trading methods; complete-source/readiness certification remains separate.',
          ['library/source-vault/book-coverage.json; catalogue mhstp']),
 'ststw':(['rule_based'],'catalogue-description','Catalogue identifies RSI2, cumulative RSI, ten-day highs/lows and Double7 rule methods; original study/universe reproduction is not established.',
           ['library/source-vault/book-coverage.json; catalogue ststw']),
 'hpetf':(['rule_based'],'catalogue-description','Catalogue identifies seven basic/aggressive ETF strategies with long/short rules. This is topic evidence, not permission to substitute instruments.',
           ['library/source-vault/book-coverage.json; catalogue hpetf']),
 'etfss':(['rule_based'],'catalogue-description','Existing description explicitly identifies six long/short rules in a manual; execution/data checks remain separate.',
           ['library/source-vault/book-coverage.json; catalogue etfss']),
 'baboops':(['rule_based'],'catalogue-description','Recorded title explicitly identifies a mechanical day-trading system; original source/contract verification remains pending.',
             ['library/source-vault/book-coverage.json; catalogue baboops']),
 'eltfl':(['psychology','money_management'],'reviewed','The actual supplied title page names Psychology, Trading Tactics and Money Management. This subject review does not establish complete deterministic Triple Screen rules.',
          ['library/source-vault/category-evidence/elder-page4.png; actual supplied PDF4 title page']),
 'rjtg':(['money_management'],'reviewed','Actually read preface and contents describe money management, Fixed Fractional Trading, Fixed Ratio Trading and risk of ruin. No missing entry/stop rule is invented.',
         ['library/source-vault/category-evidence/jones-first18.txt; supplied PDF1,4–5']),
}

CURATED['tharp']=(['rule_based','psychology','money_management'],'reviewed',
 'Actual contents/preface cover judgmental biases and position sizing. The rule-method tag also uses the existing explicit ATR/breakout catalogue description; this is not a full entry/exit source audit.',
 ['library/source-vault/category-evidence/tharp-first18.txt; supplied PDF7–9,13–15','library/source-vault/book-coverage.json; catalogue tharp'])
CURATED['rwmts']=(['rule_based','psychology','money_management'],'reviewed',
 'Actual title/contents/introduction pair mechanical systems with trader psychology; contents include volumetric risk, martingale/anti-martingale and diversification. No quoted trade-count claim is certified.',
 ['library/source-vault/category-evidence/weissman-first18.txt; supplied PDF4,10–14,17'])

INCOMPLETE=re.compile(r'\b(?:discretionary|judged|judgment|judgement|repaint(?:ing)?|incomplete|unaudited)\b|'
                      r'no complete|without complete|not (?:a )?complete|never (?:given|measured)|(?:formula|rules?|levels?) missing|not testable',re.I)
RULE_TITLE=re.compile(r'\b(?:mechanical|quantitative|systematic)\b|trading (?:systems|strategies|rules)|system design|pattern-based trading',re.I)
RULE_DESCRIPTION=re.compile(r'\bmechanical\b|\brules\b|stop-and-reverse|crossovers?|breakouts?|RSI\s*\(?\d|mean.reversion|trend.following|trailing stop|entry/exit',re.I)


def classify(card):
    selected=CURATED.get(card['id'])
    if selected:
        categories,certainty,reason,evidence=selected
    else:
        categories=[];summary=card.get('summary') or '';text=card['title']+' '+summary
        if not INCOMPLETE.search(summary) and (RULE_TITLE.search(card['title']) or RULE_DESCRIPTION.search(summary)):
            categories.append('rule_based')
        if re.search(r'psycholog|emotional|mental discipline|trader discipline',text,re.I):categories.append('psychology')
        if re.search(r'money management|position.siz|risk management|risk allocation',text,re.I):categories.append('money_management')
        certainty='catalogue-description' if categories else 'unreviewed'
        reason=('Explicit catalogue subject/rule-method wording supports these topic tags. Source rule completeness, instrument/data readiness and performance remain separate.'
                if categories else 'No evidence-qualified category assigned. Review pending; technical/discretionary titles are not automatically mechanical rules.')
        evidence=['library/source-vault/book-coverage.json; catalogue '+card['id']] if categories else []
    category_evidence={c:{'certainty':certainty,'evidence':evidence} for c in categories}
    if card['id']=='tharp':
        category_evidence['rule_based']={'certainty':'catalogue-description',
            'evidence':['library/source-vault/book-coverage.json; catalogue tharp']}
        for category in ['psychology','money_management']:
            category_evidence[category]={'certainty':'reviewed',
                'evidence':['library/source-vault/category-evidence/tharp-first18.txt; supplied PDF7–9,13–15']}
    return {'book_id':card['id'],'title':card['title'],'categories':categories,
            'category_labels':[c['label'] for c in CATEGORIES if c['id'] in categories],
            'certainty':certainty,'reason':reason,'evidence':evidence,'category_evidence':category_evidence,
            'classification_basis':'Bounded actual source topic review' if certainty=='reviewed' else 'Recorded catalogue subject metadata only' if certainty=='catalogue-description' else 'Awaiting review',
            'verification_status':'not_established_by_subject_category',
            'performance_verified':False,'complete_source_review_claimed':False}


def build(root=ROOT):
    root=Path(root);coverage=json.loads((root/'library/source-vault/book-coverage.json').read_text())
    books={b['id']:{**classify(b),'author_id':a['id']} for a in coverage['authors'].values() for b in a['catalogue_books']}
    for card in READING_ONLY_ADDITIONS:
        books.setdefault(card['id'],{**classify(card),'author_id':card['author_id']})
    source_titles={t['id']:{'categories':[],'certainty':'unreviewed','reason':'Source-title/folder candidate; content category awaits actual review.','performance_verified':False}
                   for a in coverage['authors'].values() for t in a['source_titles']}
    result={'schema_version':1,'as_of':'2026-10-13','categories':CATEGORIES,'books':books,'source_titles':source_titles,
            'default':{'categories':[],'certainty':'unreviewed','label':'Review pending / Uncategorized'},
            'count_warning':'Subject categories are not source, implementation, data or backtest verification. Tags may overlap. Unclassified does not mean no useful content.',
            'scope':'Evidence-qualified subject tags for current catalogue cards. Bounded source previews do not imply complete source review. All other source-title candidates remain unreviewed.'}
    (root/'library/source-vault/book-categories.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({'catalogue_entries':len(books),'curated':sum(bool(b['categories']) for b in books.values()),'unreviewed_source_candidates':len(source_titles)}))
    return result


if __name__=='__main__':build()
