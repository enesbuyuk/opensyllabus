//! Group assignments.
//!
//! The instructor splits students into groups per assignment and only the group leader can
//! work on the submission. The leader's row is canonical; every other member gets a mirror row
//! (`submissions.mirror_of`) that `sync` keeps identical to it, including attachments, history,
//! feedback and grade. Progress, gradebook and analytics therefore keep working per student, while
//! the review queue only lists canonical rows. Members share the group grade unless the instructor
//! sets an individual `score_override`.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::{
    errors::{AppError, AppResult},
    models::{
        assignment::Assignment,
        group::{GroupInfo, GroupInput, GroupMember},
        submission::{Grade, Submission},
    },
    util::{new_id, now, to_json},
};

pub const MAX_MEMBERS: usize = 50;

/// The student's group on this assignment as `(group_id, leader_id)`.
pub async fn group_of(pool: &SqlitePool, assignment_id: &str, student_id: &str) -> AppResult<Option<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT g.id, g.leader_id FROM assignment_group_members m JOIN assignment_groups g ON g.id = m.group_id
         WHERE m.assignment_id = ? AND m.student_id = ?",
    )
    .bind(assignment_id)
    .bind(student_id)
    .fetch_optional(pool)
    .await?)
}

/// On a group assignment only the leader may change or submit the group's work.
pub async fn ensure_can_submit(pool: &SqlitePool, a: &Assignment, student_id: &str) -> AppResult<()> {
    if !a.is_group {
        return Ok(());
    }
    match group_of(pool, &a.id, student_id).await? {
        None => Err(AppError::Conflict("You have not been assigned to a group for this assignment yet".into())),
        Some((_, leader)) if leader != student_id => Err(AppError::Conflict("Only your group leader can submit this assignment".into())),
        Some(_) => Ok(()),
    }
}

pub async fn info(pool: &SqlitePool, group_id: &str, admin: bool) -> AppResult<GroupInfo> {
    let (id, assignment_id, name, leader_id, score): (String, String, String, String, Option<f64>) =
        sqlx::query_as("SELECT id, assignment_id, name, leader_id, score FROM assignment_groups WHERE id = ?")
            .bind(group_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AppError::NotFound("Group"))?;
    let mut members: Vec<GroupMember> = sqlx::query_as(
        "SELECT m.student_id, u.name, u.email, m.score_override FROM assignment_group_members m JOIN users u ON u.id = m.student_id
         WHERE m.group_id = ? ORDER BY m.student_id = ? DESC, u.name",
    )
    .bind(&id)
    .bind(&leader_id)
    .fetch_all(pool)
    .await?;
    let submission_id: Option<String> = sqlx::query_scalar("SELECT id FROM submissions WHERE assignment_id = ? AND student_id = ?")
        .bind(&assignment_id)
        .bind(&leader_id)
        .fetch_optional(pool)
        .await?;
    if !admin {
        members.iter_mut().for_each(|m| m.score_override = None);
    }
    Ok(GroupInfo { id, assignment_id, name, leader_id, members, score: score.filter(|_| admin), submission_id })
}

pub async fn info_for(pool: &SqlitePool, a: &Assignment, student_id: &str, admin: bool) -> AppResult<Option<GroupInfo>> {
    if !a.is_group {
        return Ok(None);
    }
    match group_of(pool, &a.id, student_id).await? {
        Some((gid, _)) => Ok(Some(info(pool, &gid, admin).await?)),
        None => Ok(None),
    }
}

pub async fn list(pool: &SqlitePool, assignment_id: &str) -> AppResult<Vec<GroupInfo>> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM assignment_groups WHERE assignment_id = ? ORDER BY name")
        .bind(assignment_id)
        .fetch_all(pool)
        .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(info(pool, &id, true).await?);
    }
    Ok(out)
}

pub async fn member_ids(pool: &SqlitePool, group_id: &str) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT student_id FROM assignment_group_members WHERE group_id = ?").bind(group_id).fetch_all(pool).await?)
}

/// Re-syncs the group after the leader's submission changed.
pub async fn sync_for_leader(pool: &SqlitePool, a: &Assignment, student_id: &str) -> AppResult<()> {
    if !a.is_group {
        return Ok(());
    }
    if let Some((gid, leader)) = group_of(pool, &a.id, student_id).await? {
        if leader == student_id {
            sync(pool, &gid).await?;
        }
    }
    Ok(())
}

