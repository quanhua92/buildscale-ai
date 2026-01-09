# Static File Serving

This document explains how static file serving works in the BuildScale AI application, including configuration, architecture, and troubleshooting.

## Overview

The BuildScale AI backend serves three types of content from a single server:

- **Admin Frontend**: React SPA served at `/admin`
- **Web Frontend**: React SPA served at root `/`
- **API Backend**: REST API served at `/api/v1`

All three are served by the same Rust backend using Axum, with proper route prioritization and SPA fallback support.

## Architecture

### Route Priority

Routes are checked in registration order:

1. **`/api/v1/*`** - API routes (highest priority)
   - Exact match for API endpoints
   - Returns JSON responses
   - Example: `/api/v1/health`, `/api/v1/auth/login`

2. **`/admin/*`** - Admin frontend (medium priority)
   - Prefix match for admin routes
   - Serves static files or falls back to `index.html`
   - Example: `/admin`, `/admin/dashboard`, `/admin/assets/main.js`

3. **`/*`** - Web frontend (fallback, lowest priority)
   - Catches all unmatched routes
   - Serves static files or falls back to `index.html`
   - Example: `/`, `/workspace/123`, `/assets/main.js`

### Request Flow Examples

```
GET /api/v1/health
→ Matches API route
→ Returns JSON health check response

GET /admin
→ Matches /admin prefix
→ Serves /app/admin/index.html
→ React Router renders admin home

GET /admin/dashboard
→ Matches /admin prefix
→ File doesn't exist, falls back to /app/admin/index.html
→ React Router sees /admin/dashboard in URL
→ Renders dashboard component

GET /admin/assets/main.js
→ Matches /admin prefix
→ Serves actual file /app/admin/assets/main.js

GET /
→ No API or admin match
→ Falls back to web service
→ Serves /app/web/index.html
→ React Router renders web home

GET /workspace/123
→ No API or admin match
→ Falls back to web service
→ File doesn't exist, falls back to /app/web/index.html
→ React Router sees /workspace/123 in URL
→ Renders workspace component

GET /assets/main.js
→ No API or admin match
→ Falls back to web service
→ Serves actual file /app/web/assets/main.js
```

### SPA Fallback Implementation

Both frontends use the SPA fallback pattern:

```rust
let admin_static_service = ServeDir::new(admin_build_path)
    .not_found_service(ServeFile::new(admin_index_path));

let web_static_service = ServeDir::new(web_build_path)
    .not_found_service(ServeFile::new(web_index_path));
```

**How it works:**
1. `ServeDir` attempts to serve the requested file
2. If the file doesn't exist (404), it falls back to `ServeFile(index.html)`
3. The React app loads, and the client-side router (TanStack Router) takes over
4. The router renders the appropriate component based on the URL path

**Technical Note:**
- Uses tower-http's `fs` feature (provides `ServeDir` and `ServeFile`)
- `ServeDir` serves static files with correct MIME types
- `not_found_service()` chains the fallback for unmatched routes
- `ServeFile` serves the specified file (index.html) for all non-file requests

This enables:
- Deep linking to specific routes (e.g., `/admin/dashboard`)
- Browser refresh on any route
- Client-side navigation without page reloads
- Proper 404 handling at the client level

## Configuration

### Dependencies

Static file serving requires the `fs` and `compression-gzip` features in `Cargo.toml`:

```toml
# backend/Cargo.toml
tower-http = { version = "0.6", features = ["trace", "cors", "set-header", "request-id", "fs", "compression-gzip"] }
```

These features provide:
- `ServeDir` - Directory serving with SPA fallback support
- `ServeFile` - Individual file serving
- `CompressionLayer` - Gzip compression for responses
- Automatic MIME type detection
- Path traversal protection

### Environment Variables

Static file serving is configured via environment variables:

```bash
# Admin frontend build directory
BUILDSCALE__SERVER__ADMIN_BUILD_PATH=/app/admin

# Web frontend build directory
BUILDSCALE__SERVER__WEB_BUILD_PATH=/app/web
```

### Default Values

**Local Development:**
- `BUILDSCALE__SERVER__ADMIN_BUILD_PATH="./admin"`
- `BUILDSCALE__SERVER__WEB_BUILD_PATH="./web"`

**Docker:**
- `BUILDSCALE__SERVER__ADMIN_BUILD_PATH="/app/admin"`
- `BUILDSCALE__SERVER__WEB_BUILD_PATH="/app/web"`

### Security Features

