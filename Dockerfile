# =============================================================================
# Multi-Stage Production Dockerfile for BuildScale AI Monorepo
# =============================================================================
# This Dockerfile builds:
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
# Stage 2: Admin Frontend Dependencies
# -----------------------------------------------------------------------------
FROM pnpm-base AS admin-deps
WORKDIR /app/admin

# Copy admin package files
COPY frontend/admin/package.json frontend/admin/pnpm-lock.yaml ./

# Install admin dependencies for better caching
RUN pnpm install --frozen-lockfile

# -----------------------------------------------------------------------------
# Stage 3: Build Admin Frontend
# -----------------------------------------------------------------------------
FROM pnpm-base AS admin-builder
WORKDIR /app/admin

# Copy admin dependencies from previous stage
COPY --from=admin-deps /app/admin/node_modules ./node_modules

# Copy admin source code
COPY frontend/admin/package.json ./
COPY frontend/admin/src ./src
COPY frontend/admin/public ./public
COPY frontend/admin/index.html ./
COPY frontend/admin/vite.config.ts ./
COPY frontend/admin/tsconfig.json ./

# Build admin application
RUN pnpm build

# -----------------------------------------------------------------------------
# Stage 4: Web Frontend Dependencies
# -----------------------------------------------------------------------------
FROM pnpm-base AS web-deps
WORKDIR /app/web

# Copy web package files
COPY frontend/web/package.json frontend/web/pnpm-lock.yaml ./

# Install web dependencies for better caching
RUN pnpm install --frozen-lockfile

# -----------------------------------------------------------------------------
# Stage 5: Build Web Frontend
# -----------------------------------------------------------------------------
FROM pnpm-base AS web-builder
WORKDIR /app/web

# Copy web dependencies from previous stage
COPY --from=web-deps /app/web/node_modules ./node_modules

# Copy web source code
COPY frontend/web/package.json ./
COPY frontend/web/src ./src
COPY frontend/web/public ./public
COPY frontend/web/index.html ./
COPY frontend/web/vite.config.ts ./
COPY frontend/web/tsconfig.json ./

# Build web application
RUN pnpm build

# -----------------------------------------------------------------------------
# Stage 6: Rust Build Planner for cargo-chef
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
# Stage 7: Cache Rust Dependencies
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
# Stage 8: Build Rust Backend
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

# Build backend binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    echo "Building binary for native musl target" && \
    SQLX_OFFLINE=true cargo build --release && \
    cp target/release/buildscale /app/buildscale

# -----------------------------------------------------------------------------
# Stage 9: Final Production Image
# -----------------------------------------------------------------------------
FROM alpine:3.22 AS final
WORKDIR /app

# Accept build arguments
ARG BUILD_DATE
ARG GIT_COMMIT

# Install runtime dependencies
RUN apk add --no-cache ca-certificates curl

# Create non-root user for security
RUN addgroup -S appgroup && adduser -S -G appgroup appuser

# Set default environment variables
ENV RUST_LOG="info"
ENV BUILD_DATE="${BUILD_DATE}"
ENV GIT_COMMIT="${GIT_COMMIT}"
ENV PORT=3000

# Copy compiled binary from rust-builder
COPY --from=rust-builder /app/buildscale ./buildscale

# Copy backend migrations
COPY backend/migrations ./migrations

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
