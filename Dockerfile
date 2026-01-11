# =============================================================================
# Multi-Stage Production Dockerfile for BuildScale AI Monorepo
# =============================================================================
# This Dockerfile builds:
# - SDK shared package (TypeScript + tsup)
# - Admin React frontend (Vite + pnpm)
# - Web React frontend (Vite + pnpm)
# - Rust backend (cargo-chef optimized)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Base PNPM Setup Stage
# -----------------------------------------------------------------------------
FROM node:22-alpine AS pnpm-base
WORKDIR /app

# Install pnpm globally
RUN npm install -g pnpm

# -----------------------------------------------------------------------------
# Stage 2: Frontend Dependencies Base (includes built SDK)
# -----------------------------------------------------------------------------
FROM pnpm-base AS frontend-base
WORKDIR /app

# Copy workspace configuration
COPY frontend/pnpm-workspace.yaml frontend/pnpm-lock.yaml ./

# Copy SDK package files
COPY frontend/sdk/package.json ./sdk/
COPY frontend/sdk/tsconfig.json ./sdk/
COPY frontend/sdk/tsup.config.ts ./sdk/

# Copy admin package files
COPY frontend/admin/package.json ./admin/

# Copy web package files
COPY frontend/web/package.json ./web/

# Set API base URL for production builds (relative path for same-origin)
ENV VITE_API_BASE_URL=/api/v1

# Install all dependencies (workspace protocol links SDK)
RUN pnpm install --frozen-lockfile

# Copy SDK source (after dependencies for better layer caching)
COPY frontend/sdk/src ./sdk/src/

# Build SDK
RUN pnpm --filter @buildscale/sdk build

# -----------------------------------------------------------------------------
# Stage 3: Build Admin Frontend
# -----------------------------------------------------------------------------
FROM frontend-base AS admin-builder
WORKDIR /app

# Copy admin source code
COPY frontend/admin/src ./admin/src
COPY frontend/admin/public ./admin/public
COPY frontend/admin/index.html ./admin/
COPY frontend/admin/vite.config.ts ./admin/
COPY frontend/admin/tsconfig.json ./admin/

# Build admin application
RUN pnpm --filter admin build

# -----------------------------------------------------------------------------
# Stage 4: Build Web Frontend
# -----------------------------------------------------------------------------
FROM frontend-base AS web-builder
WORKDIR /app

# Copy web source code
COPY frontend/web/src ./web/src
COPY frontend/web/public ./web/public
COPY frontend/web/index.html ./web/
COPY frontend/web/vite.config.ts ./web/
COPY frontend/web/tsconfig.json ./web/

# Build web application
RUN pnpm --filter web build

# -----------------------------------------------------------------------------
# Stage 5: Rust Build Planner for cargo-chef
# -----------------------------------------------------------------------------
FROM rust:1.91-alpine AS chef
USER root

# Install build dependencies for Alpine
RUN apk add --no-cache \
    musl-dev \
    ca-certificates \
    gcc \
    g++ \
    make

# Install cargo-chef with cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-chef --locked

WORKDIR /app

FROM chef AS rust-planner
WORKDIR /app

# Copy only manifest files for cargo-chef analysis
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 6: Cache Rust Dependencies
# -----------------------------------------------------------------------------
FROM chef AS rust-cacher
WORKDIR /app
ARG TARGETPLATFORM

# Add Alpine build dependencies
RUN apk add --no-cache \
    musl-dev \
    ca-certificates \
    gcc \
    g++ \
    make

COPY --from=rust-planner /app/recipe.json recipe.json

# Build dependencies with cache mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    echo "Building for native musl target" && \
    cargo chef cook --release --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 7: Build Rust Backend
# -----------------------------------------------------------------------------
FROM chef AS rust-builder
WORKDIR /app
ARG TARGETPLATFORM

# Add Alpine build dependencies
RUN apk add --no-cache \
    musl-dev \
    ca-certificates \
    gcc \
    g++ \
    make

# Copy cached dependencies
COPY --from=rust-cacher /app/target target
COPY backend/Cargo.lock ./Cargo.lock
COPY backend/Cargo.toml ./Cargo.toml
COPY backend/src ./src
# Copy .sqlx cache for offline builds
COPY backend/.sqlx ./.sqlx/
# Copy migrations directory
COPY backend/migrations ./migrations

# Ensure migrations are readable during build
RUN chmod -R 644 ./migrations/*.sql

# Build backend binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    echo "Building binary for native musl target" && \
    SQLX_OFFLINE=true cargo build --release && \
    cp target/release/buildscale /app/buildscale

# -----------------------------------------------------------------------------
# Stage 8: Final Production Image
# -----------------------------------------------------------------------------
FROM alpine:3.22 AS final
WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache ca-certificates curl

# Create non-root user for security
RUN addgroup -S appgroup && adduser -S -G appgroup appuser

# Accept build arguments
ARG BUILD_DATE
ARG GIT_COMMIT

# Set default environment variables
ENV RUST_LOG="info"
ENV BUILD_DATE="${BUILD_DATE}"
ENV GIT_COMMIT="${GIT_COMMIT}"
ENV PORT=3000

# Set default environment variables for static file serving
ENV BUILDSCALE__SERVER__ADMIN_BUILD_PATH="/app/admin"
ENV BUILDSCALE__SERVER__WEB_BUILD_PATH="/app/web"

# Copy compiled binary from rust-builder
COPY --from=rust-builder /app/buildscale ./buildscale

# Copy backend migrations
COPY backend/migrations ./migrations

# Ensure migrations are readable (fixes permission issues when copying from host)
RUN chmod -R 644 ./migrations/*.sql

# Copy admin frontend build artifacts
COPY --from=admin-builder /app/admin/dist ./admin

# Copy web frontend build artifacts
COPY --from=web-builder /app/web/dist ./web

# Create data directories
RUN mkdir -p /app/data && chown -R appuser:appgroup /app/data

# Change ownership to non-root user
RUN chown -R appuser:appgroup /app

# Use non-root user
USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:${PORT:-3000}/api/v1/health || exit 1

# Default command
CMD ["./buildscale"]
