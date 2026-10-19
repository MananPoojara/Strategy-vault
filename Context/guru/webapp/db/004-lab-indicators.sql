-- Custom indicators (spec 2026-10-19 §3): per-user saved formulas. Additive and idempotent: bootstrap runs it on every start.
-- The engine parses formulas; the database only bounds their size. Names are unique per owner, case-insensitively.
CREATE TABLE IF NOT EXISTS lab_indicator (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner text NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    name varchar(60) NOT NULL CHECK (btrim(name) <> ''),
    formula varchar(4000) NOT NULL CHECK (btrim(formula) <> ''),
    -- Block-editor layout so the UI can reopen it; NULL for formulas typed as text. The formula stays authoritative.
    blocks jsonb CHECK (blocks IS NULL OR jsonb_typeof(blocks) = 'object'),
    version integer NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS lab_indicator_owner_name_idx ON lab_indicator(owner, lower(name));
