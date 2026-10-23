# Deployment and operations

Target: `https://lms.example.com` on a single Linux host with Docker.

## 1. Configure

```bash
git clone <repo> /opt/opensyllabus && cd /opt/opensyllabus
cp .env.example .env
```

Important variables (all documented in `.env.example`):

| Variable | Production value | Purpose |
|----------|------------------|---------|
| `PUBLIC_URL` | `https://lms.example.com` | Origin for CSRF/Origin checks and generated links |
| `COOKIE_SECURE` | `true` | `Secure` cookies + HSTS |
| `TRUST_PROXY` | `true` | Use `X-Real-IP`/`X-Forwarded-For` from the proxy for rate limiting and audit IPs |
| `SESSION_TTL_HOURS` | `168` | Sliding session lifetime |
| `DEFAULT_TIMEZONE` | e.g. `Europe/Istanbul` (default `UTC`) | Platform timezone (can be changed in Settings) |
| `MAX_UPLOAD_MB` | `50` | Upper bound for any upload |
| `SEED_INITIAL_COURSE` | `false` | Create the example 48-week course skeleton on first start |
| `RUST_LOG`, `LOG_FORMAT` | `info`, `json` | Logging |
| `HOST_PORT` | `8080` | Localhost port published by Compose |

Paths (`DATABASE_PATH=/data/db/opensyllabus.db`, `STORAGE_DIR=/data/storage`, `STATIC_DIR=/app/public`)
are fixed in `docker-compose.yml`; everything persistent lives in the `opensyllabus-data` volume.

## 2. Build and create the instructor account

```bash
docker compose build
docker compose run --rm opensyllabus create-admin --email you@example.com --name "Your Name"
docker compose up -d
docker compose ps        # should report "healthy"
```

`create-admin` prompts for the password (or reads `OPENSYLLABUS_ADMIN_PASSWORD`). Alternatively set
`BOOTSTRAP_ADMIN_EMAIL`/`BOOTSTRAP_ADMIN_PASSWORD` for the first start only and remove them
afterwards; they are ignored once any administrator exists.

## 3. Reverse proxy

The container listens on `127.0.0.1:8080` only. Use either:

- **Caddy** (automatic HTTPS): copy `deploy/Caddyfile` to `/etc/caddy/Caddyfile`, `systemctl reload caddy`.
- **nginx**: use `deploy/nginx.conf`, obtain certificates with certbot, `nginx -t && systemctl reload nginx`.

Requirements for any proxy: forward `Host`, set `X-Real-IP` to the client address, allow request
bodies of at least `MAX_UPLOAD_MB` + 10 MB, and a read timeout of ≥ 300 s for large uploads.
Frontend and API are served from the same origin; `/api/*` is the API, everything else is the SPA.

## 4. Health checks

- `GET /api/health`: process liveness.
- `GET /api/health/ready`: database and storage availability (503 when degraded).
- The image's `HEALTHCHECK` runs `opensyllabus healthcheck` (no curl needed).

## 5. Graceful shutdown and restarts

`docker compose stop` sends SIGTERM; the server stops accepting connections, finishes in-flight
requests, stops background jobs and closes the database pool (grace period 30 s). The service uses
`restart: unless-stopped`. Migrations run automatically on start and are idempotent.

## 6. Backups

Backups include the SQLite database **and** the uploaded files:

```bash
scripts/backup.sh /var/backups/opensyllabus     # online, safe while running
```

The database snapshot uses `VACUUM INTO`, which produces a consistent copy without stopping the
server. Schedule it with cron, e.g.:

```cron
15 3 * * * cd /opt/opensyllabus && scripts/backup.sh /var/backups/opensyllabus >> /var/log/opensyllabus-backup.log 2>&1
```

Keep copies off the host. To prune: `find /var/backups/opensyllabus -mtime +30 -delete`.

### Restore

```bash
scripts/restore.sh /var/backups/opensyllabus/opensyllabus-<stamp>.db /var/backups/opensyllabus/storage-<stamp>.tar.gz
```

The script stops the app, replaces the database (removing stale WAL files) and the upload
directory inside the volume, and starts the app again.

### Manual restore (without the script)

```bash
docker compose stop opensyllabus
docker run --rm -v opensyllabus_opensyllabus-data:/data -v "$PWD":/restore alpine sh -c \
  'rm -f /data/db/opensyllabus.db* && cp /restore/opensyllabus.db /data/db/opensyllabus.db && \
   rm -rf /data/storage && tar -C /data -xzf /restore/storage.tar.gz && chown -R 10001:10001 /data'
docker compose start opensyllabus
```

## 7. Upgrades

```bash
git pull
scripts/backup.sh /var/backups/opensyllabus
docker compose build && docker compose up -d
```

## 8. Persistence check

Data survives restarts and container recreation because it lives in the named volume:

```bash
docker compose down && docker compose up -d   # data intact
docker compose down -v                         # ⚠ deletes the volume and all data
```

## 9. Troubleshooting

| Symptom | Check |
|---------|-------|
| "Cross-origin request rejected" | `PUBLIC_URL` must exactly match the browser origin (scheme + host). |
| Cannot sign in over plain HTTP | `COOKIE_SECURE=true` requires HTTPS; set `false` only for local testing. |
| Every user shares one rate-limit bucket | Enable `TRUST_PROXY` and make sure the proxy sets `X-Real-IP`. |
| Upload fails with 413 | Increase the proxy body limit and/or `MAX_UPLOAD_MB`. |
| Forgotten instructor password | `docker compose exec opensyllabus opensyllabus reset-link --email you@…` |
