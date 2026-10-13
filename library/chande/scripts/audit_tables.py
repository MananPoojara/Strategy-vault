"""Recalculate printed tables; no original trade ledger or market performance is invented."""
import json
import statistics
from decimal import Decimal as D
from pathlib import Path

# Manually transcribed from supplied printed79/PDF95. These are rounded SOURCE claims.
entry_percentages=[
 [55,59,60,58,60,60],[54,57,56,54,50,51],[51,49,50,52,50,46],
 [53,55,56,57,59,55],[60,61,62,63,64,60],[54,53,53,56,58,45],
 [59,59,60,58,59,63],[59,59,61,62,63,62],[54,55,54,49,53,47],
 [53,55,58,56,51,51],[55,53,60,61,59,69],[57,57,59,57,55,59],
 [53,52,52,55,55,45],[48,50,45,46,44,46],[52,47,51,52,53,51],
 [54,59,58,62,58,69],[56,56,55,58,57,52],[56,56,59,58,63,61],
 [57,59,59,58,58,56],[55,52,56,50,50,46],[52,52,51,51,51,51]]
printed_means=['54.62','55','55.95','55.86','55.71','54.52']

# Printed81/PDF97, visually checked against the supplied rendered page.
# Columns: market, paper profit, trades, rounded win%, market payoff, rounded avg trade, intraday DD.
rows=[
 ['British pound',125344,105,34,'3.72',1193,-25431],
 ['Canadian dollar',-12750,125,25,'2.32',-102,-21030],
 ['Cocoa',-15370,101,28,'1.80',-153,-2219],
 ['Coffee',239096,120,30,'5.83',1993,-36956],
 ['Copper',-7890,49,34,'1.48',-161,-17355],
 ['Corn',26081,106,38,'2.98',246,-4331],
 ['Cotton',112490,110,38,'4.26',1023,-8730],
 ['Crude oil',17570,74,35,'2.58',238,-11690],
 ['Deutsche mark',68575,102,38,'2.90',673,-13250],
 ['Eurodollar',34175,60,25,'3.16',569,-7150],
 ['Gold',53770,121,33,'3.44',444,-28440],
 ['Heating oil',56198,103,32,'3.89',545,-18021],
 ['Japanese yen',143425,87,47,'3.80',1649,-12963],
 ['Live hogs',31971,120,42,'2.49',266,-5863],
 ['Orange juice',13018,120,27,'3.05',109,-27950],
 ['Silver',197305,144,37,'6.87',1370,-51040],
 ['Soybeans',62406,114,38,'2.86',547,-21768],
 ['S&P-500',-7260,101,24,'3.13',-72,-97470],
 ['Sugar',49493,113,37,'3.75',438,-10806],
 ['Swiss franc',108475,100,40,'3.28',1086,-11638],
 ['10-year T-note',34219,85,29,'3.66',402,-13743],
 ['US bond',50143,102,35,'2.62',491,-38819],
 ['Wheat',6263,138,28,'2.78',45,-19663]]


def audit():
    means=[sum(D(row[i]) for row in entry_percentages)/21 for i in range(6)]
    for calculated,printed in zip(means,printed_means):
        assert calculated.quantize(D('.01'))==D(printed).quantize(D('.01'))
    profit=sum(r[1] for r in rows);trades=sum(r[2] for r in rows)
    assert profit==1386747 and trades==2400 and sum(r[1]>0 for r in rows)==19
    return dict(kind='Arithmetic of supplied SOURCE table claims only',
                original_trades_reproduced=False,review_date='2026-10-09',
                table_4_1=dict(source='printed79/PDF95',rows=entry_percentages,
                    columns=[5,10,15,20,30,50],printed_means=printed_means,
                    calculated_unweighted_means=[str(v) for v in means],all_displayed_means_match=True,
                    pooled_win_rate_unavailable=True,
                    horizons_not_strictly_above_55=[n for n,v in zip([5,10,15,20,30,50],means) if v<=55]),
                table_4_2=dict(source='printed81/PDF97',columns=['market','paper_profit','trades','rounded_win_pct',
                        'market_payoff_ratio','printed_average_trade','printed_intraday_dd'],rows=rows,
                    total_paper_profit=profit,total_trades=trades,profitable_markets=sum(r[1]>0 for r in rows),
                    mean_market_profit=str(D(profit)/23),pooled_average_trade=str(D(profit)/trades),
                    unweighted_mean_printed_average_trade=str(sum(D(r[5]) for r in rows)/23),
                    unweighted_mean_exact_market_average_trade=str(sum(D(r[1])/r[2] for r in rows)/23),
                    unweighted_mean_market_win_pct=str(sum(D(r[3]) for r in rows)/23),
                    unweighted_mean_market_payoff_ratio=str(sum(D(r[4]) for r in rows)/23),
                    pooled_payoff_ratio_unavailable=True,exact_pooled_win_rate_unavailable=True,
                    mean_market_intraday_dd=str(sum(D(r[6]) for r in rows)/23),
                    sample_sd_profit=statistics.stdev(r[1] for r in rows),
                    sample_sd_printed_average_trade=statistics.stdev(r[5] for r in rows)),
                findings=[
                    '$558 is a rounded equal-weight average of market averages, not profit divided by 2400 trades ($577.81125).',
                    '3.3 is the rounded equal-weight mean of market payoff ratios; a pooled payoff cannot be inferred.',
                    'Swiss franc displayed108475/100=1084.75 differs from printed average1086; no source value is silently changed.',
                    'Cocoa terminal loss15370 exceeds displayed DD2219. This is a conditional consistency concern, pending the source drawdown definition/ledger.',
                    'Table4.1 horizon win percentages alone do not establish an edge over random; counts, dependence, benchmark and uncertainty are absent.',
                    'The entry-only fixed-horizon study and the cost-adjusted reversal study are different experiments.'])


if __name__=='__main__':
    output=Path(__file__).parents[1]/'SOURCE-NUMBERS-CHECK.json'
    output.write_text(json.dumps(audit(),indent=2)+'\n')
    print('Six table means and23-market totals checked; source claims remain separate from computed market results.')
