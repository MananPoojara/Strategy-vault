-- Customize book strategies (spec 2026-10-16 §7): lineage of a lab strategy created from a book template.
-- Additive and idempotent (bootstrap runs it on every start). NULL = written from scratch in the lab.
-- Set once at creation from spec.origin; the app never changes it (an edit that changes spec.origin is refused).
ALTER TABLE lab_strategy ADD COLUMN IF NOT EXISTS origin jsonb
    CHECK (origin IS NULL OR (jsonb_typeof(origin) = 'object' AND origin ? 'book' AND origin ? 'method' AND origin ? 'template_version'));
