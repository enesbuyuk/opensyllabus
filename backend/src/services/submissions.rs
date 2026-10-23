//! Submission lifecycle:
//!
//! ```text
//! (none) ──save──▶ draft ──submit──▶ submitted ──review opened──▶ under_review
//!                    ▲                   │                            │
//!                    │      (resubmission allowed, before review)     ├──return──▶ graded
//!                    └───────────────────┘                            │
//!   revision_requested ◀────────────── request revision ─────────────┘
//!          └──submit──▶ submitted (attempt + 1)
//! ```

use std::collections::BTreeMap;

use sqlx::SqlitePool;

use crate::{
    errors::{AppError, AppResult},
    models::{
        assignment::{Assignment, RubricCriterion},
        submission::{Feedback, FileMeta, Grade, LabReport, ReviewRequest, SaveDraftRequest, Submission, SubmissionDetail},
        user::{User, UserSummary},
        Json,
    },
    repositories,
    services::notifications::{self, NewNotification},
    util::{is_http_url, new_id, now, to_json},
};

pub const MAX_BODY_CHARS: usize = 100_000;

pub async fn find(pool: &SqlitePool, assignment_id: &str, student_id: &str) -> AppResult<Option<Submission>> {
    Ok(sqlx::query_as("SELECT * FROM submissions WHERE assignment_id = ? AND student_id = ?")
        .bind(assignment_id)
        .bind(student_id)
        .fetch_optional(pool)
        .await?)
}

pub fn is_past_due(a: &Assignment, now: &str) -> bool {
    a.due_at.as_deref().is_some_and(|d| now > d)
}

/// Whether the student may still change their working copy.
pub fn can_edit(a: &Assignment, sub: Option<&Submission>, now: &str) -> bool {
    if a.status != "published" {
        return false;
    }
    let status = sub.map(|s| s.status.as_str()).unwrap_or("draft");
    match status {
        // A revision request explicitly reopens the submission regardless of the deadline.
        "revision_requested" => true,
        "draft" => !(is_past_due(a, now) && a.late_policy == "reject"),
        "submitted" => a.allow_resubmission && !is_past_due(a, now),
        _ => false,
    }
}

async fn get_or_create(pool: &SqlitePool, a: &Assignment, student_id: &str) -> AppResult<Submission> {
    if let Some(s) = find(pool, &a.id, student_id).await? {
        return Ok(s);
    }
    let ts = now();
    sqlx::query(
        "INSERT OR IGNORE INTO submissions (id, assignment_id, student_id, status, created_at, updated_at) VALUES (?, ?, ?, 'draft', ?, ?)",
    )
    .bind(new_id())
    .bind(&a.id)
    .bind(student_id)
    .bind(&ts)
    .bind(&ts)
    .execute(pool)
    .await?;
    find(pool, &a.id, student_id).await?.ok_or(AppError::NotFound("Submission"))
}

/// Ensures the student may edit, creating the draft row on first use.
pub async fn editable_submission(pool: &SqlitePool, a: &Assignment, student: &User) -> AppResult<Submission> {
    let existing = find(pool, &a.id, &student.id).await?;
    if !can_edit(a, existing.as_ref(), &now()) {
        return Err(AppError::Conflict(match existing.as_ref().map(|s| s.status.as_str()) {
            Some("submitted") | Some("under_review") | Some("graded") => {
                "This submission has been finalized and can no longer be edited".into()
            }
            _ => "This assignment no longer accepts submissions".into(),
        }));
    }
    match existing {
        Some(s) => Ok(s),
        None => get_or_create(pool, a, &student.id).await,
    }
}

pub async fn save_draft(pool: &SqlitePool, a: &Assignment, student: &User, req: SaveDraftRequest) -> AppResult<Submission> {
    let sub = editable_submission(pool, a, student).await?;
    let body = req.body_md.unwrap_or(sub.body_md.clone());
    if body.chars().count() > MAX_BODY_CHARS {
        return Err(AppError::field("body_md", "Response is too long"));
    }
    let repo = match req.repository_url {
        Some(u) if u.trim().is_empty() => None,
        Some(u) => {
            if !is_http_url(&u) {
                return Err(AppError::field("repository_url", "Must be a valid http(s) URL"));
            }
            Some(u.trim().to_string())
        }
        None => sub.repository_url.clone(),
    };
    let lab = match req.lab_report {
        Some(l) => {
            if l.fields().iter().any(|(_, v)| v.chars().count() > 50_000) {
                return Err(AppError::field("lab_report", "A lab report section is too long"));
            }
            Some(l)
        }
        None => sub.lab_report.clone().map(|j| j.0),
    };
    // Editing a resubmittable submission withdraws it to draft until submitted again.
    let status = if sub.status == "submitted" { "draft" } else { sub.status.as_str() };
    sqlx::query("UPDATE submissions SET body_md = ?, repository_url = ?, lab_report = ?, status = ?, updated_at = ? WHERE id = ?")
        .bind(&body)
        .bind(&repo)
        .bind(lab.as_ref().map(to_json))
        .bind(status)
        .bind(now())
        .bind(&sub.id)
        .execute(pool)
        .await?;
    Ok(sqlx::query_as("SELECT * FROM submissions WHERE id = ?").bind(&sub.id).fetch_one(pool).await?)
}

