mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};

const WINDOW: &str = "from=2026-10-01T00:00:00Z&to=2027-01-01T00:00:00Z";

async fn event_starts(c: &mut Client<'_>, course_id: &str) -> Vec<String> {
    let r = c.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await.assert(StatusCode::OK);
    r.body.as_array().unwrap().iter().filter(|e| e["event_id"].is_string()).map(|e| e["starts_at"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn weekly_series_expands_skips_and_stops_repeating() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    // Monday 2026-10-19 10:00 Istanbul, Mondays and Wednesdays, 4 times.
    let r = admin
        .post(
            "/api/v1/calendar/events",
            json!({
                "course_id": course_id, "title": "Lab session", "kind": "lab",
                "starts_at": "2026-10-19T10:00:00+03:00", "ends_at": "2026-10-19T12:00:00+03:00",
                "timezone": "Europe/Istanbul",
                "recurrence": { "freq": "weekly", "weekdays": [1, 3], "count": 4 }
            }),
        )
        .await
        .assert(StatusCode::CREATED);
    let id = r.body["id"].as_str().unwrap().to_string();
    assert_eq!(r.body["recurrence"]["weekdays"], json!([1, 3]));

    let starts = event_starts(&mut admin, &course_id).await;
    assert_eq!(starts, ["2026-10-19T07:00:00.000Z", "2026-10-21T07:00:00.000Z", "2026-10-26T07:00:00.000Z", "2026-10-28T07:00:00.000Z"]);

    // Entries carry the series so the editor can show the rule and the first occurrence.
    let r = admin.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await;
    let second: &Value = r.body.as_array().unwrap().iter().find(|e| e["starts_at"] == "2026-10-21T07:00:00.000Z").unwrap();
    assert_eq!(second["id"], format!("event:{id}:2026-10-21"));
    assert_eq!(second["ends_at"], "2026-10-21T09:00:00.000Z");
    assert_eq!(second["series"]["starts_at"], "2026-10-19T07:00:00.000Z");
    assert_eq!(second["series"]["occurrence_date"], "2026-10-21");

    // Students see the occurrences too.
    create_student(&mut admin, "s1@example.com", &course_id).await;
    let mut s = app.client();
    s.login("s1@example.com", "student-pass-1").await.assert(StatusCode::OK);
    assert_eq!(event_starts(&mut s, &course_id).await.len(), 4);
    s.post(&format!("/api/v1/calendar/events/{id}/skip"), json!({ "date": "2026-10-21" })).await.assert(StatusCode::FORBIDDEN);

    // Remove one occurrence.
    admin.post(&format!("/api/v1/calendar/events/{id}/skip"), json!({ "date": "2026-10-21" })).await.assert(StatusCode::OK);
    assert_eq!(event_starts(&mut admin, &course_id).await.len(), 3);

    // Editing other fields keeps the rule; "none" makes it a single event again.
    admin.patch(&format!("/api/v1/calendar/events/{id}"), json!({ "title": "Lab" })).await.assert(StatusCode::OK);
    assert_eq!(event_starts(&mut admin, &course_id).await.len(), 3);
    let r = admin.patch(&format!("/api/v1/calendar/events/{id}"), json!({ "recurrence": { "freq": "none" } })).await.assert(StatusCode::OK);
    assert!(r.body["recurrence"].is_null());
    assert_eq!(event_starts(&mut admin, &course_id).await, ["2026-10-19T07:00:00.000Z"]);
    admin.post(&format!("/api/v1/calendar/events/{id}/skip"), json!({ "date": "2026-10-19" })).await.assert(StatusCode::BAD_REQUEST);

    admin.delete(&format!("/api/v1/calendar/events/{id}")).await.assert(StatusCode::NO_CONTENT);
    assert!(event_starts(&mut admin, &course_id).await.is_empty());
}

#[tokio::test]
async fn open_ended_series_and_until_date() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    admin
        .post(
            "/api/v1/calendar/events",
            json!({ "course_id": course_id, "title": "Office hours", "starts_at": "2026-12-28T14:00:00Z", "timezone": "UTC",
                    "recurrence": { "freq": "daily" } }),
        )
        .await
        .assert(StatusCode::CREATED);
    admin
        .post(
            "/api/v1/calendar/events",
            json!({ "course_id": course_id, "title": "Review", "starts_at": "2026-10-05T09:00:00Z", "timezone": "UTC",
                    "recurrence": { "freq": "monthly", "until": "2026-11-30" } }),
        )
        .await
        .assert(StatusCode::CREATED);
    // Daily from 28 Dec to the end of the window (4 days) + monthly on 5 Oct and 5 Nov.
    assert_eq!(event_starts(&mut admin, &course_id).await.len(), 6);
}

#[tokio::test]
async fn invalid_rules_are_rejected() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    for rule in [
        json!({ "freq": "yearly" }),
        json!({ "freq": "weekly", "interval": 0 }),
        json!({ "freq": "weekly", "weekdays": [8] }),
        json!({ "freq": "daily", "count": 3, "until": "2026-12-01" }),
        json!({ "freq": "daily", "until": "2026-10-01" }),
        json!({ "freq": "daily", "count": 501 }),
    ] {
        let r = admin
            .post(
                "/api/v1/calendar/events",
                json!({ "course_id": course_id, "title": "X", "starts_at": "2026-10-19T07:00:00Z", "recurrence": rule }),
            )
            .await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "rule {rule} body {}", r.body);
    }
}
