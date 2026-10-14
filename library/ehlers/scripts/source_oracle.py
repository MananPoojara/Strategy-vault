"""Independent daily oracle interpreted from Ehlers Fig.12.1 and MAMA sidebar.

No Rust helpers or generated Rust traces are inputs to these equations. All
recurrences retain the source zero seed and actual MaxBarsBack/CurrentBar gate.
Crossings use a streaming last-unequal relation rather than an array scan.
The private job contract identifies conventions omitted by the source.
"""
import math


def sign(value):
    return (value > 0) - (value < 0)


class Relation:
    def __init__(self, initial=0):
        self.previous_unequal = initial

    def observe(self, difference):
        now = sign(difference)
        event = now if now and self.previous_unequal == -now else 0
        if now:
            self.previous_unequal = now
        return event


def source_states(rows, parameters):
    """Chronological source recurrences with zero-valued historical Vars.

    Each bar dictionary is one source state. Price history is actual OHLC, even
    before calculation begins. Zero state is not seeded from an initial price.
    """
    history = []
    price = [(r['high'] + r['low']) / 2 for r in rows]
    mbb = int(parameters['max_bars_back'])
    cp = parameters.get('cyc_part', 1.0)
    sm_relation, cycle_relation = Relation(), Relation()
    days_relation = Relation(-1)  # historical sin(0) < sin(45), not Vars(0)
    mama_relation = Relation()
    days = 0
    for i in range(len(rows)):
        s = {}

        def past(key, offset):
            return history[i-offset].get(key, 0.0) if i >= offset else 0.0

        def tap(key):
            return (.0962 * s.get(key, 0.0) + .5769 * past(key, 2)
                    - .5769 * past(key, 4) - .0962 * past(key, 6))

        active = i >= mbb + 5
        if active:
            s['smooth'] = sum(w * price[i-j] for j, w in enumerate((4, 3, 2, 1))) / 10
            gain = .075 * past('period', 1) + .54
            s['detrender'] = tap('smooth') * gain
            s['q1'] = tap('detrender') * gain
            s['i1'] = past('detrender', 3)
            s['ji'] = tap('i1') * gain
            s['jq'] = tap('q1') * gain
            s['i2'] = .2 * (s['i1'] - s['jq']) + .8 * past('i2', 1)
            s['q2'] = .2 * (s['q1'] + s['ji']) + .8 * past('q2', 1)
            s['re'] = .2 * (s['i2'] * past('i2', 1) + s['q2'] * past('q2', 1)) + .8 * past('re', 1)
            s['im'] = .2 * (s['i2'] * past('q2', 1) - s['q2'] * past('i2', 1)) + .8 * past('im', 1)
            period = past('period', 1)
            if s['im'] != 0 and s['re'] != 0:
                period = 360 / math.degrees(math.atan(s['im'] / s['re']))
            # Sequential clamps; no positive price seed or atan2 replacement.
            period = min(period, 1.5 * past('period', 1))
            period = max(period, .67 * past('period', 1))
            period = min(max(period, 6), 50)
            s['period'] = .2 * period + .8 * past('period', 1)
            s['smooth_period'] = .33 * s['period'] + .67 * past('smooth_period', 1)

            # Fig.12.1 DFT of SmoothPrice, separate from homodyne phase.
            count = int(s['smooth_period'] + .5)
            real = imaginary = 0.0
            for j in range(count):
                smooth = s['smooth'] if j == 0 else past('smooth', j)
                angle = math.radians(360 * j / count)
                real += math.cos(angle) * smooth
                imaginary += math.sin(angle) * smooth
            phase = past('dcphase', 1)
            if abs(real) > 0:
                phase = math.degrees(math.atan(imaginary / real))
            if abs(real) <= .001:
                phase = 90 * sign(imaginary)
            phase += 90
            phase += 360 / s['smooth_period']
            if imaginary < 0:
                phase += 180
            if phase > 315:
                phase -= 360
            s['dcphase'] = phase
            s['sine'] = math.sin(math.radians(phase))
            s['lead'] = math.sin(math.radians(phase + 45))
            length = int(cp * s['smooth_period'] + .5)
            s['itrend'] = sum(price[i-j] for j in range(length))
            if count > 0:
                s['itrend'] /= length
            s['trendline'] = (4 * s['itrend'] + 3 * past('itrend', 1)
                              + 2 * past('itrend', 2) + past('itrend', 3)) / 10
            if i-mbb+1 < 12:
                s['trendline'] = price[i]
            trend = 1
            if days_relation.observe(s['sine'] - s['lead']):
                days, trend = 0, 0
            days += 1
            if days < .5 * s['smooth_period']:
                trend = 0
            change = phase - past('dcphase', 1)
            if .67 * 360 / s['smooth_period'] < change < 1.5 * 360 / s['smooth_period']:
                trend = 0
            if abs((s['smooth'] - s['trendline']) / s['trendline']) >= .015:
                trend = 1
            s['trend'] = trend

            # S&C MAMA: phase may retain its previous value when I1 is zero.
            phase = past('phase', 1)
            if s['i1'] != 0:
                phase = math.degrees(math.atan(s['q1'] / s['i1']))
            s['phase'] = phase
            fast, slow = parameters.get('fast_limit', .5), parameters.get('slow_limit', .05)
            alpha = min(fast, max(slow, fast / max(past('phase', 1) - phase, 1)))
            s['mama'] = alpha * price[i] + (1-alpha) * past('mama', 1)
            s['fama'] = .5 * alpha * s['mama'] + (1-.5*alpha) * past('fama', 1)
        # Vars before CurrentBar>5 stay zero; inline phase functions differ.
        s['smooth_cross'] = sm_relation.observe(s.get('smooth', 0) - s.get('trendline', 0))
        s['cycle_cross'] = cycle_relation.observe(s.get('lead', 0) - s.get('sine', 0))
        s['mama_cross'] = mama_relation.observe(s.get('mama', 0) - s.get('fama', 0))
        history.append(s)
    return history