/// Requirements that must be met before an attempt can be finalized.
fn validate_for_submit(a: &Assignment, s: &Submission, attachments: usize) -> AppResult<()> {
    let has_body = !s.body_md.trim().is_empty();
    let has_repo = s.repository_url.as_deref().is_some_and(|u| !u.is_empty());
    let has_files = attachments > 0;
    let lab = s.lab_report.as_ref().map(|j| &j.0);
    let msg = match a.kind.as_str() {
        "written" | "research" if !has_body => Some("Write a response before submitting"),
        "file_upload" if !has_files => Some("Attach at least one file before submitting"),
        "repository" if !has_repo => Some("Provide a repository URL before submitting"),
        "code" if !(has_body || has_files || has_repo) => Some("Provide code, attach files or link a repository"),
        "mixed" if !(has_body || has_files || has_repo) => Some("Add a response, link or attachment before submitting"),
        "lab_report" => {
            const REQUIRED: [(&str, &str); 5] = [
                ("experiment_title", "Experiment title"),
                ("objective", "Objective"),
                ("procedure", "Procedure"),
                ("results", "Results"),
                ("conclusion", "Conclusion"),
            ];
            match lab {
                None => Some("Complete the laboratory report before submitting"),
                Some(l) => {
                    let fields = l.fields();
                    let missing = REQUIRED.iter().find(|(key, _)| fields.iter().any(|(k, v)| k == key && v.trim().is_empty()));
                    if let Some((_, name)) = missing {
                        return Err(AppError::bad_request(format!("The lab report section \"{name}\" is required")));
                    }
                    None
                }
            }
        }
        _ => None,
    };
    match msg {
        Some(m) => Err(AppError::bad_request(m)),
        None => Ok(()),
    }
}

