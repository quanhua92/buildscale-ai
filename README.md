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

# Admin frontend
cd frontend/admin && pnpm dev

# Web frontend
cd frontend/web && pnpm dev
```

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
