-- Custom date filters (Base 1 / Base 2 style): named sets of calendar windows a strategy trades inside or outside.
-- Personal sets belong to their owner; shared sets are uploaded by an admin and visible to every user. Additive and
-- idempotent: bootstrap runs it on every start. Windows are validated by the API ({from, to[, label]} with from < to).
CREATE TABLE IF NOT EXISTS lab_date_set (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner text NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    shared boolean NOT NULL DEFAULT false,
    name varchar(60) NOT NULL CHECK (btrim(name) <> ''),
    windows jsonb NOT NULL CHECK (jsonb_typeof(windows) = 'array' AND jsonb_array_length(windows) BETWEEN 1 AND 200),
    version integer NOT NULL DEFAULT 1 CHECK (version >= 1),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
-- names are unique per owner among personal sets, and unique among shared sets
CREATE UNIQUE INDEX IF NOT EXISTS lab_date_set_owner_name_idx ON lab_date_set(owner, lower(name)) WHERE NOT shared;
CREATE UNIQUE INDEX IF NOT EXISTS lab_date_set_shared_name_idx ON lab_date_set(lower(name)) WHERE shared;
