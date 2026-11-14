-- Research YTS: issues the QA team finds on the YT Strategies pages, one row per issue, filed against a YT
-- collection (book id such as yt_bb) and optionally one strategy in it. Rows are never hard-deleted; every
-- change (create, edit, status, comment, delete) is kept in yts_issue_events.
CREATE TABLE IF NOT EXISTS yts_issues (
    id bigserial PRIMARY KEY,
    collection varchar(40) NOT NULL,
    strategy_id varchar(120) NOT NULL DEFAULT '',
    category varchar(16) NOT NULL CHECK (category IN ('wrong_trades','wrong_stats','data','page','rules','other')),
    severity varchar(8) NOT NULL DEFAULT 'normal' CHECK (severity IN ('low','normal','high')),
    status varchar(12) NOT NULL DEFAULT 'open' CHECK (status IN ('open','in_progress','fixed','not_issue')),
    title text NOT NULL CHECK (length(title) BETWEEN 3 AND 160),
    body text NOT NULL CHECK (length(body) BETWEEN 1 AND 10000),
    instrument varchar(80) NOT NULL DEFAULT '',
    period_from date,
    period_to date,
    page_link text NOT NULL DEFAULT '' CHECK (length(page_link) <= 2000),
    reporter_name text NOT NULL,
    author_id text REFERENCES "user"(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    deleted_by text REFERENCES "user"(id) ON DELETE SET NULL,
    deleted_reason text
);
CREATE INDEX IF NOT EXISTS yts_issues_target_idx ON yts_issues(collection, strategy_id, created_at DESC);
CREATE INDEX IF NOT EXISTS yts_issues_status_idx ON yts_issues(status, created_at DESC);
CREATE TABLE IF NOT EXISTS yts_issue_events (
    id bigserial PRIMARY KEY,
    issue_id bigint NOT NULL REFERENCES yts_issues(id),
    action varchar(8) NOT NULL CHECK (action IN ('created','edited','status','comment','deleted')),
    status varchar(12),
    body text NOT NULL DEFAULT '',
    actor_id text REFERENCES "user"(id) ON DELETE SET NULL,
    actor_name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS yts_issue_events_issue_idx ON yts_issue_events(issue_id, created_at);
-- Research YTS has its own sign-in: one account per QA (and Research YTS admins), separate from the app login.
-- Passwords are stored as salted scrypt hashes. Deactivated accounts keep their history; nothing is deleted.
CREATE TABLE IF NOT EXISTS yts_qa_users (
    id bigserial PRIMARY KEY,
    username varchar(40) NOT NULL CHECK (username ~ '^[a-z0-9][a-z0-9._-]{1,39}$'),
    display_name text NOT NULL CHECK (length(display_name) BETWEEN 2 AND 60),
    password_hash text NOT NULL,
    is_admin boolean NOT NULL DEFAULT false,
    session_stamp bigint NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    last_login_at timestamptz,
    deactivated_at timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS yts_qa_users_username_idx ON yts_qa_users(username);
ALTER TABLE yts_issues ADD COLUMN IF NOT EXISTS qa_user_id bigint REFERENCES yts_qa_users(id);
ALTER TABLE yts_issue_events ADD COLUMN IF NOT EXISTS qa_user_id bigint REFERENCES yts_qa_users(id);
-- General issues: one issue about several strategies (possibly across collections). They are stored with
-- collection 'general' and list the strategies and their collections; an empty list means all YT strategies.
ALTER TABLE yts_issues ADD COLUMN IF NOT EXISTS strategies text[] NOT NULL DEFAULT '{}';
ALTER TABLE yts_issues ADD COLUMN IF NOT EXISTS collections text[] NOT NULL DEFAULT '{}';
CREATE INDEX IF NOT EXISTS yts_issues_strategies_idx ON yts_issues USING gin (strategies);
CREATE INDEX IF NOT EXISTS yts_issues_collections_idx ON yts_issues USING gin (collections);
-- Photos and files attached to an issue. Removing a file is soft (deleted_at); the bytes stay in the database.
CREATE TABLE IF NOT EXISTS yts_attachments (
    id bigserial PRIMARY KEY,
    issue_id bigint NOT NULL REFERENCES yts_issues(id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    content_type varchar(100) NOT NULL,
    size integer NOT NULL CHECK (size BETWEEN 1 AND 10485760),
    data bytea NOT NULL,
    uploader_name text NOT NULL,
    qa_user_id bigint REFERENCES yts_qa_users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    deleted_by bigint REFERENCES yts_qa_users(id)
);
CREATE INDEX IF NOT EXISTS yts_attachments_issue_idx ON yts_attachments(issue_id, created_at);
ALTER TABLE yts_issue_events DROP CONSTRAINT IF EXISTS yts_issue_events_action_check;
ALTER TABLE yts_issue_events ADD CONSTRAINT yts_issue_events_action_check
    CHECK (action IN ('created','edited','status','comment','deleted','file'));
