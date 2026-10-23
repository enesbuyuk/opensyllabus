-- OpenSyllabus: initial schema.
--
-- Conventions:
--   * Primary keys are UUIDv7 strings (time-ordered, globally unique).
--   * Timestamps are UTC ISO-8601 strings ("2026-01-31T09:00:00.000Z"), which sort
--     lexicographically; calendar dates are "YYYY-MM-DD".
--   * JSON columns hold small arrays/objects (objectives, rubric scores, …).
--   * Uploaded binaries are never stored here; `files` only holds metadata and an
--     opaque storage key under the private storage directory.

CREATE TABLE users (
    id                   TEXT PRIMARY KEY NOT NULL,
    email                TEXT NOT NULL UNIQUE COLLATE NOCASE,
    name                 TEXT NOT NULL,
    role                 TEXT NOT NULL CHECK (role IN ('admin', 'student')),
    password_hash        TEXT NOT NULL,
    timezone             TEXT NOT NULL DEFAULT 'Europe/Istanbul',
    bio                  TEXT NOT NULL DEFAULT '',
    is_active            INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    must_change_password INTEGER NOT NULL DEFAULT 0 CHECK (must_change_password IN (0, 1)),
    last_login_at        TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE TABLE sessions (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash   TEXT NOT NULL UNIQUE,
    csrf_token   TEXT NOT NULL,
    ip_address   TEXT,
    user_agent   TEXT,
    created_at   TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    revoked_at   TEXT
);
CREATE INDEX idx_sessions_user ON sessions (user_id);
CREATE INDEX idx_sessions_expires ON sessions (expires_at);

CREATE TABLE password_reset_tokens (
    id         TEXT PRIMARY KEY NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_by TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at    TEXT
);
CREATE INDEX idx_reset_tokens_user ON password_reset_tokens (user_id);

CREATE TABLE files (
    id            TEXT PRIMARY KEY NOT NULL,
    uploaded_by   TEXT NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    original_name TEXT NOT NULL,
    content_type  TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256        TEXT NOT NULL,
    storage_key   TEXT NOT NULL UNIQUE,
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_files_uploader ON files (uploaded_by);

CREATE TABLE courses (
    id             TEXT PRIMARY KEY NOT NULL,
    title          TEXT NOT NULL,
    slug           TEXT NOT NULL UNIQUE,
    description    TEXT NOT NULL DEFAULT '',
    cover_file_id  TEXT REFERENCES files (id) ON DELETE SET NULL,
    difficulty     TEXT NOT NULL DEFAULT 'intermediate'
                   CHECK (difficulty IN ('beginner', 'intermediate', 'advanced')),
    duration_weeks INTEGER NOT NULL DEFAULT 12 CHECK (duration_weeks BETWEEN 1 AND 104),
    days_per_week  INTEGER NOT NULL DEFAULT 3 CHECK (days_per_week BETWEEN 1 AND 7),
    instructor_id  TEXT REFERENCES users (id) ON DELETE SET NULL,
    status         TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),
    start_date     TEXT,
    end_date       TEXT,
    objectives     TEXT NOT NULL DEFAULT '[]',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE course_enrollments (
    id          TEXT PRIMARY KEY NOT NULL,
    course_id   TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    enrolled_at TEXT NOT NULL,
    UNIQUE (course_id, user_id)
);
CREATE INDEX idx_enrollments_user ON course_enrollments (user_id);

CREATE TABLE modules (
    id          TEXT PRIMARY KEY NOT NULL,
    course_id   TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_modules_course ON modules (course_id, position);

CREATE TABLE weeks (
    id                TEXT PRIMARY KEY NOT NULL,
    course_id         TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    module_id         TEXT REFERENCES modules (id) ON DELETE SET NULL,
    week_number       INTEGER NOT NULL CHECK (week_number >= 1),
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    objectives        TEXT NOT NULL DEFAULT '[]',
    instructor_notes  TEXT NOT NULL DEFAULT '',
    estimated_minutes INTEGER NOT NULL DEFAULT 0 CHECK (estimated_minutes >= 0),
    status            TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    available_from    TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    UNIQUE (course_id, week_number)
);
CREATE INDEX idx_weeks_module ON weeks (module_id);

CREATE TABLE lessons (
    id                TEXT PRIMARY KEY NOT NULL,
    week_id           TEXT NOT NULL REFERENCES weeks (id) ON DELETE CASCADE,
    day_number        INTEGER NOT NULL DEFAULT 1 CHECK (day_number BETWEEN 1 AND 7),
    position          INTEGER NOT NULL DEFAULT 0,
    title             TEXT NOT NULL,
    summary           TEXT NOT NULL DEFAULT '',
    content_md        TEXT NOT NULL DEFAULT '',
    lab_md            TEXT NOT NULL DEFAULT '',
    video_url         TEXT,
    objectives        TEXT NOT NULL DEFAULT '[]',
    estimated_minutes INTEGER NOT NULL DEFAULT 0 CHECK (estimated_minutes >= 0),
    status            TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    published_at      TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
CREATE INDEX idx_lessons_week ON lessons (week_id, position);

CREATE TABLE lesson_prerequisites (
    lesson_id              TEXT NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    prerequisite_lesson_id TEXT NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    PRIMARY KEY (lesson_id, prerequisite_lesson_id),
    CHECK (lesson_id <> prerequisite_lesson_id)
);

CREATE TABLE lesson_progress (
    id           TEXT PRIMARY KEY NOT NULL,
    lesson_id    TEXT NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    user_id      TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    completed_at TEXT NOT NULL,
    UNIQUE (lesson_id, user_id)
);
CREATE INDEX idx_lesson_progress_user ON lesson_progress (user_id);

CREATE TABLE assignments (
    id                   TEXT PRIMARY KEY NOT NULL,
    course_id            TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    week_id              TEXT REFERENCES weeks (id) ON DELETE SET NULL,
    lesson_id            TEXT REFERENCES lessons (id) ON DELETE SET NULL,
    title                TEXT NOT NULL,
    description          TEXT NOT NULL DEFAULT '',
    instructions_md      TEXT NOT NULL DEFAULT '',
    reference_md         TEXT NOT NULL DEFAULT '',
    kind                 TEXT NOT NULL DEFAULT 'written'
                         CHECK (kind IN ('written', 'file_upload', 'code', 'lab_report', 'research', 'repository', 'mixed')),
    max_grade            REAL NOT NULL DEFAULT 100 CHECK (max_grade > 0),
    due_at               TEXT,
    publish_at           TEXT,
    late_policy          TEXT NOT NULL DEFAULT 'accept'
                         CHECK (late_policy IN ('accept', 'penalty', 'reject')),
    late_penalty_percent INTEGER NOT NULL DEFAULT 0 CHECK (late_penalty_percent BETWEEN 0 AND 100),
    allowed_file_types   TEXT NOT NULL DEFAULT '[]',
    max_upload_mb        INTEGER NOT NULL DEFAULT 20 CHECK (max_upload_mb BETWEEN 1 AND 100),
    allow_resubmission   INTEGER NOT NULL DEFAULT 0 CHECK (allow_resubmission IN (0, 1)),
    status               TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),
    published_notified   INTEGER NOT NULL DEFAULT 0,
    created_by           TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);
CREATE INDEX idx_assignments_course ON assignments (course_id, due_at);
CREATE INDEX idx_assignments_week ON assignments (week_id);

CREATE TABLE assignment_rubrics (
    id            TEXT PRIMARY KEY NOT NULL,
    assignment_id TEXT NOT NULL REFERENCES assignments (id) ON DELETE CASCADE,
    position      INTEGER NOT NULL DEFAULT 0,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    max_points    REAL NOT NULL CHECK (max_points > 0)
);
CREATE INDEX idx_rubrics_assignment ON assignment_rubrics (assignment_id, position);

-- One submission per (assignment, student). The working copy lives on the
-- submission row; every finalized attempt is frozen into submission_revisions.
CREATE TABLE submissions (
    id             TEXT PRIMARY KEY NOT NULL,
    assignment_id  TEXT NOT NULL REFERENCES assignments (id) ON DELETE CASCADE,
    student_id     TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status         TEXT NOT NULL DEFAULT 'draft'
                   CHECK (status IN ('draft', 'submitted', 'under_review', 'graded', 'revision_requested')),
    body_md        TEXT NOT NULL DEFAULT '',
    repository_url TEXT,
    lab_report     TEXT,
    attempt        INTEGER NOT NULL DEFAULT 0,
    submitted_at   TEXT,
    is_late        INTEGER NOT NULL DEFAULT 0 CHECK (is_late IN (0, 1)),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    UNIQUE (assignment_id, student_id)
);
CREATE INDEX idx_submissions_student ON submissions (student_id);
CREATE INDEX idx_submissions_status ON submissions (status);

CREATE TABLE submission_revisions (
    id             TEXT PRIMARY KEY NOT NULL,
    submission_id  TEXT NOT NULL REFERENCES submissions (id) ON DELETE CASCADE,
    attempt        INTEGER NOT NULL,
    body_md        TEXT NOT NULL DEFAULT '',
    repository_url TEXT,
    lab_report     TEXT,
    attachments    TEXT NOT NULL DEFAULT '[]',
    submitted_at   TEXT NOT NULL,
    is_late        INTEGER NOT NULL DEFAULT 0,
    UNIQUE (submission_id, attempt)
);

CREATE TABLE submission_attachments (
    id            TEXT PRIMARY KEY NOT NULL,
    submission_id TEXT NOT NULL REFERENCES submissions (id) ON DELETE CASCADE,
    file_id       TEXT NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    created_at    TEXT NOT NULL,
    UNIQUE (submission_id, file_id)
);

-- A grade is drafted by the instructor and becomes visible to the student only
-- once `released_at` is set (the review is "returned").
CREATE TABLE grades (
    id            TEXT PRIMARY KEY NOT NULL,
    submission_id TEXT NOT NULL UNIQUE REFERENCES submissions (id) ON DELETE CASCADE,
    score         REAL NOT NULL CHECK (score >= 0),
    max_score     REAL NOT NULL CHECK (max_score > 0),
    rubric_scores TEXT NOT NULL DEFAULT '{}',
    late_penalty  REAL NOT NULL DEFAULT 0,
    graded_by     TEXT REFERENCES users (id) ON DELETE SET NULL,
    graded_at     TEXT NOT NULL,
    released_at   TEXT
);

CREATE TABLE feedback (
    id            TEXT PRIMARY KEY NOT NULL,
    submission_id TEXT NOT NULL REFERENCES submissions (id) ON DELETE CASCADE,
    author_id     TEXT REFERENCES users (id) ON DELETE SET NULL,
    attempt       INTEGER NOT NULL DEFAULT 0,
    kind          TEXT NOT NULL DEFAULT 'comment' CHECK (kind IN ('comment', 'revision_request')),
    body_md       TEXT NOT NULL,
    released_at   TEXT,
    created_at    TEXT NOT NULL
);
CREATE INDEX idx_feedback_submission ON feedback (submission_id);

CREATE TABLE resources (
    id          TEXT PRIMARY KEY NOT NULL,
    course_id   TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    module_id   TEXT REFERENCES modules (id) ON DELETE SET NULL,
    week_id     TEXT REFERENCES weeks (id) ON DELETE SET NULL,
    lesson_id   TEXT REFERENCES lessons (id) ON DELETE SET NULL,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    kind        TEXT NOT NULL
                CHECK (kind IN ('pdf', 'image', 'markdown', 'code', 'link', 'video', 'datasheet', 'lab')),
    file_id     TEXT REFERENCES files (id) ON DELETE SET NULL,
    url         TEXT,
    content_md  TEXT NOT NULL DEFAULT '',
    visibility  TEXT NOT NULL DEFAULT 'students' CHECK (visibility IN ('students', 'instructor')),
    created_by  TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_resources_course ON resources (course_id);
CREATE INDEX idx_resources_week ON resources (week_id);
CREATE INDEX idx_resources_lesson ON resources (lesson_id);
CREATE INDEX idx_resources_file ON resources (file_id);

CREATE TABLE announcements (
    id           TEXT PRIMARY KEY NOT NULL,
    course_id    TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    author_id    TEXT REFERENCES users (id) ON DELETE SET NULL,
    title        TEXT NOT NULL,
    body_md      TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    pinned       INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    published_at TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX idx_announcements_course ON announcements (course_id, published_at);

CREATE TABLE notifications (
    id         TEXT PRIMARY KEY NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    link       TEXT,
    dedupe_key TEXT UNIQUE,
    read_at    TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_notifications_user ON notifications (user_id, created_at);

CREATE TABLE calendar_events (
    id          TEXT PRIMARY KEY NOT NULL,
    course_id   TEXT NOT NULL REFERENCES courses (id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    kind        TEXT NOT NULL DEFAULT 'session' CHECK (kind IN ('session', 'lab', 'milestone', 'other')),
    starts_at   TEXT NOT NULL,
    ends_at     TEXT,
    location    TEXT NOT NULL DEFAULT '',
    created_by  TEXT REFERENCES users (id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    CHECK (ends_at IS NULL OR ends_at >= starts_at)
);
CREATE INDEX idx_events_course ON calendar_events (course_id, starts_at);

CREATE TABLE audit_logs (
    id          TEXT PRIMARY KEY NOT NULL,
    actor_id    TEXT REFERENCES users (id) ON DELETE SET NULL,
    action      TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   TEXT,
    details     TEXT NOT NULL DEFAULT '{}',
    ip_address  TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_audit_created ON audit_logs (created_at);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
