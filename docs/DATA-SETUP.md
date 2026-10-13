# External data setup

Nothing in this repository supplies or invents trading prices or original book files. The original research and market-data shares are read-only sources.

Set `VAULT_SOURCE_ROOT` to a private workspace containing the approved real files, then run `Context/guru/webapp/scripts/setup.py`. Its established layout expects:

- `idx_data.csv`: admitted Indian index OHLC, with Nifty history starting in2000.
- `india_vix/`, `extra_data/india/`, `extra_data/us/`: optional/required auxiliary series according to strategy admission.
- `verification/us-book-replication/us_data.csv` and `verification/us-book-replication/vix/`: admitted actual US index/ETF/VIX inputs.
- `Context/guru/frontend/data/`: existing library metadata, basket descriptors, reading-file mappings, source reviews and optional generated downloadable inventory. It is staged separately under `VAULT_RUNTIME_ROOT/frontend-data` and mounted read-only into the gateway. It is not baked into Git or Docker images.

The engines read local staged data. `VAULT_BASKET_DATA_ROOT` and `VAULT_US_BASKET_DATA_ROOT` configure the external admitted Indian/US stock caches. Preserve their original manifest/data layout. See the established importer scripts and webapp README. Do not run a whole-NAS import automatically on a fresh checkout.

Original reading files go in `VAULT_RUNTIME_ROOT/books`; admitted IDs/paths are supplied by the existing checksum stager in `book-index.json`. A fresh setup creates an empty reader manifest until actual source copies are explicitly staged. Copying a PDF does not certify book identity, strategy rules or published performance.

The new React source is usable without generated catalogue inventories; the curated application definitions remain in code. Additional reading coverage and inventory downloads require their external metadata/files. Real backtests require the admitted prices. Do not commit any of these external inputs, or use an index as a replacement for missing stock/ETF/options/futures data.