**Empty String Disables Serving:**
```bash
# Disable admin frontend (security feature)
BUILDSCALE__SERVER__ADMIN_BUILD_PATH=""

# Disable web frontend (security feature)
BUILDSCALE__SERVER__WEB_BUILD_PATH=""
```

Empty strings prevent serving from accidental paths (e.g., root `/`).

**API-Only Mode:**
If both paths are empty, the server operates in API-only mode:
```bash
BUILDSCALE__SERVER__ADMIN_BUILD_PATH=""
BUILDSCALE__SERVER__WEB_BUILD_PATH=""
```

In this mode:
- Only `/api/v1/*` routes are available
- All other requests return 404
- Useful for API-only deployments or microservices architectures

### Directory Structure

**Local Development (Build in Place):**
```
frontend/
├── admin/
│   ├── src/
│   ├── dist/            # Build output (served by backend)
│   │   ├── index.html
│   │   └── assets/
│   │       ├── main.js
│   │       └── main.css
│   └── vite.config.ts
└── web/
    ├── src/
    ├── dist/            # Build output (served by backend)
    │   ├── index.html
    │   └── assets/
    │       ├── main.js
    │       └── main.css
    └── vite.config.ts
```

**Local Development (Copy to Backend):**
```
backend/
├── admin/               # Copied from frontend/admin/dist
│   ├── index.html
│   └── assets/
│       ├── main.js
│       └── main.css
└── web/                 # Copied from frontend/web/dist
    ├── index.html
    └── assets/
        ├── main.js
        └── main.css
```

**Docker:**
```
/app/
├── admin/               # Copied from admin-builder stage
│   ├── index.html
│   └── assets/
├── web/                 # Copied from web-builder stage
│   ├── index.html
│   └── assets/
├── buildscale           # Rust binary
└── migrations/          # Database migrations
```

## Local Development

### Setup

There are two approaches for local development:

**Option 1: Build in Place (Recommended)**

1. **Use the build script:**
   ```bash
   # From project root
   ./frontend-build.sh
   ```

   This builds both frontends in their original directories.

2. **Configure .env:**
   ```bash
   cd backend
   cp .env.example .env
   # Edit .env to use absolute paths:
   # BUILDSCALE__SERVER__ADMIN_BUILD_PATH=/Volumes/data/workspace/buildscale-ai/frontend/admin/dist
   # BUILDSCALE__SERVER__WEB_BUILD_PATH=/Volumes/data/workspace/buildscale-ai/frontend/web/dist
   ```

3. **Run Backend:**
   ```bash
   cd backend
   cargo run
   ```

**Option 2: Copy to Backend**

1. **Build Frontends:**
   ```bash
   cd frontend/admin && pnpm build
   cd ../web && pnpm build
   ```

2. **Copy to Backend:**
   ```bash
   # From project root
   cp -r frontend/admin/dist backend/admin
   cp -r frontend/web/dist backend/web
   ```

3. **Configure Environment:**
   ```bash
   cd backend
   cp .env.example .env
   # Use relative paths (defaults in .env.example work)
   ```

4. **Run Backend:**
   ```bash
   cd backend
   cargo run
   ```

### Testing

```bash
# Test API endpoint
curl http://localhost:3000/api/v1/health

# Test admin frontend
curl http://localhost:3000/admin

# Test web frontend
curl http://localhost:3000/

# Test SPA routing (should return index.html)
curl http://localhost:3000/admin/dashboard
curl http://localhost:3000/workspace/123
```

### Browser Testing

- Open `http://localhost:3000/` - Should load web frontend
- Open `http://localhost:3000/admin` - Should load admin frontend
- Navigate to `/admin/dashboard` - Should render admin dashboard (SPA routing)
- Navigate to `/workspace/123` - Should render workspace page (SPA routing)
- Refresh on any route - Should work correctly (SPA fallback)

### Development Tips

**Build Script Workflow:**
The recommended approach is to build in place using the provided script:
```bash
# Build both frontends
./frontend-build.sh

# Make changes to frontend code

# Rebuild after changes
./frontend-build.sh
```

**Watch Mode (Optional):**
For faster development iteration, use watch mode in separate terminals:
```bash
# Terminal 1: Watch and rebuild admin
cd frontend/admin
pnpm build --watch

# Terminal 2: Watch and rebuild web
cd frontend/web
pnpm build --watch

# Backend will serve the updated builds automatically
```

**Benefits of Build-in-Place Approach:**
- No need to copy files after each build
- Faster development iteration
- Less disk space usage
- Simpler workflow

