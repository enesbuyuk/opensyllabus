//! The ten critical flows from the specification, end to end, plus data isolation.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

#[tokio::test]
async fn critical_learning_and_assignment_flow() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;

    // 1. Instructor creates a student (enrolled in a course).
    let (course_id, week_id, lessons) = published_course(&mut admin).await;
    create_student(&mut admin, "ada@example.com", &course_id).await;

    // 2. Student logs in.
    let mut s = app.client();
    s.login("ada@example.com", "student-pass-1").await.assert(StatusCode::OK);

    // Drafts are invisible to students.
    s.get(&format!("/api/v1/weeks/{week_id}")).await.assert(StatusCode::NOT_FOUND);
    s.get(&format!("/api/v1/lessons/{}", lessons[0])).await.assert(StatusCode::NOT_FOUND);

    // 3. Instructor publishes the week and a lesson.
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    admin
        .patch(
            &format!("/api/v1/lessons/{}", lessons[0]),
            json!({ "status": "published", "title": "Robotics intro", "content_md": "# Hello" }),
        )
        .await
        .assert(StatusCode::OK);
    let week = s.get(&format!("/api/v1/weeks/{week_id}")).await.assert(StatusCode::OK);
    assert_eq!(week.body["lessons"].as_array().unwrap().len(), 1, "only published lessons are listed");
    s.get(&format!("/api/v1/lessons/{}", lessons[1])).await.assert(StatusCode::NOT_FOUND);
    let lesson = s.get(&format!("/api/v1/lessons/{}", lessons[0])).await.assert(StatusCode::OK);
    assert_eq!(lesson.body["content_md"], "# Hello");

    // 4. Student completes the lesson.
    s.post(&format!("/api/v1/lessons/{}/complete", lessons[0]), json!({})).await.assert(StatusCode::NO_CONTENT);
    let p = s.get("/api/v1/me/progress").await.assert(StatusCode::OK);
    assert_eq!(p.body[0]["lessons_completed"], 1);
    assert_eq!(p.body[0]["lessons_total"], 1);
    assert_eq!(p.body[0]["completion_percent"], 100.0);

    // 5. Instructor creates an assignment with a rubric.
    let a = admin
        .post(
            "/api/v1/assignments",
            json!({
                "course_id": course_id, "week_id": week_id, "title": "Ohm's law lab", "kind": "lab_report",
                "max_grade": 10, "due_at": "2099-01-01T12:00:00+03:00", "status": "published",
                "rubric": [{ "title": "Measurements", "max_points": 6 }, { "title": "Analysis", "max_points": 4 }]
            }),
        )
        .await
        .assert(StatusCode::CREATED);
    let aid = a.body["id"].as_str().unwrap().to_string();
    let detail = s.get(&format!("/api/v1/assignments/{aid}")).await.assert(StatusCode::OK);
    assert_eq!(detail.body["state"], "not_started");
    assert_eq!(detail.body["due_at"], "2099-01-01T09:00:00.000Z", "deadlines are stored in UTC");
    // A new published assignment adds to the denominator: progress drops to 50 %.
    let p = s.get("/api/v1/me/progress").await;
    assert_eq!(p.body[0]["completion_percent"], 50.0);

    // 6. Student saves a draft.
    s.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "lab_report": { "experiment_title": "Ohm" } }))
        .await
        .assert(StatusCode::OK);
    assert_eq!(s.get(&format!("/api/v1/assignments/{aid}")).await.body["state"], "draft");
    // Incomplete lab reports cannot be submitted.
    s.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::BAD_REQUEST);
    s.upload(&format!("/api/v1/assignments/{aid}/submission/attachments"), "data.csv", b"V,I\n1,0.01\n").await.assert(StatusCode::CREATED);
    s.put(
        &format!("/api/v1/assignments/{aid}/submission"),
        json!({ "lab_report": { "experiment_title": "Ohm", "objective": "Verify V=IR", "procedure": "Sweep V", "results": "Linear", "conclusion": "Holds" } }),
    )
    .await
    .assert(StatusCode::OK);

    // 7. Student submits.
    let sub = s.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::OK);
    let sub_id = sub.body["id"].as_str().unwrap().to_string();
    assert_eq!(sub.body["status"], "submitted");
    assert_eq!(sub.body["attempt"], 1);
    // Finalized work is locked.
    s.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "edit" })).await.assert(StatusCode::CONFLICT);
    // Submitting does not complete the assignment.
    assert_eq!(s.get("/api/v1/me/progress").await.body[0]["assignments_completed"], 0);

    // 8. Instructor reviews the submission.
    let queue = admin.get("/api/v1/submissions?status=pending").await.assert(StatusCode::OK);
    assert_eq!(queue.body.as_array().unwrap().len(), 1);
    admin.post(&format!("/api/v1/submissions/{sub_id}/start-review"), json!({})).await.assert(StatusCode::NO_CONTENT);
    let rubric = admin.get(&format!("/api/v1/assignments/{aid}")).await.body["rubric"].clone();
    let (r1, r2) = (rubric[0]["id"].as_str().unwrap(), rubric[1]["id"].as_str().unwrap());
    admin
        .post(
            &format!("/api/v1/submissions/{sub_id}/review"),
            json!({ "action": "save", "rubric_scores": { r1: 5, r2: 3 }, "feedback": "Draft note" }),
        )
        .await
        .assert(StatusCode::OK);
    // Unreleased grades and feedback are hidden from the student.
    let mine = s.get(&format!("/api/v1/submissions/{sub_id}")).await.assert(StatusCode::OK);
    assert!(mine.body["grade"].is_null());
    assert_eq!(mine.body["feedback"].as_array().unwrap().len(), 0);
    assert_eq!(mine.body["status"], "under_review");
    admin
        .post(&format!("/api/v1/submissions/{sub_id}/review"), json!({ "action": "return", "rubric_scores": { r1: 7, r2: 3 } }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);

    // 9. Instructor assigns a grade and returns it.
    admin
        .post(
            &format!("/api/v1/submissions/{sub_id}/review"),
            json!({ "action": "return", "rubric_scores": { r1: 6, r2: 3 }, "feedback": "Great measurements." }),
        )
        .await
        .assert(StatusCode::OK);

    // 10. Student views their feedback and grade.
    let mine = s.get(&format!("/api/v1/submissions/{sub_id}")).await.assert(StatusCode::OK);
    assert_eq!(mine.body["status"], "graded");
    assert_eq!(mine.body["grade"]["score"], 9.0);
    assert_eq!(mine.body["feedback"][0]["body_md"], "Great measurements.");
    let p = s.get("/api/v1/me/progress").await;
    assert_eq!(p.body[0]["assignments_completed"], 1);
    assert_eq!(p.body[0]["completion_percent"], 100.0);
    assert_eq!(p.body[0]["average_grade_percent"], 90.0);
    let notes = s.get("/api/v1/notifications").await.assert(StatusCode::OK);
    let kinds: Vec<&str> = notes.body["items"].as_array().unwrap().iter().map(|n| n["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"assignment_graded") && kinds.contains(&"assignment_published") && kinds.contains(&"lesson_available"));

    // Gradebook and CSV export reflect the grade.
    let gb = admin.get(&format!("/api/v1/courses/{course_id}/gradebook")).await.assert(StatusCode::OK);
    assert_eq!(gb.body["rows"][0]["cells"][&aid]["score"], 9.0);
    let csv = admin.get(&format!("/api/v1/courses/{course_id}/gradebook.csv")).await.assert(StatusCode::OK);
    assert!(String::from_utf8(csv.raw).unwrap().contains("\"ada\""));
}

#[tokio::test]
async fn revision_request_reopens_submission() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, week_id, _) = published_course(&mut admin).await;
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    create_student(&mut admin, "s@example.com", &course_id).await;
    let aid = admin
        .post("/api/v1/assignments", json!({ "course_id": course_id, "title": "Essay", "kind": "written", "status": "published" }))
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut s = app.client();
    s.login("s@example.com", "student-pass-1").await;
    s.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::BAD_REQUEST);
    s.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "First try" })).await.assert(StatusCode::OK);
    let sub_id = s.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.body["id"].as_str().unwrap().to_string();

    admin
        .post(&format!("/api/v1/submissions/{sub_id}/review"), json!({ "action": "request_revision" }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);
    admin
        .post(&format!("/api/v1/submissions/{sub_id}/review"), json!({ "action": "request_revision", "feedback": "Add references." }))
        .await
        .assert(StatusCode::OK);
    assert_eq!(s.get(&format!("/api/v1/assignments/{aid}")).await.body["state"], "revision_requested");
    s.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "Second try with references" }))
        .await
        .assert(StatusCode::OK);
    let r = s.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::OK);
    assert_eq!(r.body["attempt"], 2);
    let d = admin.get(&format!("/api/v1/submissions/{sub_id}")).await;
    assert_eq!(d.body["revisions"].as_array().unwrap().len(), 2, "every attempt is kept in history");
}

