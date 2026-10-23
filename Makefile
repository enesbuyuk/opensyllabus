# OpenSyllabus developer shortcuts.
#
#   make start-dev    API (:8080) + Vite (:5173) together; Ctrl+C stops both
#   make help         list all targets

SHELL := /bin/bash
.DEFAULT_GOAL := help

# Homebrew Node is preferred so a lazily loaded nvm shell function is not required.
export PATH := /opt/homebrew/bin:/usr/local/bin:$(HOME)/.cargo/bin:$(PATH)
NPM ?= npm
CARGO ?= cargo

BACKEND := backend
FRONTEND := frontend
DEV_ENV := $(BACKEND)/.env
# Database path as configured in backend/.env (relative paths are relative to backend/).
DB_PATH := $(or $(shell sed -n 's/^DATABASE_PATH=//p' $(DEV_ENV) 2>/dev/null),./data/opensyllabus.db)
DEV_DB ?= $(if $(filter /%,$(DB_PATH)),$(DB_PATH),$(BACKEND)/$(patsubst ./%,%,$(DB_PATH)))

.PHONY: help start-dev dev-api dev-web dev-setup dev-admin dev-reset-link dev-reset \
        test lint fmt build docker-build docker-up docker-down docker-logs backup

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

# ---------------------------------------------------------------------------
# Local development
# ---------------------------------------------------------------------------

start-dev: dev-setup ## Start API and frontend (creates .env, installs deps, asks for an admin on first run)
	@echo ""
	@echo "  App:      http://localhost:5173"
	@echo "  API docs: http://localhost:8080/api/docs"
	@echo "  Ctrl+C stops both."
	@echo ""
	@trap 'kill 0' INT TERM EXIT; \
	 (cd $(BACKEND) && $(CARGO) run) & \
	 (cd $(FRONTEND) && $(NPM) run dev) & \
	 wait

dev-api: $(DEV_ENV) ## Start only the API
	cd $(BACKEND) && $(CARGO) run

dev-web: $(FRONTEND)/node_modules ## Start only the Vite dev server
	cd $(FRONTEND) && $(NPM) run dev

dev-setup: $(DEV_ENV) $(FRONTEND)/node_modules
	@admins=0; \
	 if [ -f $(DEV_DB) ] && command -v sqlite3 >/dev/null; then \
	   admins=$$(sqlite3 $(DEV_DB) "SELECT count(*) FROM users WHERE role = 'admin'" 2>/dev/null || echo 0); \
	 fi; \
	 if [ "$$admins" = "0" ]; then \
	   echo "No instructor account yet, let's create one."; \
	   $(MAKE) --no-print-directory dev-admin; \
	 fi

$(DEV_ENV):
	@echo "Creating $(DEV_ENV) for local development"
	@printf '%s\n' \
	  'PUBLIC_URL=http://localhost:5173' \
	  'COOKIE_SECURE=false' \
	  'TRUST_PROXY=false' \
	  'BIND_ADDR=127.0.0.1:8080' \
	  'DATABASE_PATH=./data/opensyllabus.db' \
	  'STORAGE_DIR=./storage' \
	  'SEED_INITIAL_COURSE=true' \
	  'RUST_LOG=info,tower_http=info,sqlx=warn' \
	  'LOG_FORMAT=text' > $@

$(FRONTEND)/node_modules: $(FRONTEND)/package-lock.json
	cd $(FRONTEND) && $(NPM) ci
	@touch $@

dev-admin: $(DEV_ENV) ## Create an instructor account (EMAIL=… NAME=… optional)
	@email="$(EMAIL)"; name="$(NAME)"; \
	 [ -n "$$email" ] || read -rp "Instructor email: " email; \
	 [ -n "$$name" ] || read -rp "Display name [Instructor]: " name; \
	 cd $(BACKEND) && $(CARGO) run --quiet -- create-admin --email "$$email" --name "$${name:-Instructor}"

dev-reset-link: $(DEV_ENV) ## Print a password reset link (EMAIL=…)
	@[ -n "$(EMAIL)" ] || { echo "Usage: make dev-reset-link EMAIL=you@example.com"; exit 1; }
	cd $(BACKEND) && $(CARGO) run --quiet -- reset-link --email "$(EMAIL)"

dev-reset: ## Delete the local database and uploads (asks first)
	@read -rp "Delete $(BACKEND)/data and $(BACKEND)/storage? [y/N] " ok; \
	 [ "$$ok" = "y" ] || [ "$$ok" = "Y" ] || { echo "Aborted."; exit 1; }; \
	 rm -rf $(BACKEND)/data $(BACKEND)/storage; echo "Local data removed."

# ---------------------------------------------------------------------------
# Quality
# ---------------------------------------------------------------------------

test: $(FRONTEND)/node_modules ## Run backend and frontend tests
	cd $(BACKEND) && $(CARGO) test
	cd $(FRONTEND) && $(NPM) test

lint: $(FRONTEND)/node_modules ## fmt check, clippy, eslint, typecheck
	cd $(BACKEND) && $(CARGO) fmt --check && $(CARGO) clippy --all-targets -- -D warnings
	cd $(FRONTEND) && $(NPM) run lint && $(NPM) run typecheck

fmt: ## Format backend code
	cd $(BACKEND) && $(CARGO) fmt

build: $(FRONTEND)/node_modules ## Production build of both parts
	cd $(FRONTEND) && $(NPM) run build
	cd $(BACKEND) && $(CARGO) build --release

# ---------------------------------------------------------------------------
# Docker
# ---------------------------------------------------------------------------

docker-build: ## Build the Docker image
	docker compose build

docker-up: ## Start the container in the background
	docker compose up -d

docker-down: ## Stop the container (data is kept)
	docker compose down

docker-logs: ## Follow container logs
	docker compose logs -f opensyllabus

backup: ## Back up the Docker volume (DIR=./backups)
	scripts/backup.sh $(or $(DIR),./backups)