## Docker Deployment

### Configuration

Environment variables are set in two places:

**Dockerfile** (default values):
```dockerfile
ENV BUILDSCALE__SERVER__ADMIN_BUILD_PATH="/app/admin"
ENV BUILDSCALE__SERVER__WEB_BUILD_PATH="/app/web"
```

**docker-compose.yml** (explicit override):
```yaml
environment:
  - BUILDSCALE__SERVER__ADMIN_BUILD_PATH=/app/admin
  - BUILDSCALE__SERVER__WEB_BUILD_PATH=/app/web
```

### Build Process

The Dockerfile uses multi-stage builds:

1. **Admin Build Stage:**
   ```dockerfile
   FROM pnpm-base AS admin-builder
   # ... install dependencies ...
   RUN pnpm build
   # Output: /app/admin/dist/
   ```

2. **Web Build Stage:**
   ```dockerfile
   FROM pnpm-base AS web-builder
   # ... install dependencies ...
   RUN pnpm build
   # Output: /app/web/dist/
   ```

3. **Final Stage:**
   ```dockerfile
   FROM alpine:3.22 AS final
   COPY --from=admin-builder /app/admin/dist ./admin
   COPY --from=web-builder /app/web/dist ./web
   # Result: /app/admin/ and /app/web/
   ```

### Testing Docker Deployment

```bash
# Build and start containers
docker-compose up --build

# Test endpoints
curl http://localhost:3000/
curl http://localhost:3000/admin
curl http://localhost:3000/api/v1/health

# Check container logs for serving status
docker-compose logs -f buildscale
```

Expected log output:
```
INFO Admin frontend serving enabled at path: '/app/admin'
INFO Web frontend serving enabled at path: '/app/web'
INFO API server listening on http://0.0.0.0:3000
```

## Frontend Configuration

### Vite Base Path

Vite's `base` configuration ensures assets load from correct paths:

**Admin Frontend** (`frontend/admin/vite.config.ts`):
```typescript
export default defineConfig({
  base: '/admin',
  // ...
})
```

**Web Frontend** (`frontend/web/vite.config.ts`):
```typescript
export default defineConfig({
  base: '/',
  // ...
})
```

**Why this matters:**
- Vite prefixes all asset paths with `base` during build
- Admin assets: `/admin/assets/main.js`, `/admin/assets/main.css`
- Web assets: `/assets/main.js`, `/assets/main.css`
- Without correct `base`, assets would 404

### TanStack Router Integration

Both frontends use TanStack Router for client-side routing:

```typescript
// Example admin route
import { createRoute } from '@tanstack/react-router'

const DashboardRoute = createRoute({
  path: '/dashboard',  // Becomes /admin/dashboard
  component: Dashboard,
})
```

The router handles:
- Client-side navigation without page reloads
- URL parsing and parameter extraction
- Route-based code splitting
- Nested layouts and routing

## Security Considerations

### Path Traversal Protection

**Tower-http's `ServeDir`** automatically prevents path traversal attacks:
- Blocks requests like `/admin/../../../etc/passwd`
- Validates all paths are within the served directory
- No additional implementation needed

### Directory Validation

The backend checks for directory existence:
```rust
if !Path::new(admin_build_path).is_dir() {
    tracing::warn!("Admin build directory not found");
}
```

This provides:
- Early warning of misconfiguration
- Prevents runtime crashes
- Helps debug deployment issues

### Empty String Security

Empty strings disable serving:
```rust
if !admin_build_path.is_empty() {
    // Enable serving
}
```

This prevents:
- Accidental serving from root `/` filesystem
- Security misconfigurations
- Unintended file exposure

### File Type Security

`ServeDir` serves files with correct MIME types:
- HTML files: `text/html`
- CSS files: `text/css`
- JavaScript files: `application/javascript`
- Images: Proper image MIME types

Security headers are set via middleware:
- `X-Content-Type-Options: nosniff` - Prevents MIME sniffing
- `X-Frame-Options: DENY` - Prevents clickjacking

### CORS Configuration

CORS is enabled for all routes:
```rust
.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any),
)
```

This applies to:
- API endpoints
- Static files (admin and web)

## Troubleshooting

### Issue: Assets Return 404

**Symptoms:**
- Page loads but CSS/JS files don't
- Browser console shows 404 errors for assets

**Solutions:**