#[tokio::test]
async fn late_policy_reject_and_penalty() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    create_student(&mut admin, "s@example.com", &course_id).await;
    let reject = admin
        .post(
            "/api/v1/assignments",
            json!({ "course_id": course_id, "title": "Closed", "kind": "written", "status": "published",
            "due_at": "2020-01-01T00:00:00Z", "late_policy": "reject" }),
        )
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    let penalty = admin
        .post(
            "/api/v1/assignments",
            json!({ "course_id": course_id, "title": "Penalized", "kind": "written", "status": "published",
            "due_at": "2020-01-01T00:00:00Z", "late_policy": "penalty", "late_penalty_percent": 20 }),
        )
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut s = app.client();
    s.login("s@example.com", "student-pass-1").await;
    s.put(&format!("/api/v1/assignments/{reject}/submission"), json!({ "body_md": "x" })).await.assert(StatusCode::CONFLICT);
    s.put(&format!("/api/v1/assignments/{penalty}/submission"), json!({ "body_md": "late work" })).await.assert(StatusCode::OK);
    let sub = s.post(&format!("/api/v1/assignments/{penalty}/submissions"), json!({})).await.assert(StatusCode::OK);
    assert_eq!(sub.body["is_late"], true);
    assert_eq!(s.get(&format!("/api/v1/assignments/{penalty}")).await.body["state"], "late");
    let sid = sub.body["id"].as_str().unwrap();
    admin.post(&format!("/api/v1/submissions/{sid}/review"), json!({ "action": "return", "score": 50 })).await.assert(StatusCode::OK);
    let g = s.get(&format!("/api/v1/submissions/{sid}")).await;
    assert_eq!(g.body["grade"]["score"], 40.0);
    assert_eq!(g.body["grade"]["late_penalty"], 10.0);
}

