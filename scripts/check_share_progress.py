#!/usr/bin/env python3
"""Fail a deployment check if a share inventory embeds stale research progress.

Usage: python3 scripts/check_share_progress.py SHARE_INVENTORY RESEARCH_PROGRESS
Read-only; run after the inventory has been generated and again on the installed file.
"""
import json
import sys


def validate(share, progress):
    c, totals = share['claude'], progress['totals']
    for key in ('run', 'as_of'):
        if not isinstance(progress[key], str) or not progress[key] or c[key] != progress[key]:
            raise ValueError(f'{key} differs or is missing')
    mapping = {'strategies': 'strategies', 'backtested': 'verified', 'held': 'held', 'unreviewed': 'unreviewed'}
    for field, target in mapping.items():
        value = totals[target]
        if type(value) is not int or value < 0 or type(c[field]) is not int or c[field] != value:
            raise ValueError(f'{field} differs or is invalid')
        parts = [s[field] for s in share['sources']] + [c['unmatched'][field]]
        if any(type(n) is not int or n < 0 for n in parts) or sum(parts) != value:
            raise ValueError(f'{field} source attribution does not add up')
    if len(progress['strategies']) != totals['strategies']:
        raise ValueError('progress strategy rows do not add up')
    if sum(totals[k] for k in ('verified', 'held', 'unreviewed')) != totals['strategies']:
        raise ValueError('progress status counts do not add up')


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        with open(sys.argv[1], encoding='utf-8') as f:
            share = json.load(f)
        with open(sys.argv[2], encoding='utf-8') as f:
            progress = json.load(f)
        validate(share, progress)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f'Share progress check failed: {exc}', file=sys.stderr)
        return 1
    print(f"Share progress matches {progress['run']} as of {progress['as_of']}.")
    return 0


if __name__ == '__main__':
    sys.exit(main())
