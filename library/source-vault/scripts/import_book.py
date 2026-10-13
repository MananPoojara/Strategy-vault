#!/usr/bin/env python3
"""Copy one original PDF from the read-only research vault into Books/<author>.

Never overwrites a destination or writes into the source vault. The manifest
records exact bytes and provenance, not source completeness or backtest proof.
"""
import argparse
import hashlib
import json
from pathlib import Path

VAULT = Path('/run/user/1000/gvfs/smb-share:server=192.168.4.50,share=research')
PROJECT = Path(__file__).resolve().parents[3]

def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def import_pdf(relative, author, vault=VAULT, books=None):
    vault = vault.resolve()
    source = (vault / relative).resolve()
    if not source.is_relative_to(vault) or not source.is_file():
        raise ValueError('Source must be a file inside the research vault')
    if not author or author in ('.', '..') or Path(author).name != author or '\\' in author:
        raise ValueError('Author must be one folder name')
    if source.suffix.lower() != '.pdf':
        raise ValueError('Only original PDF books are supported')
    with source.open('rb') as stream:
        if b'%PDF-' not in stream.read(1024):
            raise ValueError('Source has no PDF header')
    books = (books or PROJECT / 'Books').resolve()
    if books.is_relative_to(vault) or vault.is_relative_to(books):
        raise ValueError('Destination Books must be separate from the source vault')
    destination = books / author / source.name
    if not destination.resolve().is_relative_to(books):
        raise ValueError('Destination must remain inside Books')
    original_hash = digest(source)
    if destination.exists():
        if digest(destination) != original_hash:
            raise FileExistsError('Different destination file exists; refusing to overwrite')
        status = 'already_present_identical'
    else:
        destination.parent.mkdir(parents=True, exist_ok=True)
        # Exclusive creation also protects against a concurrent importer.
        created = False
        try:
            with source.open('rb') as inp, destination.open('xb') as out:
                created = True
                for chunk in iter(lambda: inp.read(1024 * 1024), b''):
                    out.write(chunk)
            if digest(destination) != original_hash:
                raise ValueError('Copied bytes do not match the recorded source hash')
        except BaseException:
            if created:
                destination.unlink(missing_ok=True)
            raise
        status = 'copied'
    if digest(destination) != original_hash:
        raise ValueError('Copied bytes do not match the recorded source hash')
    return {'author': author, 'source_vault': str(vault),
            'source_relative': str(source.relative_to(vault)),
            'destination': str(destination.relative_to(books.parent)),
            'bytes': destination.stat().st_size, 'sha256': original_hash,
            'status': status, 'source_completeness_verified': False,
            'independent_performance_verified': False}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_relative')
    parser.add_argument('--author', required=True)
    args = parser.parse_args()
    print(json.dumps(import_pdf(args.source_relative, args.author), indent=2))
