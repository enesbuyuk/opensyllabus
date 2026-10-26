-- Repeating calendar events. A row is a series when repeat_freq is set; occurrences are
-- expanded on read (services::recurrence), at the same wall-clock time in `timezone`.

ALTER TABLE calendar_events ADD COLUMN repeat_freq TEXT CHECK (repeat_freq IN ('daily', 'weekly', 'monthly'));
-- Every N days / weeks / months.
ALTER TABLE calendar_events ADD COLUMN repeat_interval INTEGER NOT NULL DEFAULT 1 CHECK (repeat_interval BETWEEN 1 AND 99);
-- Weekly rules: ISO weekdays, comma separated (1 = Monday ... 7 = Sunday). Empty = the start day.
ALTER TABLE calendar_events ADD COLUMN repeat_weekdays TEXT NOT NULL DEFAULT '';
-- Last occurrence date (inclusive, YYYY-MM-DD in `timezone`); at most one of until / count.
ALTER TABLE calendar_events ADD COLUMN repeat_until TEXT;
ALTER TABLE calendar_events ADD COLUMN repeat_count INTEGER CHECK (repeat_count IS NULL OR repeat_count BETWEEN 1 AND 500);
-- Occurrence dates removed from the series (JSON array of YYYY-MM-DD).
ALTER TABLE calendar_events ADD COLUMN repeat_exceptions TEXT NOT NULL DEFAULT '[]';
-- IANA timezone the series keeps its wall-clock time in.
ALTER TABLE calendar_events ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC';
