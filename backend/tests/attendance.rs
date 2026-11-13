//! Attendance per scheduled class.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

#[tokio::test]
async fn instructor_marks_classes_and_students_see_their_own_record() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, _, _) = published_course(&mut admin).await;
    let ada = create_student(&mut admin, "ada@example.com", &course_id).await;
    let bob = create_student(&mut admin, "bob@example.com", &course_id).await;

    // Without a schedule there are no classes to mark.
    let att = admin.get(&format!("/api/v1/courses/{course_id}/attendance")).await.assert(StatusCode::OK);
    assert_eq!(att.body["sessions"].as_array().unwrap().len(), 0);

    admin.patch(&format!("/api/v1/courses/{course_id}"), json!({ "start_date": "2020-01-06" })).await.assert(StatusCode::OK);
    admin
        .put(
            &format!("/api/v1/courses/{course_id}/schedule"),
            json!({ "timezone": "Europe/Istanbul", "slots": [
                { "day_number": 1, "weekday": 1, "start_time": "10:00", "end_time": "12:00", "location": "Lab" },
                { "day_number": 2, "weekday": 3, "start_time": "10:00" }
            ] }),
        )
        .await
        .assert(StatusCode::OK);
    let att = admin.get(&format!("/api/v1/courses/{course_id}/attendance")).await.assert(StatusCode::OK);
    assert_eq!(att.body["sessions"].as_array().unwrap().len(), 8, "4 weeks x 2 scheduled days");
    assert_eq!(att.body["sessions"][0]["date"], "2020-01-06");
    assert_eq!(att.body["sessions"][1]["date"], "2020-01-08");

    let url = format!("/api/v1/courses/{course_id}/attendance");
    admin.put(&url, json!({ "week_number": 1, "day_number": 3, "records": [] })).await.assert(StatusCode::BAD_REQUEST);
    admin
        .put(&url, json!({ "week_number": 1, "day_number": 1, "records": [{ "student_id": ada, "status": "sick" }] }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);
    admin
        .put(&url, json!({ "week_number": 1, "day_number": 1, "records": [{ "student_id": ada, "status": "present" }, { "student_id": bob, "status": "absent" }] }))
        .await
        .assert(StatusCode::OK);
    admin
        .put(&url, json!({ "week_number": 1, "day_number": 2, "records": [{ "student_id": ada, "status": "late" }, { "student_id": bob, "status": "excused" }] }))
        .await
        .assert(StatusCode::OK);
    let att = admin
        .put(&url, json!({ "week_number": 2, "day_number": 1, "records": [{ "student_id": ada, "status": "absent" }, { "student_id": bob, "status": "present" }] }))
        .await
        .assert(StatusCode::OK);
    assert_eq!(att.body["sessions"][0]["marked"], 2);
    assert_eq!(att.body["sessions"][0]["key"], "class:1-1");
    let summary = |id: &str| att.body["summaries"].as_array().unwrap().iter().find(|s| s["student_id"] == id).unwrap().clone();
    assert_eq!(summary(&ada)["rate_percent"], 66.7);
    assert_eq!(summary(&bob)["rate_percent"], 50.0, "excused classes are not counted");
    // Clearing a record removes it.
    let att = admin.put(&url, json!({ "week_number": 2, "day_number": 1, "records": [{ "student_id": bob, "status": null }] })).await;
    assert_eq!(att.body["records"].as_array().unwrap().len(), 5);

    let mut s = app.client();
    s.login("ada@example.com", "student-pass-1").await;
    s.get(&url).await.assert(StatusCode::FORBIDDEN);
    let mine = s.get(&format!("{url}/me")).await.assert(StatusCode::OK);
    assert_eq!(mine.body["summary"]["absent"], 1);
    assert_eq!(mine.body["sessions"][0]["status"], "present");
    assert_eq!(mine.body["sessions"].as_array().unwrap().len(), 8, "all past classes are listed");
}

