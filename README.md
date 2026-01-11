# BuildScale AI

Monorepo with Rust backend and React frontends.

## Quick Start (Docker Compose)

```bash
# 1. Generate SQLx cache (first time only)
cd backend
cargo sqlx prepare
cd ..

# 2. Start all services
docker compose up -d
```

- **Server**: http://localhost:3000
- **Health check**: http://localhost:3000/api/v1/health

## Project Structure

```
buildscale-ai/
├── backend/          # Rust API (Axum + SQLx + PostgreSQL)
├── frontend/
│   ├── admin/        # Admin React app (port 5173)
│   └── web/          # Public React app (port 5174)
├── docker-compose.yml
└── Dockerfile        # Multi-stage build
```

## Development

### Local Development

```bash
# Backend
cd backend && cargo run

# Frontend (all services with SDK watch mode)
cd frontend && pnpm dev

# Admin frontend (standalone)
cd frontend/admin && pnpm dev

# Web frontend (standalone)
cd frontend/web && pnpm dev
```

**Frontend Development:**

Running `pnpm dev` from the `frontend/` directory starts all services in parallel:
- **SDK watch mode** - Automatically rebuilds on changes
- **Admin**: `http://localhost:5173/admin` (with TanStack devtools)
- **Web**: `http://localhost:5174` (devtools disabled - focusing on admin)

This enables seamless development where SDK changes automatically propagate to both admin and web applications without manual rebuilding.

### Docker Build

```bash
# Build image
docker compose build

# Start services
docker compose up -d

# View logs
docker compose logs -f buildscale

# Stop services
docker compose down
```

## Configuration

Environment variables in `docker-compose.yml`:

- `BUILDSCALE_DATABASE_HOST` (default: `postgres`)
- `BUILDSCALE_JWT_SECRET` (min 32 chars)
- `BUILDSCALE_SESSIONS_EXPIRATION_HOURS` (default: `720`)

See `backend/.env.example` for all options.

## Database

```bash
# Run migrations
cd backend
sqlx migrate run
```

## Prerequisites

- Rust + Cargo
- Node.js 22 + pnpm
- Docker + Docker Compose
- PostgreSQL 18
