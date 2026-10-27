//! Group assignments: only the leader submits, the review is shared, overrides are per student.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

#[tokio::test]
async fn group_leader_submits_and_members_share_the_grade() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, week_id, _) = published_course(&mut admin).await;
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    let lead = create_student(&mut admin, "lead@example.com", &course_id).await;
    let mate = create_student(&mut admin, "mate@example.com", &course_id).await;
    create_student(&mut admin, "solo@example.com", &course_id).await;
    let aid = admin
        .post(
            "/api/v1/assignments",
            json!({ "course_id": course_id, "title": "Project proposal", "kind": "written", "status": "published", "is_group": true }),
        )
        .await
        .assert(StatusCode::CREATED)
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    let gid = admin
        .post(&format!("/api/v1/assignments/{aid}/groups"), json!({ "name": "Team A", "leader_id": lead, "member_ids": [mate] }))
        .await
        .assert(StatusCode::CREATED)
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    // A student can only be in one group per assignment.
    admin
        .post(&format!("/api/v1/assignments/{aid}/groups"), json!({ "name": "Team B", "leader_id": mate, "member_ids": [] }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);

    let mut l = app.client();
    l.login("lead@example.com", "student-pass-1").await;
    let mut m = app.client();
    m.login("mate@example.com", "student-pass-1").await;
    let mut solo = app.client();
    solo.login("solo@example.com", "student-pass-1").await;

    // Only the leader can work on the submission.
    m.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "x" })).await.assert(StatusCode::CONFLICT);
    solo.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "x" })).await.assert(StatusCode::CONFLICT);
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}")).await.body["accepts_submissions"], false);
    l.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "Our proposal" })).await.assert(StatusCode::OK);
    let sub_id = l.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::OK).body["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mine = m.get(&format!("/api/v1/assignments/{aid}/submission")).await.assert(StatusCode::OK);
    assert_eq!(mine.body["status"], "submitted");
    assert_eq!(mine.body["revisions"][0]["body_md"], "Our proposal");
    assert_eq!(mine.body["can_edit"], false);
    assert_eq!(mine.body["group"]["name"], "Team A");
    let mirror_id = mine.body["id"].as_str().unwrap().to_string();
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}")).await.body["state"], "submitted");

    // The review queue shows the group once.
    let queue = admin.get(&format!("/api/v1/submissions?assignment_id={aid}")).await.assert(StatusCode::OK);
    assert_eq!(queue.body.as_array().unwrap().len(), 1);
    assert_eq!(queue.body[0]["group_name"], "Team A");
    admin
        .post(&format!("/api/v1/submissions/{mirror_id}/review"), json!({ "action": "return", "score": 80 }))
        .await
        .assert(StatusCode::CONFLICT);
    admin.post(&format!("/api/v1/submissions/{sub_id}/review"), json!({ "action": "return", "score": 80 })).await.assert(StatusCode::OK);
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["grade"]["score"], 80.0);

    // Individual overrides, including the leader's, leave the group grade alone.
    let ov = |student: &str| format!("/api/v1/assignments/{aid}/groups/{gid}/members/{student}/override");
    admin.put(&ov(&mate), json!({ "score": 95 })).await.assert(StatusCode::OK);
    admin.put(&ov(&lead), json!({ "score": 70 })).await.assert(StatusCode::OK);
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["grade"]["score"], 95.0);
    assert_eq!(l.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["grade"]["score"], 70.0);
    assert_eq!(admin.get(&format!("/api/v1/submissions/{sub_id}")).await.body["grade"]["score"], 80.0);
    // Regrading the group keeps the overrides.
    admin.post(&format!("/api/v1/submissions/{sub_id}/review"), json!({ "action": "return", "score": 85 })).await.assert(StatusCode::OK);
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["grade"]["score"], 95.0);
    admin.put(&ov(&mate), json!({ "score": null })).await.assert(StatusCode::OK);
    assert_eq!(m.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["grade"]["score"], 85.0);
    let gb = admin.get(&format!("/api/v1/courses/{course_id}/gradebook")).await.assert(StatusCode::OK);
    let cells: Vec<f64> = gb.body["rows"].as_array().unwrap().iter().filter_map(|r| r["cells"][&aid]["score"].as_f64()).collect();
    assert_eq!(cells.len(), 2, "both members are graded in the gradebook");

    // A new leader takes over the group's submission.
    admin
        .put(&format!("/api/v1/assignments/{aid}/groups/{gid}"), json!({ "name": "Team A", "leader_id": mate, "member_ids": [lead] }))
        .await
        .assert(StatusCode::OK);
    let mine = m.get(&format!("/api/v1/assignments/{aid}/submission")).await;
    assert_eq!(mine.body["id"], sub_id);
    assert_eq!(mine.body["mirror_of"], serde_json::Value::Null);
    let theirs = l.get(&format!("/api/v1/assignments/{aid}/submission")).await;
    assert_eq!(theirs.body["mirror_of"], sub_id);
    assert_eq!(theirs.body["revisions"][0]["body_md"], "Our proposal");
    assert_eq!(admin.get(&format!("/api/v1/submissions?assignment_id={aid}")).await.body.as_array().unwrap().len(), 1);

    // Groups must be removed before group submission can be turned off.
    admin.patch(&format!("/api/v1/assignments/{aid}"), json!({ "is_group": false })).await.assert(StatusCode::UNPROCESSABLE_ENTITY);
    admin.delete(&format!("/api/v1/assignments/{aid}/groups/{gid}")).await.assert(StatusCode::NO_CONTENT);
    m.get(&format!("/api/v1/assignments/{aid}/submission")).await.assert(StatusCode::OK);
    assert_eq!(l.get(&format!("/api/v1/assignments/{aid}/submission")).await.body["status"], "not_started");
}
