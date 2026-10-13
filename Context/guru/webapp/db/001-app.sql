CREATE TABLE IF NOT EXISTS vault_notes (
    note_key varchar(80) PRIMARY KEY,
    content text NOT NULL DEFAULT '',
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS vault_settings (
    user_id text PRIMARY KEY REFERENCES "user"(id) ON DELETE CASCADE,
    theme varchar(16) NOT NULL DEFAULT 'aqua' CHECK (theme IN ('steel','aqua','lagoon','glacier')),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS vault_note_history (
    id bigserial PRIMARY KEY,
    note_key varchar(80) NOT NULL,
    content text NOT NULL,
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS vault_note_history_key_idx ON vault_note_history(note_key, created_at DESC);
CREATE TABLE IF NOT EXISTS vault_audit (
    id bigserial PRIMARY KEY,
    actor_id text REFERENCES "user"(id) ON DELETE SET NULL,
    action text NOT NULL,
    entity text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE vault_settings ADD COLUMN IF NOT EXISTS last_view text NOT NULL DEFAULT '#/';

ALTER TABLE vault_settings DROP CONSTRAINT IF EXISTS vault_settings_theme_check;
ALTER TABLE vault_settings ADD CONSTRAINT vault_settings_theme_check CHECK (theme IN ('steel','aqua','lagoon','glacier','midnight','forest','sand','plum','cobalt'));
CREATE TABLE IF NOT EXISTS vault_annotations (
    entity_key varchar(80) PRIMARY KEY,
    tags jsonb NOT NULL DEFAULT '[]'::jsonb,
    ceo boolean NOT NULL DEFAULT false,
    ceo_author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