/// Makes every member's mirror row an exact copy of the leader's submission and applies the
/// group grade (or each member's override). Idempotent.
pub async fn sync(pool: &SqlitePool, group_id: &str) -> AppResult<()> {
    let Some((assignment_id, leader_id, group_score)): Option<(String, String, Option<f64>)> =
        sqlx::query_as("SELECT assignment_id, leader_id, score FROM assignment_groups WHERE id = ?")
            .bind(group_id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(());
    };
    let members: Vec<(String, Option<f64>)> =
        sqlx::query_as("SELECT student_id, score_override FROM assignment_group_members WHERE group_id = ?")
            .bind(group_id)
            .fetch_all(pool)
            .await?;
    let overrides: HashMap<String, Option<f64>> = members.into_iter().collect();

    let mut tx = pool.begin().await?;
    let canon: Option<Submission> = sqlx::query_as("SELECT * FROM submissions WHERE assignment_id = ? AND student_id = ?")
        .bind(&assignment_id)
        .bind(&leader_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(canon) = canon else {
        // The leader has not started, so there is nothing to mirror yet.
        return Ok(());
    };
    if canon.mirror_of.is_some() {
        sqlx::query("UPDATE submissions SET mirror_of = NULL WHERE id = ?").bind(&canon.id).execute(&mut *tx).await?;
    }

    // Drop the copies of students who left the group.
    let mirrors: Vec<(String, String)> =
        sqlx::query_as("SELECT id, student_id FROM submissions WHERE mirror_of = ?").bind(&canon.id).fetch_all(&mut *tx).await?;
    for (id, student) in &mirrors {
        if !overrides.contains_key(student) {
            sqlx::query("DELETE FROM submissions WHERE id = ?").bind(id).execute(&mut *tx).await?;
        }
    }

    let attachments: Vec<(String, String)> =
        sqlx::query_as("SELECT file_id, created_at FROM submission_attachments WHERE submission_id = ?")
            .bind(&canon.id)
            .fetch_all(&mut *tx)
            .await?;
    #[allow(clippy::type_complexity)]
    let revisions: Vec<(i64, String, Option<String>, Option<String>, String, String, bool)> = sqlx::query_as(
        "SELECT attempt, body_md, repository_url, lab_report, attachments, submitted_at, is_late FROM submission_revisions WHERE submission_id = ?",
    )
    .bind(&canon.id)
    .fetch_all(&mut *tx)
    .await?;
    #[allow(clippy::type_complexity)]
    let feedback: Vec<(Option<String>, i64, String, String, Option<String>, String)> =
        sqlx::query_as("SELECT author_id, attempt, kind, body_md, released_at, created_at FROM feedback WHERE submission_id = ?")
            .bind(&canon.id)
            .fetch_all(&mut *tx)
            .await?;
    let grade: Option<Grade> =
        sqlx::query_as("SELECT * FROM grades WHERE submission_id = ?").bind(&canon.id).fetch_optional(&mut *tx).await?;
    // An override is the student's final grade, so no late penalty is shown on top of it.
    let grade_for = |student: &str, g: &Grade| -> (f64, f64) {
        match overrides.get(student).copied().flatten() {
            Some(s) => (s, 0.0),
            None => (group_score.unwrap_or(g.score), g.late_penalty),
        }
    };

    if let Some(g) = &grade {
        let (score, penalty) = grade_for(&leader_id, g);
        // The canonical row keeps the group's penalty so the review form can show the raw score.
        let penalty = if overrides.get(&leader_id).copied().flatten().is_some() { g.late_penalty } else { penalty };
        sqlx::query("UPDATE grades SET score = ?, late_penalty = ? WHERE id = ?")
            .bind(score)
            .bind(penalty)
            .bind(&g.id)
            .execute(&mut *tx)
            .await?;
    }

    let lab = canon.lab_report.as_ref().map(|j| to_json(&j.0));
    for student in overrides.keys().filter(|s| **s != leader_id) {
        sqlx::query(
            "INSERT INTO submissions (id, assignment_id, student_id, status, body_md, repository_url, lab_report, attempt, submitted_at,
                is_late, created_at, updated_at, mirror_of)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(assignment_id, student_id) DO UPDATE SET status = excluded.status, body_md = excluded.body_md,
               repository_url = excluded.repository_url, lab_report = excluded.lab_report, attempt = excluded.attempt,
               submitted_at = excluded.submitted_at, is_late = excluded.is_late, updated_at = excluded.updated_at,
               mirror_of = excluded.mirror_of",
        )
        .bind(new_id())
        .bind(&assignment_id)
        .bind(student)
        .bind(&canon.status)
        .bind(&canon.body_md)
        .bind(&canon.repository_url)
        .bind(&lab)
        .bind(canon.attempt)
        .bind(&canon.submitted_at)
        .bind(canon.is_late)
        .bind(&canon.created_at)
        .bind(&canon.updated_at)
        .bind(&canon.id)
        .execute(&mut *tx)
        .await?;
        let mid: String = sqlx::query_scalar("SELECT id FROM submissions WHERE assignment_id = ? AND student_id = ?")
            .bind(&assignment_id)
            .bind(student)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM submission_attachments WHERE submission_id = ?").bind(&mid).execute(&mut *tx).await?;
        for (file_id, created_at) in &attachments {
            sqlx::query("INSERT INTO submission_attachments (id, submission_id, file_id, created_at) VALUES (?, ?, ?, ?)")
                .bind(new_id())
                .bind(&mid)
                .bind(file_id)
                .bind(created_at)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("DELETE FROM submission_revisions WHERE submission_id = ?").bind(&mid).execute(&mut *tx).await?;
        for (attempt, body, repo, lab, files, submitted_at, late) in &revisions {
            sqlx::query(
                "INSERT INTO submission_revisions (id, submission_id, attempt, body_md, repository_url, lab_report, attachments, submitted_at, is_late)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id())
            .bind(&mid)
            .bind(attempt)
            .bind(body)
            .bind(repo)
            .bind(lab)
            .bind(files)
            .bind(submitted_at)
            .bind(late)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("DELETE FROM feedback WHERE submission_id = ?").bind(&mid).execute(&mut *tx).await?;
        for (author, attempt, kind, body, released_at, created_at) in &feedback {
            sqlx::query(
                "INSERT INTO feedback (id, submission_id, author_id, attempt, kind, body_md, released_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id())
            .bind(&mid)
            .bind(author)
            .bind(attempt)
            .bind(kind)
            .bind(body)
            .bind(released_at)
            .bind(created_at)
            .execute(&mut *tx)
            .await?;
        }

        match &grade {
            Some(g) => {
                let (score, penalty) = grade_for(student, g);
                sqlx::query(
                    "INSERT INTO grades (id, submission_id, score, max_score, rubric_scores, late_penalty, graded_by, graded_at, released_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(submission_id) DO UPDATE SET score = excluded.score, max_score = excluded.max_score,
                       rubric_scores = excluded.rubric_scores, late_penalty = excluded.late_penalty, graded_by = excluded.graded_by,
                       graded_at = excluded.graded_at, released_at = excluded.released_at",
                )
                .bind(new_id())
                .bind(&mid)
                .bind(score)
                .bind(g.max_score)
                .bind(to_json(&g.rubric_scores.0))
                .bind(penalty)
                .bind(&g.graded_by)
                .bind(&g.graded_at)
                .bind(&g.released_at)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM grades WHERE submission_id = ?").bind(&mid).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Creates (`group_id` = None) or replaces a group. Returns the group id.
pub async fn save(pool: &SqlitePool, a: &Assignment, group_id: Option<&str>, input: GroupInput) -> AppResult<String> {
    if !a.is_group {
        return Err(AppError::bad_request("Turn on group submission for this assignment first"));
    }
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::field("name", "Enter a group name (up to 100 characters)"));
    }
    let mut ids: Vec<String> = vec![input.leader_id.clone()];
    for id in input.member_ids {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() > MAX_MEMBERS {
        return Err(AppError::field("member_ids", &format!("A group can have at most {MAX_MEMBERS} members")));
    }
    let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assignment_groups WHERE assignment_id = ? AND name = ? AND id <> ?")
        .bind(&a.id)
        .bind(&name)
        .bind(group_id.unwrap_or(""))
        .fetch_one(pool)
        .await?;
    if taken > 0 {
        return Err(AppError::field("name", "Another group already uses this name"));
    }
    let old_leader: Option<String> = match group_id {
        Some(gid) => Some(
            sqlx::query_scalar("SELECT leader_id FROM assignment_groups WHERE id = ? AND assignment_id = ?")
                .bind(gid)
                .bind(&a.id)
                .fetch_optional(pool)
                .await?
                .ok_or(AppError::NotFound("Group"))?,
        ),
        None => None,
    };
    let old_canon: Option<String> = match &old_leader {
        Some(l) => {
            sqlx::query_scalar("SELECT id FROM submissions WHERE assignment_id = ? AND student_id = ? AND mirror_of IS NULL")
                .bind(&a.id)
                .bind(l)
                .fetch_optional(pool)
                .await?
        }
        None => None,
    };

    for id in &ids {
        let student: Option<String> = sqlx::query_scalar(
            "SELECT u.name FROM course_enrollments e JOIN users u ON u.id = e.user_id WHERE e.course_id = ? AND e.user_id = ? AND u.role = 'student'",
        )
        .bind(&a.course_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
        let Some(student) = student else {
            return Err(AppError::field("member_ids", "Members must be students enrolled in this course"));
        };
        let other: Option<String> = sqlx::query_scalar(
            "SELECT g.name FROM assignment_group_members m JOIN assignment_groups g ON g.id = m.group_id
             WHERE m.assignment_id = ? AND m.student_id = ? AND m.group_id <> ?",
        )
        .bind(&a.id)
        .bind(id)
        .bind(group_id.unwrap_or(""))
        .fetch_optional(pool)
        .await?;
        if let Some(other) = other {
            return Err(AppError::field("member_ids", &format!("{student} is already in {other}")));
        }
        // Own work would be overwritten by the group's copy. A new leader's work becomes the
        // group's submission, unless the group already has one.
        let own_attempts: Option<i64> =
            sqlx::query_scalar("SELECT attempt FROM submissions WHERE assignment_id = ? AND student_id = ? AND mirror_of IS NULL")
                .bind(&a.id)
                .bind(id)
                .fetch_optional(pool)
                .await?;
        let becomes_canonical = *id == input.leader_id && old_canon.is_none();
        if own_attempts.unwrap_or(0) > 0 && old_leader.as_ref() != Some(id) && !becomes_canonical {
            return Err(AppError::field("member_ids", &format!("{student} has already submitted this assignment on their own")));
        }
    }

    let gid = group_id.map(str::to_string).unwrap_or_else(new_id);
    let mut tx = pool.begin().await?;
    if group_id.is_some() {
        sqlx::query("UPDATE assignment_groups SET name = ?, leader_id = ? WHERE id = ?")
            .bind(&name)
            .bind(&input.leader_id)
            .bind(&gid)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("INSERT INTO assignment_groups (id, assignment_id, name, leader_id, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&gid)
            .bind(&a.id)
            .bind(&name)
            .bind(&input.leader_id)
            .bind(now())
            .execute(&mut *tx)
            .await?;
    }
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT student_id FROM assignment_group_members WHERE group_id = ?").bind(&gid).fetch_all(&mut *tx).await?;
    for id in existing.iter().filter(|e| !ids.contains(e)) {
        sqlx::query("DELETE FROM assignment_group_members WHERE group_id = ? AND student_id = ?")
            .bind(&gid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    for id in ids.iter().filter(|i| !existing.contains(i)) {
        sqlx::query("INSERT INTO assignment_group_members (group_id, assignment_id, student_id) VALUES (?, ?, ?)")
            .bind(&gid)
            .bind(&a.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    // A new leader takes over the group's submission; their own copy (if any) is replaced by it.
    if let (Some(old), Some(canon)) = (&old_leader, &old_canon) {
        if *old != input.leader_id {
            sqlx::query("DELETE FROM submissions WHERE assignment_id = ? AND student_id = ?")
                .bind(&a.id)
                .bind(&input.leader_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE submissions SET student_id = ? WHERE id = ?").bind(&input.leader_id).bind(canon).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    sync(pool, &gid).await?;
    Ok(gid)
}

/// Deletes a group. The leader keeps the submission; the members' copies are removed.
pub async fn delete(pool: &SqlitePool, a: &Assignment, group_id: &str) -> AppResult<()> {
    let leader: String = sqlx::query_scalar("SELECT leader_id FROM assignment_groups WHERE id = ? AND assignment_id = ?")
        .bind(group_id)
        .bind(&a.id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("Group"))?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM submissions WHERE mirror_of = (SELECT id FROM submissions WHERE assignment_id = ? AND student_id = ?)")
        .bind(&a.id)
        .bind(&leader)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM assignment_groups WHERE id = ?").bind(group_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn set_override(pool: &SqlitePool, a: &Assignment, group_id: &str, student_id: &str, score: Option<f64>) -> AppResult<()> {
    if let Some(s) = score {
        if !s.is_finite() || s < 0.0 || s > a.max_grade {
            return Err(AppError::field("score", &format!("Score must be between 0 and {}", a.max_grade)));
        }
    }
    let res =
        sqlx::query("UPDATE assignment_group_members SET score_override = ? WHERE group_id = ? AND assignment_id = ? AND student_id = ?")
            .bind(score.map(|s| (s * 100.0).round() / 100.0))
            .bind(group_id)
            .bind(&a.id)
            .bind(student_id)
            .execute(pool)
            .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Group member"));
    }
    sync(pool, group_id).await
}
