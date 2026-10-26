-- Weekly class times of a course: course day N (lessons.day_number) meets on an ISO weekday
-- (1 = Monday) at a wall-clock time in `timezone`. Lesson dates follow from courses.start_date
-- (services::schedule); without a start date the schedule is kept but nothing is dated.

CREATE TABLE course_schedule (
    course_id  TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    day_number INTEGER NOT NULL CHECK (day_number BETWEEN 1 AND 7),
    weekday    INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_time TEXT NOT NULL,  -- HH:MM
    end_time   TEXT,           -- HH:MM, after start_time
    location   TEXT NOT NULL DEFAULT '',
    timezone   TEXT NOT NULL,
    PRIMARY KEY (course_id, day_number)
);
