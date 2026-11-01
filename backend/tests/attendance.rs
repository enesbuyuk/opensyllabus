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
