# Security

| Concern | Implementation |
|---------|----------------|
| Password storage | Argon2id (19 MiB, t=2, p=1), random salt; minimum 10 characters with a number or symbol. Unknown accounts take the same time to reject. |
| Sessions | 256-bit random token in an `HttpOnly`, `SameSite=Lax`, `Secure` (production) cookie; only its SHA-256 is stored. Sliding expiry (`SESSION_TTL_HOURS`), revocation per session / all others / by instructor; revoked on password change (other devices), password reset and account deactivation. |
| CSRF | Per-session synchronizer token required in `X-CSRF-Token` for every mutating request; mutating requests with a foreign `Origin` are rejected. |
| Authorization | Role checks via `AdminUser` extractor; resource checks in `auth/access.rs` (enrollment, publication state, ownership). Inaccessible resources return 404 to avoid leaking existence. Students can never read other students' submissions, grades, feedback or files, or instructor-only resources. |
| Rate limiting | Login per IP and per account, password reset per IP, password change per user. |
| Input validation | All inputs validated server-side (lengths, enums, dates, URLs restricted to http/https, video URLs to https, rubric totals, ownership of referenced records). SQL is always parameterized. |
| Uploads | Extension allow-list, server-chosen content type, magic-byte or UTF-8 validation, size limits (global and per assignment), filename sanitization, random storage keys outside the web root, no SVG/HTML. |
| Downloads | Authorization per file reference; `nosniff`; text served as `text/plain`; CSP `sandbox` for non-PDF files; `Content-Disposition: attachment` unless inline preview is safe. |
| XSS | React escaping; Markdown rendered without raw HTML and sanitized; external links get `rel="noopener noreferrer nofollow"`. Strict CSP: `script-src 'self'`, no inline scripts, `frame-ancestors 'none'`, iframes only from YouTube (nocookie)/Vimeo. |
| HTTP headers | CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`, COOP, CORP, HSTS (when secure), `Cache-Control: no-store` on API responses. |
| CORS | None: the API is same-origin only. `ALLOWED_ORIGINS` exists solely for development. |
| Secrets | No default credentials. The first administrator is created with the CLI (password prompted) or a one-time environment bootstrap that is ignored once an administrator exists. Password hashes, session tokens, CSRF tokens of other sessions and storage paths are never serialized. |
| Audit log | Logins (successful and failed), password changes/resets, user creation/updates/reset links/session revocation, course/week/lesson publishing and deletion, enrollment changes, assignment status changes, reviews, resources, announcements and settings. Visible to the instructor under Settings → Platform. |
| Container | Non-root user, read-only root filesystem, `no-new-privileges`, all capabilities dropped, port bound to localhost. |

## Reporting a vulnerability

Please do **not** open a public issue for security problems. Use GitHub's
[private vulnerability reporting](../../../security/advisories/new) for this repository instead.
Include the affected version or commit, reproduction steps and the impact you observed. You can
expect an acknowledgement within a few days.