#[tokio::test]
async fn attendance_for_calendar_events() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, _, _) = published_course(&mut admin).await;
    let ada = create_student(&mut admin, "ada@example.com", &course_id).await;
    let url = format!("/api/v1/courses/{course_id}/attendance");

    // Events only show up once they take attendance.
    let r = admin
        .post(
            "/api/v1/calendar/events",
            json!({ "course_id": course_id, "title": "Workshop", "starts_at": "2020-03-02T09:00:00+03:00", "timezone": "Europe/Istanbul",
                    "recurrence": { "freq": "weekly", "count": 3 } }),
        )
        .await
        .assert(StatusCode::CREATED);
    let id = r.body["id"].as_str().unwrap().to_string();
    assert_eq!(r.body["takes_attendance"], false);
    let att = admin.get(&url).await.assert(StatusCode::OK);
    assert!(att.body["sessions"].as_array().unwrap().is_empty());
    admin.put(&url, json!({ "event_id": id, "occurrence_date": "2020-03-02", "records": [] })).await.assert(StatusCode::BAD_REQUEST);

    admin.patch(&format!("/api/v1/calendar/events/{id}"), json!({ "takes_attendance": true })).await.assert(StatusCode::OK);
    let att = admin.get(&url).await.assert(StatusCode::OK);
    let sessions = att.body["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 3, "one session per occurrence");
    assert_eq!(sessions[0]["kind"], "event");
    assert_eq!(sessions[0]["title"], "Workshop");
    assert_eq!(sessions[1]["date"], "2020-03-09");
    assert_eq!(sessions[1]["key"], format!("event:{id}:2020-03-09"));

    admin
        .put(&url, json!({ "event_id": id, "occurrence_date": "2020-03-02", "records": [{ "student_id": ada, "status": "present" }] }))
        .await
        .assert(StatusCode::OK);
    let att = admin
        .put(&url, json!({ "key": format!("event:{id}:2020-03-09"), "records": [{ "student_id": ada, "status": "absent" }] }))
        .await
        .assert(StatusCode::OK);
    assert_eq!(att.body["sessions"][0]["marked"], 1);
    assert_eq!(att.body["summaries"][0]["rate_percent"], 50.0);
    admin
        .put(&url, json!({ "event_id": id, "occurrence_date": "2020-03-04", "records": [{ "student_id": ada, "status": "present" }] }))
        .await
        .assert(StatusCode::BAD_REQUEST);

    let mut s = app.client();
    s.login("ada@example.com", "student-pass-1").await;
    let mine = s.get(&format!("{url}/me")).await.assert(StatusCode::OK);
    assert_eq!(mine.body["sessions"].as_array().unwrap().len(), 3);
    assert_eq!(mine.body["sessions"][1]["status"], "absent");
    assert_eq!(mine.body["summary"]["absent"], 1);

    // A skipped occurrence drops out together with its record.
    admin.post(&format!("/api/v1/calendar/events/{id}/skip"), json!({ "date": "2020-03-09" })).await.assert(StatusCode::OK);
    let att = admin.get(&url).await.assert(StatusCode::OK);
    assert_eq!(att.body["sessions"].as_array().unwrap().len(), 2);
    assert_eq!(att.body["summaries"][0]["rate_percent"], 100.0);

    // Marking and event changes show up in the audit log.
    let logs = admin.get("/api/v1/audit-logs?action=calendar").await.assert(StatusCode::OK);
    assert_eq!(logs.body.as_array().unwrap().len(), 3, "create, update, skip");
    let logs = admin.get("/api/v1/audit-logs?action=attendance&limit=1").await.assert(StatusCode::OK);
    assert_eq!(logs.body.as_array().unwrap().len(), 1);
    let before = logs.body[0]["created_at"].as_str().unwrap().to_string();
    let older =
        admin.get(&format!("/api/v1/audit-logs?action=attendance&before={}", before.replace('+', "%2B"))).await.assert(StatusCode::OK);
    assert_eq!(older.body.as_array().unwrap().len(), 1);

    admin.delete(&format!("/api/v1/calendar/events/{id}")).await.assert(StatusCode::NO_CONTENT);
    let att = admin.get(&url).await.assert(StatusCode::OK);
    assert!(att.body["sessions"].as_array().unwrap().is_empty());
    assert!(att.body["records"].as_array().unwrap().is_empty());
}