def source_order(method, states, i, position, parameters):
    if i < parameters['max_bars_back'] + 5:
        return None
    s = states[i]
    if method == 'ehmd_mama':
        return {1: 1, -1: 0}.get(s['mama_cross'])
    if s['trend'] == 0:
        return s['cycle_cross'] or None
    order = None
    if states[i-1].get('trend', 0) == 0:
        if position == -1 and s['smooth'] >= s['trendline']:
            order = 1
        if position == 1 and s['smooth'] < s['trendline']:
            order = -1
    return s['smooth_cross'] or order


def record(rows, entry, exit_, direction, is_open):
    return {
        'entry_i': entry, 'entry_date': rows[entry]['date'],
        'entry_signal_i': entry-1, 'entry_signal_date': rows[entry-1]['date'],
        'dir': direction, 'entry_px': rows[entry]['open'], 'entry_fill': 'next_open',
        'exit_i': exit_, 'exit_date': rows[exit_]['date'],
        'exit_signal_i': None if is_open else exit_-1,
        'exit_signal_date': None if is_open else rows[exit_-1]['date'],
        'exit_px': rows[exit_]['close' if is_open else 'open'],
        'exit_fill': 'terminal_close_mark' if is_open else 'next_open', 'open': is_open,
    }


def ledger(method, rows, first, last, parameters, states=None):
    states = states if states is not None else source_states(rows, parameters)
    trades, signals = [], []
    position, entry = 0, None
    for i in range(first, last+1):
        target = source_order(method, states, i, position, parameters)
        if target is None:
            continue
        signals.append({'signal_i': i, 'signal_date': rows[i]['date'], 'target': target,
                        'changes_position': target != position, 'has_next_bar': i < last})
        if target == position or i == last:
            continue
        if position:
            trades.append(record(rows, entry, i+1, position, False))
        position, entry = target, i+1 if target else None
    if position:
        trades.append(record(rows, entry, last, position, True))
    return trades, signals, states
