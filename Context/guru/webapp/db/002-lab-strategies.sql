-- Indicators lab saved strategies (spec §3). Additive and idempotent: bootstrap runs it on every start.
CREATE TABLE IF NOT EXISTS lab_strategy (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner text NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    name varchar(120) NOT NULL,
    spec jsonb NOT NULL CHECK (jsonb_typeof(spec) = 'object'),
    market varchar(8) NOT NULL CHECK (market IN ('in','us','gl','crypto')),
    instrument varchar(64) NOT NULL,
    status varchar(10) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','submitted','published','rejected')),
    version integer NOT NULL DEFAULT 1 CHECK (version >= 1),
    submitted_at timestamptz,
    reviewed_by text REFERENCES "user"(id) ON DELETE SET NULL,
    reviewed_at timestamptz,
    review_note varchar(1000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (status <> 'published' OR market IN ('in','us'))
);
CREATE INDEX IF NOT EXISTS lab_strategy_owner_idx ON lab_strategy(owner, updated_at DESC);
CREATE INDEX IF NOT EXISTS lab_strategy_submitted_idx ON lab_strategy(submitted_at) WHERE status = 'submitted';
-- Immutable snapshot of each published version; it outlives the author's account and working row.
-- Only India and US are publishable until licensed global/crypto data exists (a later migration widens this).
-- Snapshots are immutable by app contract; the only UPDATEs are the FK ON DELETE SET NULL actions.
CREATE TABLE IF NOT EXISTS lab_strategy_version (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    strategy_id integer REFERENCES lab_strategy(id) ON DELETE SET NULL,
    version integer NOT NULL CHECK (version >= 1),
    owner text REFERENCES "user"(id) ON DELETE SET NULL,
    name varchar(120) NOT NULL,
    spec jsonb NOT NULL CHECK (jsonb_typeof(spec) = 'object'),
    market varchar(8) NOT NULL CHECK (market IN ('in','us')),
    instrument varchar(64) NOT NULL,
    published_by text REFERENCES "user"(id) ON DELETE SET NULL,
    published_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (strategy_id, version)
);
CREATE INDEX IF NOT EXISTS lab_strategy_version_published_idx ON lab_strategy_version(published_at DESC);
