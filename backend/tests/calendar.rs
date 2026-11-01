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

#[tokio::test]
async fn course_schedule_places_lessons_on_the_calendar() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, week_id, lessons) = published_course(&mut admin).await;
    // published_course leaves the week and its lessons as drafts.
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    for l in &lessons {
        admin.patch(&format!("/api/v1/lessons/{l}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    }
    // Wednesday start: week 1 runs Wed 21 Oct .. Tue 27 Oct.
    admin.patch(&format!("/api/v1/courses/{course_id}"), json!({ "start_date": "2026-10-21" })).await.assert(StatusCode::OK);
    // Week 2 stays a draft: students see its class times, not its lessons.
    admin
        .post(&format!("/api/v1/courses/{course_id}/weeks"), json!({ "title": "Week 2", "create_day_lessons": true }))
        .await
        .assert(StatusCode::CREATED);

    let empty = admin.get(&format!("/api/v1/courses/{course_id}/schedule")).await.assert(StatusCode::OK);
    assert_eq!(empty.body["slots"], json!([]));

    let schedule = json!({ "timezone": "Europe/Istanbul", "slots": [
        { "day_number": 1, "weekday": 1, "start_time": "10:00", "end_time": "12:00", "location": "Lab 2" },
        { "day_number": 2, "weekday": 3, "start_time": "14:00", "end_time": null },
        { "day_number": 3, "weekday": 5, "start_time": "09:30", "end_time": "11:00" }
    ]});
    let r = admin.put(&format!("/api/v1/courses/{course_id}/schedule"), schedule).await.assert(StatusCode::OK);
    assert_eq!(r.body["slots"].as_array().unwrap().len(), 3);

    let classes = |body: &Value| -> Vec<(String, String, Value)> {
        body.as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "class")
            .map(|e| (e["starts_at"].as_str().unwrap().to_string(), e["title"].as_str().unwrap().to_string(), e["link"].clone()))
            .collect()
    };
    let r = admin.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await.assert(StatusCode::OK);
    let c = classes(&r.body);
    assert_eq!(
        c.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        [
            "2026-10-21T11:00:00.000Z", // W1 day 2: Wed 14:00
            "2026-10-23T06:30:00.000Z", // W1 day 3: Fri 09:30
            "2026-10-26T07:00:00.000Z", // W1 day 1: Mon 10:00
            "2026-10-28T11:00:00.000Z", // W2 day 2
            "2026-10-30T06:30:00.000Z", // W2 day 3
            "2026-11-02T07:00:00.000Z", // W2 day 1
        ]
    );
    let monday = r.body.as_array().unwrap().iter().find(|e| e["starts_at"] == "2026-10-26T07:00:00.000Z").unwrap();
    assert_eq!(monday["ends_at"], "2026-10-26T09:00:00.000Z");
    assert_eq!(monday["location"], "Lab 2");
    assert_eq!(monday["link"], format!("/lessons/{}", lessons[0]));

    create_student(&mut admin, "s1@example.com", &course_id).await;
    let mut s = app.client();
    s.login("s1@example.com", "student-pass-1").await.assert(StatusCode::OK);
    let r = s.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await.assert(StatusCode::OK);
    let c = classes(&r.body);
    assert_eq!(c.len(), 6, "students see every class time");
    assert!(c[..3].iter().all(|x| x.1.starts_with("W1 · ") && x.2.is_string()));
    assert!(c[3..].iter().all(|x| x.1.starts_with("Week 2, day ") && x.2.is_null()), "draft week: no lesson title or link");
    s.get(&format!("/api/v1/courses/{course_id}/schedule")).await.assert(StatusCode::OK);
    s.put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "UTC", "slots": [] })).await.assert(StatusCode::FORBIDDEN);

    // Clearing the schedule removes the classes.
    admin.put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "UTC", "slots": [] })).await.assert(StatusCode::OK);
    let r = admin.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await;
    assert!(classes(&r.body).is_empty());
}