#[tokio::test]
async fn students_are_isolated_from_each_other() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    create_student(&mut admin, "a@example.com", &course_id).await;
    create_student(&mut admin, "b@example.com", &course_id).await;
    let aid = admin
        .post("/api/v1/assignments", json!({ "course_id": course_id, "title": "Upload", "kind": "file_upload", "status": "published" }))
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut a = app.client();
    a.login("a@example.com", "student-pass-1").await;
    let file = a
        .upload(&format!("/api/v1/assignments/{aid}/submission/attachments"), "notes.txt", b"private notes")
        .await
        .assert(StatusCode::CREATED);
    let file_id = file.body["id"].as_str().unwrap().to_string();
    let sub_id = a.post(&format!("/api/v1/assignments/{aid}/submissions"), json!({})).await.assert(StatusCode::OK).body["id"]
        .as_str()
        .unwrap()
        .to_string();
    a.get(&format!("/api/v1/files/{file_id}")).await.assert(StatusCode::OK);

    let mut b = app.client();
    b.login("b@example.com", "student-pass-1").await;
    b.get(&format!("/api/v1/submissions/{sub_id}")).await.assert(StatusCode::NOT_FOUND);
    b.get(&format!("/api/v1/files/{file_id}")).await.assert(StatusCode::NOT_FOUND);
    b.get(&format!("/api/v1/assignments/{aid}/submissions")).await.assert(StatusCode::FORBIDDEN);
    b.get(&format!("/api/v1/courses/{course_id}/students/x/progress")).await.assert(StatusCode::FORBIDDEN);
    let own = b.get(&format!("/api/v1/assignments/{aid}/submission")).await.assert(StatusCode::OK);
    assert_eq!(own.body["status"], "not_started");
    let rows = b.get(&format!("/api/v1/courses/{course_id}/progress")).await.assert(StatusCode::OK);
    assert_eq!(rows.body.as_array().unwrap().len(), 1, "students only see their own progress row");

    // A student not enrolled in a course cannot see it at all.
    let other = admin.post("/api/v1/courses", json!({ "title": "Other course" })).await.body["id"].as_str().unwrap().to_string();
    admin.patch(&format!("/api/v1/courses/{other}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    b.get(&format!("/api/v1/courses/{other}")).await.assert(StatusCode::NOT_FOUND);
    assert_eq!(b.get("/api/v1/courses").await.body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn instructor_only_resources_are_hidden() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    create_student(&mut admin, "s@example.com", &course_id).await;
    let f = admin.upload("/api/v1/files", "answers.pdf", b"%PDF-1.4 test").await.assert(StatusCode::CREATED);
    let file_id = f.body["id"].as_str().unwrap().to_string();
    admin
        .post(
            "/api/v1/resources",
            json!({ "course_id": course_id, "title": "Answer key", "kind": "pdf", "file_id": file_id, "visibility": "instructor" }),
        )
        .await
        .assert(StatusCode::CREATED);
    admin
        .post(
            "/api/v1/resources",
            json!({ "course_id": course_id, "title": "Datasheet", "kind": "link", "url": "https://example.com/ds.pdf" }),
        )
        .await
        .assert(StatusCode::CREATED);
    let mut s = app.client();
    s.login("s@example.com", "student-pass-1").await;
    let list = s.get("/api/v1/resources").await.assert(StatusCode::OK);
    let titles: Vec<&str> = list.body.as_array().unwrap().iter().map(|r| r["title"].as_str().unwrap()).collect();
    assert_eq!(titles, vec!["Datasheet"]);
    s.get(&format!("/api/v1/files/{file_id}")).await.assert(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn upload_validation() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    admin.upload("/api/v1/files", "fake.pdf", b"not a pdf").await.assert(StatusCode::BAD_REQUEST);
    admin.upload("/api/v1/files", "image.svg", b"<svg onload=alert(1)>").await.assert(StatusCode::BAD_REQUEST);
    admin.upload("/api/v1/files", "binary.csv", b"a,b\0c").await.assert(StatusCode::BAD_REQUEST);
    let ok = admin.upload("/api/v1/files", "../../evil name.txt", b"hello").await.assert(StatusCode::CREATED);
    assert_eq!(ok.body["original_name"], "evil name.txt");
    let id = ok.body["id"].as_str().unwrap();
    let r = admin.get(&format!("/api/v1/files/{id}")).await.assert(StatusCode::OK);
    assert_eq!(r.headers.get("content-type").unwrap(), "text/plain; charset=utf-8");
    assert!(r.headers.get("content-security-policy").unwrap().to_str().unwrap().contains("sandbox"));
}

#[tokio::test]
async fn curriculum_management() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let c = admin
        .post("/api/v1/courses", json!({ "title": "Long Course", "duration_weeks": 48, "days_per_week": 3, "generate_weeks": true }))
        .await
        .assert(StatusCode::CREATED);
    let cid = c.body["id"].as_str().unwrap();
    let cur = admin.get(&format!("/api/v1/courses/{cid}/weeks")).await.assert(StatusCode::OK);
    let weeks = cur.body["weeks"].as_array().unwrap();
    assert_eq!(weeks.len(), 48);
    assert!(weeks.iter().all(|w| w["lesson_count"] == 3));
    let w1 = weeks[0]["id"].as_str().unwrap();
    let dup = admin.post(&format!("/api/v1/weeks/{w1}/duplicate"), json!({ "target_week_number": 49 })).await.assert(StatusCode::CREATED);
    let dup_id = dup.body["id"].as_str().unwrap();
    assert_eq!(admin.get(&format!("/api/v1/weeks/{dup_id}")).await.body["lessons"].as_array().unwrap().len(), 3);
    admin.post(&format!("/api/v1/weeks/{w1}/duplicate"), json!({ "target_week_number": 2 })).await.assert(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn assignments_in_locked_weeks_are_hidden() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, week_id, _) = published_course(&mut admin).await;
    create_student(&mut admin, "s@example.com", &course_id).await;
    let aid = admin
        .post(
            "/api/v1/assignments",
            json!({ "course_id": course_id, "week_id": week_id, "title": "Week task", "kind": "written", "status": "published" }),
        )
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut s = app.client();
    s.login("s@example.com", "student-pass-1").await;
    s.get(&format!("/api/v1/assignments/{aid}")).await.assert(StatusCode::NOT_FOUND);
    s.put(&format!("/api/v1/assignments/{aid}/submission"), json!({ "body_md": "x" })).await.assert(StatusCode::NOT_FOUND);
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    s.get(&format!("/api/v1/assignments/{aid}")).await.assert(StatusCode::OK);
}
