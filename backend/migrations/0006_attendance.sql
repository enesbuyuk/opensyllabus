-- Attendance per class session. A session is course day N of week W from the class schedule
-- (course_schedule); its date follows from courses.start_date (services::schedule). Keyed by
-- week and day numbers so records survive changes to the week rows or the schedule times.

CREATE TABLE attendance (
    course_id   TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    week_number INTEGER NOT NULL CHECK (week_number >= 1),
    day_number  INTEGER NOT NULL CHECK (day_number BETWEEN 1 AND 7),
    student_id  TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status      TEXT NOT NULL CHECK (status IN ('present', 'late', 'absent', 'excused')),
    marked_by   TEXT REFERENCES users (id) ON DELETE SET NULL,
    marked_at   TEXT NOT NULL,
    PRIMARY KEY (course_id, week_number, day_number, student_id)
);
CREATE INDEX idx_attendance_student ON attendance (student_id, course_id);