pub async fn submit(pool: &SqlitePool, a: &Assignment, student: &User) -> AppResult<Submission> {
    let sub = find(pool, &a.id, &student.id).await?.ok_or_else(|| AppError::bad_request("Save a draft before submitting"))?;
    let ts = now();
    if !can_edit(a, Some(&sub), &ts) || sub.status == "submitted" {
        return Err(AppError::Conflict("This submission cannot be submitted in its current state".into()));
    }
    let files = repositories::submissions::attachments(pool, &sub.id).await?;
    validate_for_submit(a, &sub, files.len())?;
    let late = is_past_due(a, &ts);
    if late && a.late_policy == "reject" && sub.status != "revision_requested" {
        return Err(AppError::Conflict("The deadline has passed and late submissions are not accepted".into()));
    }
    let attempt = sub.attempt + 1;

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO submission_revisions (id, submission_id, attempt, body_md, repository_url, lab_report, attachments, submitted_at, is_late)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(&sub.id)
    .bind(attempt)
    .bind(&sub.body_md)
    .bind(&sub.repository_url)
    .bind(sub.lab_report.as_ref().map(|j| to_json(&j.0)))
    .bind(to_json(&files))
    .bind(&ts)
    .bind(late)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE submissions SET status = 'submitted', attempt = ?, submitted_at = ?, is_late = ?, updated_at = ? WHERE id = ?")
        .bind(attempt)
        .bind(&ts)
        .bind(late)
        .bind(&ts)
        .bind(&sub.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let admins: Vec<String> = sqlx::query_scalar("SELECT id FROM users WHERE role = 'admin' AND is_active = 1").fetch_all(pool).await?;
    notifications::notify_users(
        pool,
        &admins,
        &NewNotification {
            kind: "submission_received",
            title: format!("{} submitted \"{}\"", student.name, a.title),
            body: if late { "Submitted after the deadline.".into() } else { format!("Attempt {attempt}.") },
            link: Some(format!("/admin/reviews?submission={}", sub.id)),
            dedupe: Some(format!("submission:{}:{attempt}", sub.id)),
        },
    )
    .await?;

    Ok(sqlx::query_as("SELECT * FROM submissions WHERE id = ?").bind(&sub.id).fetch_one(pool).await?)
}

/// Marks a freshly submitted attempt as being reviewed.
pub async fn mark_under_review(pool: &SqlitePool, submission_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE submissions SET status = 'under_review', updated_at = ? WHERE id = ? AND status = 'submitted'")
        .bind(now())
        .bind(submission_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn compute_score(a: &Assignment, rubric: &[RubricCriterion], req: &ReviewRequest) -> AppResult<Option<(f64, BTreeMap<String, f64>)>> {
    let mut rubric_scores = BTreeMap::new();
    for (k, v) in &req.rubric_scores {
        let c = rubric.iter().find(|c| &c.id == k).ok_or_else(|| AppError::field("rubric_scores", "Unknown rubric criterion"))?;
        if !v.is_finite() || *v < 0.0 || *v > c.max_points {
            return Err(AppError::field("rubric_scores", &format!("\"{}\" must be between 0 and {}", c.title, c.max_points)));
        }
        rubric_scores.insert(k.clone(), *v);
    }
    let score = match req.score {
        Some(s) => Some(s),
        None if !rubric_scores.is_empty() => Some(rubric_scores.values().sum()),
        None => None,
    };
    match score {
        Some(s) if !s.is_finite() || s < 0.0 || s > a.max_grade => {
            Err(AppError::field("score", &format!("Score must be between 0 and {}", a.max_grade)))
        }
        Some(s) => Ok(Some(((s * 100.0).round() / 100.0, rubric_scores))),
        None => Ok(None),
    }
}

pub async fn review(pool: &SqlitePool, sub: &Submission, a: &Assignment, reviewer: &User, req: ReviewRequest) -> AppResult<()> {
    if sub.status == "draft" && sub.attempt == 0 {
        return Err(AppError::Conflict("This submission has not been submitted yet".into()));
    }
    let rubric = repositories::assignments::rubric(pool, &a.id).await?;
    let scored = compute_score(a, &rubric, &req)?;
    let feedback = req.feedback.as_deref().map(str::trim).filter(|f| !f.is_empty()).map(str::to_string);
    if feedback.as_ref().is_some_and(|f| f.chars().count() > MAX_BODY_CHARS) {
        return Err(AppError::field("feedback", "Feedback is too long"));
    }
    let ts = now();
    let action = req.action.as_str();
    let (new_status, release) = match action {
        "save" if sub.status == "graded" => {
            return Err(AppError::Conflict("This review was already returned; use \"Return to student\" to update it".into()))
        }
        "save" => (if sub.status == "submitted" { "under_review" } else { sub.status.as_str() }, false),
        "return" => {
            if scored.is_none() {
                return Err(AppError::field("score", "Enter a grade before returning the submission"));
            }
            ("graded", true)
        }
        "request_revision" => {
            if feedback.is_none() {
                return Err(AppError::field("feedback", "Explain what needs to be revised"));
            }
            ("revision_requested", true)
        }
        _ => return Err(AppError::field("action", "Unknown review action")),
    };

    let mut tx = pool.begin().await?;
    if let Some((raw, rubric_scores)) = &scored {
        let penalty = if sub.is_late && a.late_policy == "penalty" {
            (raw * a.late_penalty_percent as f64 / 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };
        let final_score = (raw - penalty).max(0.0);
        let released = (action == "return").then_some(ts.as_str());
        sqlx::query(
            "INSERT INTO grades (id, submission_id, score, max_score, rubric_scores, late_penalty, graded_by, graded_at, released_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(submission_id) DO UPDATE SET score = excluded.score, max_score = excluded.max_score,
               rubric_scores = excluded.rubric_scores, late_penalty = excluded.late_penalty, graded_by = excluded.graded_by,
               graded_at = excluded.graded_at, released_at = excluded.released_at",
        )
        .bind(new_id())
        .bind(&sub.id)
        .bind(final_score)
        .bind(a.max_grade)
        .bind(to_json(rubric_scores))
        .bind(penalty)
        .bind(&reviewer.id)
        .bind(&ts)
        .bind(released)
        .execute(&mut *tx)
        .await?;
    }
    if action == "request_revision" {
        // A revision request makes any earlier grade provisional again.
        sqlx::query("UPDATE grades SET released_at = NULL WHERE submission_id = ?").bind(&sub.id).execute(&mut *tx).await?;
    }

    // Keep at most one unreleased (draft) feedback entry per attempt.
    sqlx::query("DELETE FROM feedback WHERE submission_id = ? AND attempt = ? AND released_at IS NULL")
        .bind(&sub.id)
        .bind(sub.attempt)
        .execute(&mut *tx)
        .await?;
    if let Some(body) = &feedback {
        sqlx::query(
            "INSERT INTO feedback (id, submission_id, author_id, attempt, kind, body_md, released_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&sub.id)
        .bind(&reviewer.id)
        .bind(sub.attempt)
        .bind(if action == "request_revision" { "revision_request" } else { "comment" })
        .bind(body)
        .bind(release.then_some(ts.as_str()))
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE submissions SET status = ?, updated_at = ? WHERE id = ?")
        .bind(new_status)
        .bind(&ts)
        .bind(&sub.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let link = Some(format!("/assignments/{}", a.id));
    match action {
        "return" => {
            notifications::notify_users(
                pool,
                std::slice::from_ref(&sub.student_id),
                &NewNotification {
                    kind: "assignment_graded",
                    title: format!("Graded: {}", a.title),
                    body: if feedback.is_some() {
                        "Your grade and feedback are available.".into()
                    } else {
                        "Your grade is available.".into()
                    },
                    link,
                    dedupe: None,
                },
            )
            .await?
        }
        "request_revision" => {
            notifications::notify_users(
                pool,
                std::slice::from_ref(&sub.student_id),
                &NewNotification {
                    kind: "revision_requested",
                    title: format!("Revision requested: {}", a.title),
                    body: "Your instructor asked you to revise and resubmit.".into(),
                    link,
                    dedupe: None,
                },
            )
            .await?
        }
        _ => {}
    }
    Ok(())
}

pub async fn detail(pool: &SqlitePool, sub: Submission, viewer: &User) -> AppResult<SubmissionDetail> {
    let a: Assignment = sqlx::query_as("SELECT * FROM assignments WHERE id = ?").bind(&sub.assignment_id).fetch_one(pool).await?;
    let (course_title, week_number): (String, Option<i64>) =
        sqlx::query_as("SELECT c.title, w.week_number FROM courses c LEFT JOIN weeks w ON w.id = ? WHERE c.id = ?")
            .bind(&a.week_id)
            .bind(&a.course_id)
            .fetch_one(pool)
            .await?;
    let student: UserSummary =
        sqlx::query_as("SELECT id, name, email FROM users WHERE id = ?").bind(&sub.student_id).fetch_one(pool).await?;
    let attachments: Vec<FileMeta> = repositories::submissions::attachments(pool, &sub.id).await?;
    let revisions = sqlx::query_as(
        "SELECT id, attempt, body_md, repository_url, lab_report, attachments, submitted_at, is_late
         FROM submission_revisions WHERE submission_id = ? ORDER BY attempt DESC",
    )
    .bind(&sub.id)
    .fetch_all(pool)
    .await?;
    let grade: Option<Grade> = sqlx::query_as("SELECT * FROM grades WHERE submission_id = ?").bind(&sub.id).fetch_optional(pool).await?;
    let feedback: Vec<Feedback> = sqlx::query_as(
        "SELECT f.id, f.submission_id, f.author_id, u.name AS author_name, f.attempt, f.kind, f.body_md, f.released_at, f.created_at
         FROM feedback f LEFT JOIN users u ON u.id = f.author_id WHERE f.submission_id = ? ORDER BY f.created_at",
    )
    .bind(&sub.id)
    .fetch_all(pool)
    .await?;
    let (grade, feedback) = if viewer.is_admin() {
        (grade, feedback)
    } else {
        (grade.filter(|g| g.released_at.is_some()), feedback.into_iter().filter(|f| f.released_at.is_some()).collect())
    };
    let can_edit = !viewer.is_admin() && can_edit(&a, Some(&sub), &now());
    let rubric = repositories::assignments::rubric(pool, &a.id).await?;
    Ok(SubmissionDetail {
        submission: sub,
        student,
        assignment: a,
        course_title,
        week_number,
        rubric,
        attachments,
        revisions,
        grade,
        feedback,
        can_edit,
    })
}

/// A blank, unsaved submission returned to students who have not started yet.
pub fn empty_for(a: &Assignment, student: &User) -> Submission {
    let ts = now();
    Submission {
        id: String::new(),
        assignment_id: a.id.clone(),
        student_id: student.id.clone(),
        status: "not_started".into(),
        body_md: String::new(),
        repository_url: None,
        lab_report: None::<Json<LabReport>>,
        attempt: 0,
        submitted_at: None,
        is_late: false,
        created_at: ts.clone(),
        updated_at: ts,
    }
}
