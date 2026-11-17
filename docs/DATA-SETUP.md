# External data setup

Nothing in this repository supplies or invents trading prices or original book files. The original research and market-data shares are read-only sources.

Set `VAULT_SOURCE_ROOT` to a private workspace containing the approved real files, then run `Context/guru/webapp/scripts/setup.py`. Its established layout expects:

- `idx_data.csv`: admitted Indian index OHLC, with Nifty history starting in2000.
- `india_vix/`, `extra_data/india/`, `extra_data/us/`: optional/required auxiliary series according to strategy admission.
- `verification/us-book-replication/us_data.csv` and `verification/us-book-replication/vix/`: admitted actual US index/ETF/VIX inputs.
- `Context/guru/frontend/data/`: existing library metadata, basket descriptors, reading-file mappings, source reviews and optional generated downloadable inventory. It is staged separately under `VAULT_RUNTIME_ROOT/frontend-data` and mounted read-only into the gateway. It is not baked into Git or Docker images.

The engines read local staged data. `VAULT_BASKET_DATA_ROOT` and `VAULT_US_BASKET_DATA_ROOT` configure the external admitted Indian/US stock caches. Preserve their original manifest/data layout. See the established importer scripts and webapp README. Do not run a whole-NAS import automatically on a fresh checkout.

For dated YT cash-stock AUX research, set `VAULT_YT_AUX_DIR` to the matching read-only local copy of the AmiBroker cash database used to build the basket index. Its symbol files must match the index's original-source SHA-256 and their dated prices must match the staged CSVs. The current-and-past C&P mapping uses AUX1 for NSE500 and AUX2 for F&O cash stocks; a database explicitly documented with both requires AUX1 = 1 **and** AUX2 = 1. AUX meanings are database-specific: do not apply the cash-stock mapping to intraday futures. The original NR7 AFL does not contain this basket condition, so applying it is a declared entry-filter adaptation, not proof of a source-equivalent backtest. Plain NR7 and short Candle Range have close-price execution paths; the filtered short's hard stop must be off. When enabled for this portfolio path, the short turnover filter reads the original dated OpenInt field; the existing single-stock turnover calculation is unchanged. Other portfolio execution contracts remain held pending review. A dated flag does not prove the symbol inventory is complete or every historical value is correct.

`python3 scripts/stage_yt_aux_cash.py SOURCE_DIR NEW_LOCAL_OUTPUT_DIR` scans an explicitly selected cash database and writes a separate local staging directory with `native/`, `prices/`, and a validation report. It never modifies the original database, refuses an existing output directory, and creates `prices/index.csv` only if the entire source inventory passes its checks. Supply `prices/` as the basket root and `native/` as `VAULT_YT_AUX_DIR` after review. No staging or full-database run happens at startup.

The AmiBroker-style YT portfolio (`basket-run?mode=amibroker`, `src/yt_amibroker_portfolio.rs`) reads native AmiBroker databases directly from `AMIBROKER_DATA_DIR` (compose mounts `VAULT_AMIBROKER_DATA_ROOT` read-only at `/amibroker-data`; unset means the mode reports `needs_data`). Expected subfolders: `cp_updated_2026` (AUX1 NSE 500 C&P, AUX2 F&O), `cp_old_2022`, `nifty50_cp_2026`, `midcap100_cp_2026`, `midcap100_aux12_2025` (AUX1 and AUX2), `midcapselect_cp_2026`. Each request reads the symbol files lazily (bounded threads, 90-second budget); only the declared bar count of each file is read.

Original reading files go in `VAULT_RUNTIME_ROOT/books`; admitted IDs/paths are supplied by the existing checksum stager in `book-index.json`. A fresh setup creates an empty reader manifest until actual source copies are explicitly staged. Copying a PDF does not certify book identity, strategy rules or published performance.

The new React source is usable without generated catalogue inventories; the curated application definitions remain in code. Additional reading coverage and inventory downloads require their external metadata/files. Real backtests require the admitted prices. Do not commit any of these external inputs, or use an index as a replacement for missing stock/ETF/options/futures data.
