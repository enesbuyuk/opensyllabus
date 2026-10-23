# Architecture and engineering decisions

## Overview

```text
Browser ──HTTPS──▶ Reverse proxy (Caddy/nginx) ──HTTP──▶ opensyllabus (Axum)
                                                        ├─ /api/v1/*   JSON API
                                                        ├─ /api/docs   Swagger UI, /api/openapi.json
                                                        ├─ /api/health, /api/health/ready
                                                        └─ /*          built React app (SPA fallback)
                                                             │
                                          /data/db/opensyllabus.db (SQLite, WAL)   /data/storage/files/** (private uploads)
```

A single process serves both the API and the static frontend, so the browser always talks to one
origin. That keeps cookies `SameSite=Lax`, avoids CORS entirely and makes the deployment a single
container with a single volume, which is appropriate for one instructor and a handful of students.

## Backend layering

| Layer | Responsibility |
|-------|----------------|
| `handlers/` | HTTP concerns only: extract and validate input, call services/repositories, map to responses. `router.rs` holds the route table and middleware stack. |
| `auth/` | Argon2id hashing, server-side sessions, `AuthUser`/`AdminUser` extractors and `access.rs`, the single place that decides whether a user may see a course, week, lesson or assignment. |
| `services/` | Business rules that span records: submission lifecycle, progress calculation, curriculum duplication, notifications, file storage, seed data, background jobs, audit log. |
| `repositories/` | Connection setup (WAL, foreign keys, busy timeout) and composed list queries (assignments, submissions, resources) that apply visibility filters in SQL. |
| `models/` | Row types (`sqlx::FromRow`) and request/response DTOs (`utoipa::ToSchema`). |
| `middleware/` | Session resolution + CSRF + Origin checks, rate limiter, security headers. |

## Key decisions

- **SQLite + WAL** with `foreign_keys=ON`, `busy_timeout=5s`, `synchronous=NORMAL`, an 8-connection
  pool. Multi-record updates (submitting, grading, duplicating, publishing with lessons) run in
  transactions.
- **Identifiers** are UUIDv7 strings (time-ordered, unguessable enough for URLs; authorization never
  relies on secrecy of IDs).
- **Timestamps** are stored as UTC RFC 3339 strings with millisecond precision
  (`2026-01-31T09:00:00.000Z`), which sort lexicographically so comparisons are done in SQL as plain
  string comparisons. Calendar dates (course start, week opening) are `YYYY-MM-DD` and are evaluated in
  the platform timezone (`DEFAULT_TIMEZONE`, default UTC, configurable in Settings).
- **SQLx queries** use runtime-checked `query`/`query_as` with bound parameters rather than the
  compile-time `query!` macros. The macros require a live database or an offline query cache at
  build time; with ~200 queries this would add friction to Docker builds without improving safety
  (every query is parameterized and covered by integration tests). This is the "where practical"
  trade-off from the requirements.
- **Sessions** are opaque random tokens in an `HttpOnly` cookie; only the SHA-256 of the token is
  stored. Expiry slides with activity. Sessions can be revoked individually, for all other devices,
  by the instructor, and are revoked automatically on password reset or deactivation.
- **CSRF** uses the synchronizer-token pattern: each session has a CSRF token returned by
  `/auth/login` and `/auth/me`; every mutating request must echo it in `X-CSRF-Token`. Additionally
  any mutating request carrying an `Origin` header that is not the configured public origin is rejected.
- **Password reset without email:** the instructor (or the `opensyllabus reset-link` CLI) generates a
  single-use, 24-hour link that they share privately; new students receive a 72-hour setup link.
  Email delivery can be added later by sending the same link.
- **Uploads** are streamed to a temporary file, size-limited, hashed, content-checked (magic bytes
  for PDF/images/ZIP containers, UTF-8 and no NUL bytes for text formats) against an extension
  allow-list, then moved to `storage/files/<prefix>/<uuid>` without an extension. Only metadata is in
  SQLite. SVG and HTML are not accepted.
- **Downloads** go through `/api/v1/files/:id`, which checks what references the file (own
  submission, visible resource, visible course cover). Text is always served as `text/plain`, every
  file response carries a restrictive CSP (`sandbox` for non-PDF), `nosniff` and an
  `attachment` disposition unless inline preview is safe.
- **Notifications** are rows with an optional `dedupe_key`, which makes the background reminder job
  idempotent. A Tokio task runs every five minutes to announce assignments whose publication date
  has arrived, send 24-hour deadline reminders, purge expired sessions and clean abandoned uploads.
- **Grades are separate from completion** and hidden from students until the review is returned.
  See [PROGRESS.md](PROGRESS.md).
- **Rate limiting** is an in-memory fixed-window limiter (login: 30/15 min per IP and 10/15 min per
  account; password reset: 10/15 min per IP; password change: 10/15 min per user). Adequate for a
  single instance; no Redis.
- **Example data:** with `SEED_INITIAL_COURSE=true` the server creates, once, the example course
  *Light-Based Tactile Robotic Hand* as a draft with 8 modules, 48 titled draft weeks and three
  Week 1 lesson outlines. It is off by default; nothing is ever seeded into an existing course.

## Frontend

- `api/`: fetch wrapper (CSRF header, typed errors, 401 handling) and typed endpoint functions.
- `types/api.ts`: TypeScript mirrors of the backend DTOs.
- `components/ui/`: shadcn-style primitives built on Radix UI. `components/`: app-level building
  blocks (Markdown renderer/editor, file upload and preview, status badges, charts).
- `features/`: larger feature components (submission workspace, lab report form, review panel,
  resource dialog).
- `pages/`: route components; instructor pages live in `pages/admin/` and are lazy-loaded.
- Markdown is rendered with `react-markdown` (raw HTML disabled) + `rehype-sanitize`, then
  highlighted with `rehype-highlight`.
- Server state is managed with TanStack Query; forms with React Hook Form + Zod mirroring the
  server-side validation (the server remains authoritative and its field errors are mapped back).
- Theme: CSS variables in OKLCH, dark by default, applied before first paint by `public/theme-init.js`
  (an external script, because the CSP forbids inline scripts). Chart colors were validated for
  colour-vision deficiency and contrast in both themes.

## API conventions

- Base path `/api/v1`, JSON bodies, `PATCH` for partial updates (empty string clears optional fields).
- Errors: `{"error": {"code": "validation_error", "message": "...", "fields": {"title": "..."}}}` with
  401/403/404/409/413/422/429 status codes. Resources the user may not access are reported as 404.
- The full, generated contract is at `/api/openapi.json` (Swagger UI at `/api/docs`).
