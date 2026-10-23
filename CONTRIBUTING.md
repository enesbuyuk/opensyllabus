# Contributing to OpenSyllabus

Thanks for your interest in improving OpenSyllabus! Bug reports, documentation fixes and pull requests
are all welcome.

## Reporting bugs and requesting features

- Search the existing issues first.
- For bugs, include what you did, what you expected, what happened instead, and the version or
  commit you are running. Logs from `docker compose logs opensyllabus` help a lot.
- **Security issues** must not be reported publicly; follow
  [docs/SECURITY.md](docs/SECURITY.md#reporting-a-vulnerability).

## Development setup

You need Rust (stable, 1.85+), Node.js 20+ and `make`.

```bash
make start-dev
```

This creates `backend/.env`, installs dependencies, asks you to create an instructor account and
starts the API (`:8080`) and the Vite dev server (`:5173`). See the
[README](README.md#local-development) for details and the other `make` targets.

## Before opening a pull request

Run the same checks as CI:

```bash
make lint    # cargo fmt --check, clippy -D warnings, eslint, tsc
make test    # cargo test, vitest
```

Please also:

- keep pull requests focused on one change and describe the motivation;
- add or update tests for behaviour changes, especially anything touching authorization,
  submissions, grading or progress;
- update the documentation in `docs/` when you change rules or configuration;
- never commit `.env` files, databases or uploaded files.

## Conventions

### Backend (`backend/`)

- Handlers stay thin: parse and validate input, check access, call a service or repository.
- Every query uses bound parameters; never build SQL from user input.
- Resource access goes through `auth/access.rs`. Return 404 rather than 403 for records a user
  may not see, so their existence is not revealed.
- Timestamps are stored as UTC RFC 3339 strings; calendar dates (`YYYY-MM-DD`) are evaluated in
  the platform timezone.
- Schema changes go in a **new** migration in `backend/migrations/`; never edit an existing one.
- Format with `cargo fmt` (see `rustfmt.toml`).

### Frontend (`frontend/`)

- Server state lives in TanStack Query; forms use React Hook Form with Zod schemas.
- UI primitives are in `src/components/ui`; reuse them before adding new ones.
- All API calls go through `src/api`, and their types live in `src/types/api.ts`.
- Show dates through the helpers in `src/lib/utils.ts` so they respect the user's timezone.

### Commits

Write commit messages in the imperative mood ("Add rubric export", not "Added…") with a short
summary line and, when useful, a body explaining why.

## License

By contributing you agree that your contributions are licensed under the [MIT License](LICENSE).