1. **Check Vite `base` configuration:**
   ```bash
   # Verify base is set correctly
   cat frontend/admin/vite.config.ts | grep "base:"
   cat frontend/web/vite.config.ts | grep "base:"
   ```

2. **Verify build directory structure:**
   ```bash
   # Check admin build
   ls -la /app/admin/
   # Should contain: index.html, assets/

   # Check web build
   ls -la /app/web/
   # Should contain: index.html, assets/
   ```

3. **Check browser network tab:**
   - Open browser DevTools → Network
   - Look for failed asset requests
   - Verify paths are correct (e.g., `/admin/assets/main.js` vs `/assets/main.js`)

4. **Rebuild frontends:**
   ```bash
   cd frontend/admin && pnpm build
   cd ../web && pnpm build
   ```

### Issue: API Routes Return HTML

**Symptoms:**
- `/api/v1/health` returns index.html instead of JSON
- API calls fail with parse errors

**Solutions:**

1. **Verify route registration order:**
   ```rust
   // API routes must be registered FIRST
   let mut app = Router::new()
       .nest("/api/v1", api_routes);
   ```

2. **Check for typos in route paths:**
   ```bash
   # Verify API is called correctly
   curl -H "Accept: application/json" http://localhost:3000/api/v1/health
   ```

3. **Review router construction:**
   - Ensure `/api/v1` is added before static serving
   - Check that `nest_service` is used for admin, not `nest`

### Issue: SPA Routing Doesn't Work

**Symptoms:**
- Deep links return 404 or serve wrong frontend
- Refresh on `/admin/dashboard` shows 404

**Solutions:**

1. **Verify SPA fallback is configured:**
   ```rust
   .not_found_service(ServeFile::new(admin_index_path))
   ```

2. **Check index.html exists:**
   ```bash
   ls -la /app/admin/index.html
   ls -la /app/web/index.html
   ```

3. **Test fallback directly:**
   ```bash
   # Should return index.html
   curl http://localhost:3000/admin/nonexistent-route
   curl http://localhost:3000/nonexistent-route
   ```

4. **Ensure client-side router is initialized:**
   - Check TanStack Router setup in main.tsx
   - Verify router is mounted in App component

### Issue: Directory Not Found Warnings

**Symptoms:**
- Logs show "build directory not found" warnings
- Static files return 404

**Solutions:**

1. **Check configured paths:**
   ```bash
   # Verify environment variables
   echo $BUILDSCALE__SERVER__ADMIN_BUILD_PATH
   echo $BUILDSCALE__SERVER__WEB_BUILD_PATH
   ```

2. **Verify directories exist:**
   ```bash
   # Local development
   ls -la backend/admin
   ls -la backend/web

   # Docker
   docker-compose exec buildscale ls -la /app/admin
   docker-compose exec buildscale ls -la /app/web
   ```

3. **Check Dockerfile copy commands:**
   ```dockerfile
   COPY --from=admin-builder /app/admin/dist ./admin
   COPY --from=web-builder /app/web/dist ./web
   ```

4. **For local dev, copy builds:**
   ```bash
   cp -r frontend/admin/dist backend/admin
   cp -r frontend/web/dist backend/web
   ```

### Issue: Wrong Frontend Served

**Symptoms:**
- Accessing `/admin` serves web frontend
- Accessing `/` serves admin frontend

**Solutions:**

1. **Check route priority:**
   ```rust
   // Order matters!
   app = app.nest_service("/admin", admin_static_service);  // First
   app = app.fallback_service(web_static_service);          // Second
   ```

2. **Verify paths are not swapped:**
   ```bash
   # Check admin path
   echo $BUILDSCALE__SERVER__ADMIN_BUILD_PATH
   # Should be: /app/admin or ./admin

   # Check web path
   echo $BUILDSCALE__SERVER__WEB_BUILD_PATH
   # Should be: /app/web or ./web
   ```

3. **Test with curl:**
   ```bash
   # Should return admin index.html
   curl http://localhost:3000/admin

   # Should return web index.html
   curl http://localhost:3000/
   ```

### Issue: Configuration Not Loaded

**Symptoms:**
- Changes to .env have no effect
- Default paths are used instead

**Solutions:**

1. **Verify .env file location:**
   ```bash
   # .env must be in backend directory
   ls -la backend/.env
   ```

2. **Check environment variable prefix:**
   ```bash
   # Must use BUILDSCALE__ prefix
   BUILDSCALE__SERVER__ADMIN_BUILD_PATH=./admin
   ```

3. **Restart backend after changes:**
   ```bash
   # Stop and restart
   cargo run
   ```

