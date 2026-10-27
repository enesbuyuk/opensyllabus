-- Group assignments: the instructor splits students into groups per assignment and only the
-- group leader submits. The leader's submission is canonical; every other member gets a mirror
-- row (submissions.mirror_of) kept identical by services::groups::sync, so progress, gradebook and
-- analytics keep working per student. Members share the group grade unless score_override is set.

ALTER TABLE assignments ADD COLUMN is_group INTEGER NOT NULL DEFAULT 0 CHECK (is_group IN (0, 1));

CREATE TABLE assignment_groups (
    id            TEXT PRIMARY KEY NOT NULL,
    assignment_id TEXT NOT NULL REFERENCES assignments (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    leader_id     TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    score         REAL,  -- group grade after any late penalty, before individual overrides
    created_at    TEXT NOT NULL,
    UNIQUE (assignment_id, name)
);

CREATE TABLE assignment_group_members (
    group_id       TEXT NOT NULL REFERENCES assignment_groups (id) ON DELETE CASCADE,
    assignment_id  TEXT NOT NULL REFERENCES assignments (id) ON DELETE CASCADE,
    student_id     TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    score_override REAL,  -- final grade for this student instead of the group grade
    PRIMARY KEY (group_id, student_id),
    UNIQUE (assignment_id, student_id)
);

ALTER TABLE submissions ADD COLUMN mirror_of TEXT REFERENCES submissions (id) ON DELETE CASCADE;
CREATE INDEX idx_submissions_mirror_of ON submissions (mirror_of);
