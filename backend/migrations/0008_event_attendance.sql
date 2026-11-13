-- Attendance for instructor-defined calendar events, opt-in per event. A repeating event is marked
-- per occurrence (occurrence_date is YYYY-MM-DD in the event's timezone); a one-off event uses the
-- date it starts on.

ALTER TABLE calendar_events ADD COLUMN takes_attendance INTEGER NOT NULL DEFAULT 0;

CREATE TABLE event_attendance (
    event_id        TEXT NOT NULL REFERENCES calendar_events (id) ON DELETE CASCADE,
    occurrence_date TEXT NOT NULL,
    student_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status          TEXT NOT NULL CHECK (status IN ('present', 'late', 'absent', 'excused')),
    marked_by       TEXT REFERENCES users (id) ON DELETE SET NULL,
    marked_at       TEXT NOT NULL,
    PRIMARY KEY (event_id, occurrence_date, student_id)
);
CREATE INDEX idx_event_attendance_student ON event_attendance (student_id);