4. **Check logs for loaded config:**
   ```
   INFO Admin frontend serving enabled at path: './admin'
   INFO Web frontend serving enabled at path: './web'
   ```

## Performance Considerations

### Static Asset Caching

Currently, no cache headers are set for static files. This is intentional for development to ensure changes are immediately visible.

In production, consider adding cache headers for immutable assets like JS/CSS files with content hashes in their filenames.

### Response Compression

The server automatically compresses all responses using Gzip compression:

**What gets compressed:**
- HTML files (up to 70% size reduction)
- CSS files (up to 80% size reduction)
- JavaScript files (up to 70% size reduction)
- JSON API responses (up to 60% size reduction)
- Plain text files

**How it works:**
- Compression layer is applied globally to all responses
- Clients request compression via `Accept-Encoding: gzip` header
- Server automatically compresses and sets `Content-Encoding: gzip` response header
- Compression is transparent to the application code

**Benefits:**
- Reduced bandwidth usage (up to 70% for text files)
- Faster page loads
- Lower bandwidth costs
- Better user experience on slow connections

**Note:** Binary files (images, fonts, videos) are not compressed as they're already compressed.

## Monitoring and Logging

### Log Messages

The backend provides informative logs:

```
INFO Admin frontend serving enabled at path: '/app/admin'
INFO Web frontend serving enabled at path: '/app/web'
WARN Admin build directory not found at '/admin'. Admin frontend will fail to serve.
INFO Admin frontend serving disabled (admin_build_path is empty)
```

### Health Checks

Static file health can be added:

```rust
.route("/health/statics", get(|| async {
    let admin_exists = Path::new(&config.server.admin_build_path).exists();
    let web_exists = Path::new(&config.server.web_build_path).exists();
    Json(json!({ "admin": admin_exists, "web": web_exists }))
}))
```

## Related Documentation

- [Configuration Guide](./CONFIGURATION.md) - Environment variable configuration
- [Authentication Guide](./AUTHENTICATION.md) - JWT and session management
- [REST API Guide](./REST_API_GUIDE.md) - API endpoint documentation
- [Architecture](./ARCHITECTURE.md) - Overall system architecture

## Common Pitfalls and Best Practices

### ❌ Common Mistakes

1. **Forgetting to Rebuild Frontends**
   - Symptom: Old code still runs after changes
   - Solution: Always run `pnpm build` after frontend changes

2. **Incorrect Vite Base Path**
   - Symptom: Assets 404, page loads without styling
   - Solution: Verify `base: '/admin'` or `base: '/'` in vite.config.ts

3. **Missing Build Directories**
   - Symptom: "directory not found" warnings
   - Solution: Copy frontend builds to backend before running

4. **Route Registration Order**
   - Symptom: API routes return HTML
   - Solution: Ensure `/api/v1` is registered before static serving

5. **Trailing Slash Issues**
   - Symptom: Inconsistent behavior between `/admin` and `/admin/`
   - Solution: Axum handles both correctly, no action needed

### ✅ Best Practices

1. **Development Workflow:**
   ```bash
   # Option 1: Use build script (recommended)
   ./frontend-build.sh
   cd backend && cargo run

   # Option 2: Manual build
   cd frontend/admin && pnpm build
   cd ../web && pnpm build
   cd ../../backend && cargo run
   ```

2. **Git Repository:**
   - Add `frontend/admin/dist/` and `frontend/web/dist/` to `.gitignore`
   - These are build artifacts, not source code
   - Only commit `frontend/` source files
   - The `frontend-build.sh` script should be committed for convenience

3. **Docker Development:**
   - Use `docker-compose up --build` for full rebuilds
   - Mount volumes for hot reloading: `./frontend/admin:/app/admin`
   - Set `RUST_LOG=debug` for verbose logs

4. **Production Deployment:**
   - Always build frontends in Docker (multi-stage build)
   - Never mount local frontend builds in production
   - Verify build artifacts before deploying
   - Use semantic versioning for frontend releases

5. **Performance Monitoring:**
   - Monitor static file response times
   - Check for 404 errors (indicates missing assets)
   - Verify compression is working (check `Content-Encoding: gzip` response header)
   - Monitor compression ratios in production

6. **Security Hardening:**
   - Keep `admin_build_path` and `web_build_path` non-empty in production
   - Never serve from root `/` filesystem
   - Use HTTPS in production (reverse proxy)
   - Regularly update dependencies (tower-http, axum)

