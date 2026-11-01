//! Curriculum operations that touch several records: generation, duplication, publishing.

use std::collections::HashMap;

use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::{
    errors::{AppError, AppResult},
    models::{
        course::Course,
        curriculum::{Lesson, Week},
    },
    services::notifications::{self, NewNotification},
    util::{new_id, now},
};

pub async fn next_lesson_position(pool: &SqlitePool, week_id: &str) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(MAX(position) + 1, 0) FROM lessons WHERE week_id = ?").bind(week_id).fetch_one(pool).await?)
}

pub async fn insert_lesson_shells(tx: &mut Transaction<'_, Sqlite>, week_id: &str, days: i64) -> AppResult<()> {
    let ts = now();
    for day in 1..=days {
        sqlx::query("INSERT INTO lessons (id, week_id, day_number, position, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(week_id)
            .bind(day)
            .bind(day - 1)
            .bind(format!("Day {day}"))
            .bind(&ts)
            .bind(&ts)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Creates missing draft weeks 1..=duration. Returns the number created.
pub async fn generate_weeks(pool: &SqlitePool, course: &Course, with_lessons: bool) -> AppResult<i64> {
    let existing: Vec<i64> =
        sqlx::query_scalar("SELECT week_number FROM weeks WHERE course_id = ?").bind(&course.id).fetch_all(pool).await?;
    let ts = now();
    let mut created = 0;
    let mut tx = pool.begin().await?;
    for n in 1..=course.duration_weeks {
        if existing.contains(&n) {
            continue;
        }
        let id = new_id();
        sqlx::query("INSERT INTO weeks (id, course_id, week_number, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&id)
            .bind(&course.id)
            .bind(n)
            .bind(format!("Week {n}"))
            .bind(&ts)
            .bind(&ts)
            .execute(&mut *tx)
            .await?;
        if with_lessons {
            insert_lesson_shells(&mut tx, &id, course.days_per_week).await?;
        }
        created += 1;
    }
    tx.commit().await?;
    Ok(created)
}

async fn copy_resources(
    tx: &mut Transaction<'_, Sqlite>,
    column: &str,
    from_id: &str,
    new_week_id: Option<&str>,
    new_lesson_id: Option<&str>,
) -> AppResult<()> {
    let ts = now();
    let sql = "INSERT INTO resources (id, course_id, module_id, week_id, lesson_id, title, description, kind, file_id, url, content_md, visibility, created_by, created_at, updated_at)
         SELECT ?, course_id, module_id, COALESCE(?, week_id), ?, title, description, kind, file_id, url, content_md, visibility, created_by, ?, ?
         FROM resources WHERE id = ?";
    let ids: Vec<String> =
        sqlx::query_scalar(&format!("SELECT id FROM resources WHERE {column} = ?")).bind(from_id).fetch_all(&mut **tx).await?;
    for rid in ids {
        sqlx::query(sql).bind(new_id()).bind(new_week_id).bind(new_lesson_id).bind(&ts).bind(&ts).bind(&rid).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn copy_lesson(tx: &mut Transaction<'_, Sqlite>, l: &Lesson, week_id: &str, position: i64, title: &str) -> AppResult<String> {
    let id = new_id();
    let ts = now();
    sqlx::query(
        "INSERT INTO lessons (id, week_id, day_number, position, title, summary, content_md, lab_md, video_url, meeting_url, objectives, estimated_minutes, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?)",
    )
    .bind(&id)
    .bind(week_id)
    .bind(l.day_number)
    .bind(position)
    .bind(title)
    .bind(&l.summary)
    .bind(&l.content_md)
    .bind(&l.lab_md)
    .bind(&l.video_url)
    .bind(&l.meeting_url)
    .bind(crate::util::to_json(&l.objectives.0))
    .bind(l.estimated_minutes)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut **tx)
    .await?;
    copy_resources(tx, "lesson_id", &l.id, Some(week_id), Some(&id)).await?;
    Ok(id)
}

/// Copies a week (as drafts) including lessons, week/lesson resources and assignments (without dates).
pub async fn duplicate_week(pool: &SqlitePool, week: &Week, target_number: i64) -> AppResult<String> {
    if !(1..=104).contains(&target_number) {
        return Err(AppError::field("target_week_number", "Week number must be between 1 and 104"));
    }
    let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM weeks WHERE course_id = ? AND week_number = ?")
        .bind(&week.course_id)
        .bind(target_number)
        .fetch_one(pool)
        .await?;
    if taken > 0 {
        return Err(AppError::field("target_week_number", "That week number already exists"));
    }
    let lessons: Vec<Lesson> =
        sqlx::query_as("SELECT * FROM lessons WHERE week_id = ? ORDER BY position").bind(&week.id).fetch_all(pool).await?;
    let ts = now();
    let new_week_id = new_id();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO weeks (id, course_id, module_id, week_number, title, description, objectives, instructor_notes, estimated_minutes, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?)",
    )
    .bind(&new_week_id)
    .bind(&week.course_id)
    .bind(&week.module_id)
    .bind(target_number)
    .bind(format!("{} (copy)", week.title))
    .bind(&week.description)
    .bind(crate::util::to_json(&week.objectives.0))
    .bind(&week.instructor_notes)
    .bind(week.estimated_minutes)
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;

    let mut lesson_map: HashMap<String, String> = HashMap::new();
    for l in &lessons {
        let nid = copy_lesson(&mut tx, l, &new_week_id, l.position, &l.title).await?;
        lesson_map.insert(l.id.clone(), nid);
    }
    // Week-level resources (not attached to a specific lesson).
    let week_resources: Vec<String> =
        sqlx::query_scalar("SELECT id FROM resources WHERE week_id = ? AND lesson_id IS NULL").bind(&week.id).fetch_all(&mut *tx).await?;
    for rid in week_resources {
        copy_resources(&mut tx, "id", &rid, Some(&new_week_id), None).await?;
    }
    // Assignments are copied as drafts without dates so nothing is published accidentally.
    let assignment_ids: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT id, lesson_id FROM assignments WHERE week_id = ?").bind(&week.id).fetch_all(&mut *tx).await?;
    for (aid, lesson_id) in assignment_ids {
        let new_aid = new_id();
        let new_lesson = lesson_id.and_then(|l| lesson_map.get(&l).cloned());
        sqlx::query(
            "INSERT INTO assignments (id, course_id, week_id, lesson_id, title, description, instructions_md, reference_md, kind, max_grade,
                late_policy, late_penalty_percent, allowed_file_types, max_upload_mb, allow_resubmission, status, created_by, created_at, updated_at)
             SELECT ?, course_id, ?, ?, title, description, instructions_md, reference_md, kind, max_grade,
                late_policy, late_penalty_percent, allowed_file_types, max_upload_mb, allow_resubmission, 'draft', created_by, ?, ?
             FROM assignments WHERE id = ?",
        )
        .bind(&new_aid)
        .bind(&new_week_id)
        .bind(new_lesson)
        .bind(&ts)
        .bind(&ts)
        .bind(&aid)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO assignment_rubrics (id, assignment_id, position, title, description, max_points)
             SELECT lower(hex(randomblob(16))), ?, position, title, description, max_points FROM assignment_rubrics WHERE assignment_id = ?",
        )
        .bind(&new_aid)
        .bind(&aid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(new_week_id)
}

pub async fn duplicate_lesson(pool: &SqlitePool, lesson: &Lesson, target_week_id: &str) -> AppResult<String> {
    let position = next_lesson_position(pool, target_week_id).await?;
    let title = if target_week_id == lesson.week_id { format!("{} (copy)", lesson.title) } else { lesson.title.clone() };
    let mut tx = pool.begin().await?;
    let id = copy_lesson(&mut tx, lesson, target_week_id, position, &title).await?;
    tx.commit().await?;
    Ok(id)
}

/// Notifies enrolled students when a lesson becomes visible (published lesson in an open week).
pub async fn notify_lesson_available(pool: &SqlitePool, lesson: &Lesson, week: &Week, today: &str) -> AppResult<()> {
    if lesson.status != "published" || !crate::auth::access::week_is_open(week, today) {
        return Ok(());
    }
    notifications::notify_course(
        pool,
        &week.course_id,
        &NewNotification {
            kind: "lesson_available",
            title: format!("New lesson: {}", lesson.title),
            body: format!("Week {} · Day {}", week.week_number, lesson.day_number),
            link: Some(format!("/lessons/{}", lesson.id)),
            dedupe: Some(format!("lesson:{}", lesson.id)),
        },
    )
    .await
}
