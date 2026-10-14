"""Independent source contract oracle for BTA pp.20-21,80,94-97,99.

This code does not read/translate the Rust engine. Decimal full-window means
and a target/execution event table independently determine every trade.
"""
from decimal import Decimal, localcontext


DEFAULTS = {
    'ch_bta2_ma_stack': dict(first=3, count=2, slow=65),
    'ch_bta2_65sma_ravi': dict(sma=65, confirm=3, fast=7, ravi=1),
}


def source_targets(rows, method, overrides):
    p = DEFAULTS[method] | overrides
    close = [Decimal(r['close']) for r in rows]
    def mean(i, n):
        return None if i + 1 < n else sum(close[i + 1 - n:i + 1], Decimal(0)) / n
    targets = []
    with localcontext() as context:
        context.prec = 40
        if method == 'ch_bta2_ma_stack':
            for i in range(len(rows)):
                long = mean(i, p['slow'])
                short = [mean(i, n) for n in range(p['first'], p['first'] + p['count'])]
                if long is None or any(v is None for v in short):
                    targets.append(0)
                else:
                    targets.append(1 if all(v > long for v in short) else -1 if all(v < long for v in short) else 0)
        else:
            # Each close is compared with its own completed 65-day average.
            sides = []
            for i, c in enumerate(close):
                long = mean(i, p['sma'])
                side = 0 if long is None else (1 if c > long else -1 if c < long else 0)
                sides.append(side)
                fast = mean(i, p['fast'])
                confirmed = (i + 1 >= p['confirm'] and side != 0
                             and all(s == side for s in sides[i + 1 - p['confirm']:i + 1]))
                barrier_pass = (long is not None and fast is not None and
                                abs(Decimal(100) * (fast - long) / long) > Decimal(str(p['ravi'])))
                targets.append(side if confirmed and barrier_pass else 0)
    return targets


def ledger(rows, method, start, end, overrides=None):
    targets = source_targets(rows, method, overrides or {})
    on_close = method == 'ch_bta2_ma_stack'
    events, trades, current = [], [], None
    # Source orders: close agreement fills immediately; RAVI orders fill at
    # the next observed open. No order before the test-start close is carried.
    for signal in range(start, end + 1):
        target = targets[signal]
        fill = signal if on_close else signal + 1
        if target == 0:
            state = 'neutral'
        elif current is not None and current['dir'] == str(target):
            state = 'same_direction_ignored'
        elif fill > end:
            state = 'unfilled_end_of_sample'
        else:
            state = 'filled'
            px = rows[fill]['close' if on_close else 'open']
            if current is not None:
                current.update(exit=str(fill), exit_px=px, open='false', signal_i=str(signal),
                    reason='Opposing SMA agreement · close reversal' if on_close else 'Opposite signal · next-open reversal')
                trades.append(current)
            current = dict(entry=str(fill), dir=str(target), entry_px=px,
                           entry_kind='close' if on_close else 'open', weight='1')
        events.append(dict(signal=signal, signal_date=rows[signal]['date'], target=target,
                           fill_index=fill if fill <= end else None, disposition=state))
    if current is not None:
        current.update(exit=str(end), exit_px=rows[end]['close'], open='true', signal_i=str(end), reason='open')
        trades.append(current)
    return trades, events
