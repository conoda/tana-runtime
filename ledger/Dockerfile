# Tana Ledger Service - Dockerfile
# Multi-stage build for production deployment

# ============================================================================
# Stage 1: Dependencies
# ============================================================================
FROM oven/bun:1.1.38-alpine AS deps

WORKDIR /app

# Copy package files
COPY package.json bun.lockb* ./

# Install dependencies
RUN bun install --frozen-lockfile --production

# ============================================================================
# Stage 2: Build
# ============================================================================
FROM oven/bun:1.1.38-alpine AS builder

WORKDIR /app

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules

# Copy source files
COPY . .

# Build the application
RUN bun build src/index.ts --outdir dist --target bun

# ============================================================================
# Stage 3: Production
# ============================================================================
FROM oven/bun:1.1.38-alpine AS runner

WORKDIR /app

# Set production environment
ENV NODE_ENV=production
ENV PORT=8080

# Create non-root user for security
RUN addgroup -g 1001 -S tana && \
    adduser -S tana -u 1001

# Copy dependencies and built files
COPY --from=deps --chown=tana:tana /app/node_modules ./node_modules
COPY --from=builder --chown=tana:tana /app/dist ./dist
COPY --from=builder --chown=tana:tana /app/package.json ./package.json
COPY --from=builder --chown=tana:tana /app/migrations ./migrations
COPY --from=builder --chown=tana:tana /app/drizzle.config.ts ./drizzle.config.ts

# Switch to non-root user
USER tana

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD bun run -e "fetch('http://localhost:8080/health').then(r => r.ok ? process.exit(0) : process.exit(1))"

# Start the application
CMD ["bun", "run", "start"]
