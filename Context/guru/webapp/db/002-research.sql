-- Research portal: the team's verdicts, notes and the shared timeline. Rows are never hard-deleted.
CREATE TABLE IF NOT EXISTS research_notes (
    id bigserial PRIMARY KEY,
    target_type varchar(10) NOT NULL CHECK (target_type IN ('strategy','book','author','general')),
    target_key varchar(160) NOT NULL DEFAULT '',
    note_type varchar(12) NOT NULL CHECK (note_type IN ('rule_error','data_issue','question','verified_ok')),
    body text NOT NULL CHECK (length(body) BETWEEN 1 AND 10000),
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    author_name text NOT NULL,
    import_ref text UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    deleted_by text REFERENCES "user"(id) ON DELETE SET NULL,
    deleted_reason text
);
CREATE INDEX IF NOT EXISTS research_notes_target_idx ON research_notes(target_type, target_key, created_at DESC);
CREATE INDEX IF NOT EXISTS research_notes_created_idx ON research_notes(created_at DESC);
CREATE TABLE IF NOT EXISTS research_note_history (
    id bigserial PRIMARY KEY,
    note_id bigint NOT NULL REFERENCES research_notes(id),
    action varchar(8) NOT NULL CHECK (action IN ('created','edited','deleted')),
    note_type varchar(12) NOT NULL,
    body text NOT NULL,
    actor_id text REFERENCES "user"(id) ON DELETE SET NULL,
    actor_name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS research_note_history_note_idx ON research_note_history(note_id, created_at);
CREATE INDEX IF NOT EXISTS research_note_history_created_idx ON research_note_history(created_at DESC);
CREATE TABLE IF NOT EXISTS research_verdicts (
    strategy_key varchar(160) PRIMARY KEY,
    verdict varchar(12) NOT NULL CHECK (verdict IN ('not_checked','matches','mismatch','unclear')),
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    author_name text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS research_verdict_history (
    id bigserial PRIMARY KEY,
    strategy_key varchar(160) NOT NULL,
    verdict varchar(12) NOT NULL,
    previous varchar(12),
    actor_id text REFERENCES "user"(id) ON DELETE SET NULL,
    actor_name text NOT NULL,
    import_ref text UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS research_verdict_history_created_idx ON research_verdict_history(created_at DESC);
CREATE TABLE IF NOT EXISTS research_platform_events (
    event_key varchar(40) PRIMARY KEY,
    happened_at timestamptz NOT NULL,
    label text NOT NULL,
    who text NOT NULL,
    work text NOT NULL,
    backtested integer,
    proof text NOT NULL DEFAULT '',
    source text NOT NULL
);
CREATE INDEX IF NOT EXISTS research_platform_events_at_idx ON research_platform_events(happened_at DESC);
-- Employees who enter the research page with their name and the team PIN. Removing one keeps their history.
CREATE TABLE IF NOT EXISTS research_employees (
    id bigserial PRIMARY KEY,
    name text NOT NULL CHECK (length(name) BETWEEN 2 AND 60),
    name_key text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    removed_at timestamptz
);