#[tokio::test]
async fn invalid_schedules_are_rejected() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, ..) = published_course(&mut admin).await;
    for slots in [
        json!([{ "day_number": 4, "weekday": 1, "start_time": "10:00" }]), // course has 3 days
        json!([{ "day_number": 1, "weekday": 1, "start_time": "10:00" }, { "day_number": 1, "weekday": 2, "start_time": "10:00" }]),
        json!([{ "day_number": 1, "weekday": 8, "start_time": "10:00" }]),
        json!([{ "day_number": 1, "weekday": 1, "start_time": "25:00" }]),
        json!([{ "day_number": 1, "weekday": 1, "start_time": "10:00", "end_time": "09:00" }]),
    ] {
        let r = admin.put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "Europe/Istanbul", "slots": slots })).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "slots {slots} body {}", r.body);
    }
    admin
        .put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "Mars/Base", "slots": [] }))
        .await
        .assert(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn class_links_come_from_the_schedule_and_lessons_can_override_them() {
    let app = TestApp::new().await;
    let mut admin = app.admin().await;
    let (course_id, week_id, lessons) = published_course(&mut admin).await;
    admin.patch(&format!("/api/v1/weeks/{week_id}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    for l in &lessons {
        admin.patch(&format!("/api/v1/lessons/{l}"), json!({ "status": "published" })).await.assert(StatusCode::OK);
    }
    admin.patch(&format!("/api/v1/courses/{course_id}"), json!({ "start_date": "2026-10-21" })).await.assert(StatusCode::OK);

    // Only https links are accepted.
    for bad in ["javascript:alert(1)", "http://meet.example.com/x", "not a link"] {
        let slots = json!([{ "day_number": 1, "weekday": 1, "start_time": "10:00", "meeting_url": bad }]);
        let r = admin.put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "Europe/Istanbul", "slots": slots })).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        let r = admin.patch(&format!("/api/v1/lessons/{}", lessons[0]), json!({ "meeting_url": bad })).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }

    let slots = json!([
        { "day_number": 1, "weekday": 1, "start_time": "10:00", "end_time": "12:00", "meeting_url": "https://meet.example.com/monday" },
        { "day_number": 2, "weekday": 3, "start_time": "14:00" }
    ]);
    let r = admin
        .put(&format!("/api/v1/courses/{course_id}/schedule"), json!({ "timezone": "Europe/Istanbul", "slots": slots }))
        .await
        .assert(StatusCode::OK);
    assert_eq!(r.body["slots"][0]["meeting_url"], "https://meet.example.com/monday");
    assert_eq!(r.body["slots"][1]["meeting_url"], Value::Null);

    // Day 2 has no slot link; the lesson sets its own.
    admin
        .patch(&format!("/api/v1/lessons/{}", lessons[1]), json!({ "meeting_url": "https://zoom.example.com/j/2" }))
        .await
        .assert(StatusCode::OK);

    create_student(&mut admin, "s1@example.com", &course_id).await;
    let mut s = app.client();
    s.login("s1@example.com", "student-pass-1").await.assert(StatusCode::OK);

    let l0 = s.get(&format!("/api/v1/lessons/{}", lessons[0])).await.assert(StatusCode::OK);
    assert_eq!(l0.body["class_meeting_url"], "https://meet.example.com/monday");
    assert_eq!(l0.body["class_starts_at"], "2026-10-26T07:00:00.000Z");
    assert_eq!(l0.body["class_ends_at"], "2026-10-26T09:00:00.000Z");
    let l1 = s.get(&format!("/api/v1/lessons/{}", lessons[1])).await.assert(StatusCode::OK);
    assert_eq!(l1.body["class_meeting_url"], "https://zoom.example.com/j/2");
    let l2 = s.get(&format!("/api/v1/lessons/{}", lessons[2])).await.assert(StatusCode::OK);
    assert_eq!(l2.body["class_meeting_url"], Value::Null, "day 3 has no slot");

    let r = s.get(&format!("/api/v1/calendar?{WINDOW}&course_id={course_id}")).await.assert(StatusCode::OK);
    let link_of = |lesson: &str| {
        r.body.as_array().unwrap().iter().find(|e| e["link"] == format!("/lessons/{lesson}")).map(|e| e["meeting_url"].clone()).unwrap()
    };
    assert_eq!(link_of(&lessons[0]), "https://meet.example.com/monday");
    assert_eq!(link_of(&lessons[1]), "https://zoom.example.com/j/2");

    // Clearing the lesson link falls back to the slot (none for day 2).
    admin.patch(&format!("/api/v1/lessons/{}", lessons[1]), json!({ "meeting_url": "" })).await.assert(StatusCode::OK);
    let l1 = s.get(&format!("/api/v1/lessons/{}", lessons[1])).await.assert(StatusCode::OK);
    assert_eq!(l1.body["class_meeting_url"], Value::Null);
}
