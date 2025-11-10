---
title: Environment Variables
description: Configuration via environment variables
sidebar:
  order: 3
---

Tana uses environment variables for configuration across all components. This provides a simple, consistent way to configure development, testing, and production environments.

## TANA_LEDGER_URL

**Purpose:** Configures the ledger API endpoint for all Tana components

**Default:** `http://localhost:8080`

**Usage:**
```bash
# Development (default - no env var needed)
tana new user @alice

# Remote chain
export TANA_LEDGER_URL=https://mainnet.tana.network
tana new user @alice

# Per-command override
TANA_LEDGER_URL=https://testnet.tana.network tana deploy user @alice
```

### Components That Use This Variable

**CLI (TypeScript/Bun):**
```typescript
// cli/utils/config.ts
export function getLedgerUrl(): string {
  return process.env.TANA_LEDGER_URL || 'http://localhost:8080'
}
```

**Contract Runner Script:**
```typescript
// scripts/run-contract.ts
const LEDGER_URL = process.env.TANA_LEDGER_URL || 'http://localhost:8080';
```

**Edge Server (Rust):**
```rust
// tana-edge/src/main.rs
use std::env;
let ledger_url = env::var("TANA_LEDGER_URL")
    .unwrap_or_else(|_| "http://localhost:8080".to_string());
```

**Mobile App (React Native/Expo):**
```javascript
// mobile/app.config.js
extra: {
  ledgerApiUrl: process.env.TANA_LEDGER_URL || 'http://localhost:8080'
}
```

### Framework-Specific Considerations

Different frameworks access environment variables differently:

**Bun/Node.js:**
```typescript
process.env.TANA_LEDGER_URL
```

**Astro:**
```typescript
import.meta.env.TANA_LEDGER_URL
// Add to .env file in project root
```

**Next.js:**
```typescript
// Server-side
process.env.TANA_LEDGER_URL

// Client-side (requires NEXT_PUBLIC_ prefix)
process.env.NEXT_PUBLIC_TANA_LEDGER_URL
```

**Vite:**
```typescript
import.meta.env.VITE_TANA_LEDGER_URL
// Add to .env file with VITE_ prefix
```

**Rust:**
```rust
use std::env;
env::var("TANA_LEDGER_URL").unwrap_or_else(|_| "default".to_string())
```

## Development Workflows

### Local Development

For local development, the default value (`http://localhost:8080`) works automatically:

```bash
# Start the ledger
tana start

# Use CLI (automatically connects to localhost:8080)
tana new user @alice
tana deploy user @alice
```

### Multi-Chain Development

Switch between chains by setting the environment variable:

```bash
# Work on local development chain
TANA_LEDGER_URL=http://localhost:8080 tana status

# Work on testnet
TANA_LEDGER_URL=https://testnet.tana.network tana status

# Work on custom community chain
TANA_LEDGER_URL=https://my-chain.example.com tana status
```

### Docker/Container Deployment

**Dockerfile:**
```dockerfile
ENV TANA_LEDGER_URL=https://mainnet.tana.network
```

**docker-compose.yml:**
```yaml
services:
  tana-edge:
    environment:
      - TANA_LEDGER_URL=https://mainnet.tana.network
```

**Kubernetes:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: tana-config
data:
  TANA_LEDGER_URL: "https://mainnet.tana.network"
```

### CI/CD Pipelines

**GitHub Actions:**
```yaml
env:
  TANA_LEDGER_URL: https://testnet.tana.network

steps:
  - name: Run tests
    run: bun test
    env:
      TANA_LEDGER_URL: http://localhost:8080
```

**GitLab CI:**
```yaml
variables:
  TANA_LEDGER_URL: "https://testnet.tana.network"
```

## Production Configuration

### Managed Hosting (tana.network)

When using the official tana.network hosted service:

```bash
export TANA_LEDGER_URL=https://api.tana.network
```

### Self-Hosted

When running your own blockchain infrastructure:

```bash
# Single instance
export TANA_LEDGER_URL=https://ledger.mycompany.com

# Load balanced
export TANA_LEDGER_URL=https://ledger-lb.mycompany.com

# Internal network
export TANA_LEDGER_URL=http://ledger.internal:8080
```

## Validation and Debugging

### Check Current Configuration

**CLI Status:**
```bash
tana status
# Shows: Ledger: ● Running on http://localhost:8080
```

**Environment Check:**
```bash
echo $TANA_LEDGER_URL
# Should output the URL or be empty (uses default)
```

### Common Issues

**Issue:** CLI can't connect to ledger
```bash
# Check if ledger is reachable
curl $TANA_LEDGER_URL/health

# Verify environment variable is set correctly
echo $TANA_LEDGER_URL
```

**Issue:** Wrong chain being targeted
```bash
# Check current configuration
tana status

# Override temporarily
TANA_LEDGER_URL=https://correct-chain.com tana status
```

**Issue:** Docker container can't reach localhost
```bash
# In Docker, localhost refers to the container
# Use host.docker.internal on Mac/Windows or host network mode on Linux

# Docker for Mac/Windows
export TANA_LEDGER_URL=http://host.docker.internal:8080

# Docker for Linux (use host network)
docker run --network host -e TANA_LEDGER_URL=http://localhost:8080 ...
```

## Future Environment Variables

As Tana develops, additional environment variables may be added:

- `TANA_EDGE_URL` - Edge server endpoint
- `TANA_STORAGE_URL` - Distributed storage endpoint
- `TANA_PRIVATE_KEY` - For automated deployments (use with caution)
- `TANA_LOG_LEVEL` - Logging verbosity (debug, info, warn, error)

Check this page for updates as new configuration options are added.

## Best Practices

1. **Never commit `.env` files with secrets** - Use `.env.example` instead
2. **Use different values per environment** - dev, staging, production
3. **Validate URLs before deployment** - Ensure endpoints are reachable
4. **Document custom chains** - Keep track of which URL points where
5. **Use HTTPS in production** - Never use `http://` for production chains
6. **Set sensible defaults** - localhost:8080 works for most development

## Related Documentation

- [Development Setup](/contributing/setup/) - Initial environment configuration
- [Architecture](/contributing/architecture/) - System design overview
- [Deployment Guide](#) - Production deployment best practices (coming soon)
