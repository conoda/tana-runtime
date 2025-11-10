---
title: Edge Server Overview
description: Introduction to the Tana Edge HTTP server for off-chain contract execution
sidebar:
  order: 1
---

The Tana Edge server (`tana-edge`) is a high-performance HTTP server that executes TypeScript smart contracts off-chain, enabling fast API endpoints while maintaining blockchain integration for critical operations.

## Architecture

Edge follows a **hybrid on-chain/off-chain model** inspired by Cloudflare Workers:

- **On-chain** (`Init()`, `Contract()`) - Deployed via blockchain consensus, stores critical state
- **Off-chain** (`Get()`, `Post()`) - Executed on edge servers, handles HTTP requests at millisecond latency

Each HTTP request spawns a fresh V8 isolate (Cloudflare-style), ensuring security and isolation between requests.

## Why Edge?

Traditional blockchains are too slow for real-time applications. Edge solves this by:

- ⚡ **Speed** - HTTP requests complete in <100ms vs blockchain's seconds
- 💰 **Cost** - No gas fees for queries
- 🔄 **Flexibility** - Read data, compose responses, call external APIs
- 🔒 **Security** - Still backed by blockchain for critical operations

**Use Cases:**
- REST APIs for blockchain data
- Real-time dashboards and analytics
- File uploads and processing
- Comment systems, notifications
- Anything requiring fast reads with occasional writes

## Installation

```bash
# Build tana-edge from source
cd tana-edge
cargo build --release

# Binary will be at: ./target/release/tana-edge
```

## Quick Start

**1. Create a contract:**

```bash
# Create contract directory
mkdir -p contracts/hello

# Create GET endpoint
cat > contracts/hello/get.ts << 'EOF'
import { Request, Response } from 'tana/net'
import { console } from 'tana/core'

export function Get(req: Request) {
  console.log('Request received!')

  return Response.json({
    message: 'Hello from tana-edge!',
    timestamp: Date.now()
  })
}
EOF
```

**2. Start the server:**

```bash
cd tana-edge
./target/release/tana-edge
```

**3. Test your endpoint:**

```bash
curl http://localhost:8082/hello

# Response:
# {
#   "status": 200,
#   "body": {
#     "message": "Hello from tana-edge!",
#     "timestamp": 1762631458341
#   }
# }
```

## Contract Structure

Contracts live in `contracts/{contractId}/`:

```
contracts/
└── myapp/
    ├── get.ts   # Handles GET  /myapp
    └── post.ts  # Handles POST /myapp
```

Each file exports a function that receives a `Request` and returns a `Response`.

## Available Modules

Edge contracts have access to all Tana modules:

| Module | Purpose | Example |
|--------|---------|---------|
| `tana/core` | Console, version info | `console.log('hi')` |
| `tana/net` | HTTP requests/responses | `Response.json({...})` |
| `tana/block` | Blockchain queries | `block.getHeight()` |
| `tana/data` | Key-value storage | `await data.set('key', val)` |
| `tana/tx` | Transaction staging | `tx.transfer('alice', 'bob', 100)` |
| `tana/utils` | External HTTP calls | `await fetch('http://...')` |

See [Examples](/tana-edge/examples/) for complete code samples.

## Configuration

**Default Settings:**
- **Port:** 8082
- **Contracts Directory:** `../contracts/` (relative to binary)
- **CORS:** Permissive (all origins)

**Environment Variables:**
```bash
DATABASE_URL='postgres://tana:[PASSWORD]@localhost:5432/tana'  # For blockchain queries
```

## Production Deployment

Edge is designed for self-hosting:

```bash
# 1. Build optimized binary
cargo build --release

# 2. Run as systemd service
sudo systemctl start tana-edge

# 3. Map subdomains in reverse proxy
# Production routing:
# cnt_abc123.tana.network → localhost:8082/cnt_abc123
```

**Subdomain Routing:**

In production, contracts are accessed via subdomains:
- Dev: `localhost:8082/myapp` (path-based)
- Prod: `myapp.tana.network` (subdomain)

## Performance

- **Latency:** <100ms per request (fresh V8 isolate per request)
- **Throughput:** Thousands of requests/second
- **Memory:** Isolated per-request (no state leakage)
- **Scaling:** Horizontal (stateless design)

## Next Steps

- **[Examples](/tana-edge/examples/)** - Complete code examples
- **[API Reference](/tana-api/modules/)** - Available Tana modules
- **[Architecture Guide](/guides/architecture/)** - System design
