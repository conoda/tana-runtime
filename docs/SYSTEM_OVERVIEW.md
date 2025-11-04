# Tana Blockchain - Complete System Overview

## High-Level Architecture

```
                                  INTERNET
                                     │
                ┌────────────────────┼────────────────────┐
                │                    │                    │
                ↓                    ↓                    ↓
        ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
        │ tana.network │    │  blockchain  │    │   runtime    │
        │   (Website)  │    │    .tana     │    │  (Local CLI) │
        │  Cloudflare  │    │   .network   │    │   Users      │
        │    Pages     │    │  (Your VPS)  │    │              │
        └──────────────┘    └──────────────┘    └──────────────┘
               │                    │                    │
               │ HTTPS              │ HTTPS              │ Local
               ↓                    ↓                    ↓
        ┌──────────────────────────────────────────────────────┐
        │                  Your DigitalOcean VPS                │
        │                  (Ubuntu 22.04 LTS)                   │
        │                                                        │
        │  ┌──────────────────────────────────────────────┐   │
        │  │           Nginx Reverse Proxy                 │   │
        │  │  • SSL/TLS termination (Let's Encrypt)       │   │
        │  │  • Rate limiting (10 req/s)                  │   │
        │  │  • CORS headers                              │   │
        │  │  • Security headers                          │   │
        │  │  Ports: 80 → 443 → 8080                     │   │
        │  └──────────────────────────────────────────────┘   │
        │                         │                            │
        │  ┌──────────────────────┼──────────────────────┐   │
        │  │                 Docker Compose                │   │
        │  │                                               │   │
        │  │  ┌─────────────────────────────────────┐    │   │
        │  │  │     Tana Ledger Service             │    │   │
        │  │  │     (Bun + Hono + Drizzle)         │    │   │
        │  │  │                                     │    │   │
        │  │  │  Endpoints:                        │    │   │
        │  │  │  • GET  /health                   │    │   │
        │  │  │  • GET  /users                    │    │   │
        │  │  │  • POST /users                    │    │   │
        │  │  │  • GET  /balances                 │    │   │
        │  │  │  • POST /balances                 │    │   │
        │  │  │  • GET  /transactions             │    │   │
        │  │  │  • POST /transactions             │    │   │
        │  │  │                                     │    │   │
        │  │  │  Port: 8080 (internal)            │    │   │
        │  │  │  CPU: 0.25-1.0 cores              │    │   │
        │  │  │  RAM: 256-512 MB                  │    │   │
        │  │  └─────────────────────────────────────┘    │   │
        │  │                    │                         │   │
        │  │                    ↓                         │   │
        │  │  ┌─────────────────────────────────────┐    │   │
        │  │  │        PostgreSQL 16                │    │   │
        │  │  │        (Alpine Linux)               │    │   │
        │  │  │                                     │    │   │
        │  │  │  Tables:                           │    │   │
        │  │  │  • users                           │    │   │
        │  │  │  • balances                        │    │   │
        │  │  │  • transactions                    │    │   │
        │  │  │  • currencies                      │    │   │
        │  │  │                                     │    │   │
        │  │  │  Port: 5432 (internal only)       │    │   │
        │  │  │  Storage: 20-100 GB                │    │   │
        │  │  └─────────────────────────────────────┘    │   │
        │  │                                               │   │
        │  │  ┌─────────────────────────────────────┐    │   │
        │  │  │        Redis 7                      │    │   │
        │  │  │        (Alpine Linux)               │    │   │
        │  │  │                                     │    │   │
        │  │  │  Usage: Future caching/sessions    │    │   │
        │  │  │  Port: 6379 (internal only)        │    │   │
        │  │  └─────────────────────────────────────┘    │   │
        │  │                                               │   │
        │  │  [ Contracts Service - Future ]              │   │
        │  │  [ Node Service - Future ]                   │   │
        │  └───────────────────────────────────────────────┘   │
        │                                                        │
        │  File System:                                         │
        │  /var/lib/docker/volumes/                            │
        │  ├── postgres_data/  (Database files)                │
        │  └── redis_data/     (Cache files)                   │
        └────────────────────────────────────────────────────────┘
```

## Component Breakdown

### 1. **Website/Playground** - `tana.network`

**What it is:**
- Public-facing website with interactive TypeScript playground
- Built with Astro + Svelte
- Deployed to Cloudflare Pages (free, global CDN)

**What it does:**
- Provides educational interface to learn Tana smart contracts
- Monaco editor with TypeScript autocomplete
- Simulates contract execution in browser
- Makes READ-ONLY API calls to blockchain data

**Technology:**
- Astro (static site generator)
- Svelte (reactive components)
- Monaco Editor (VS Code editor in browser)
- TypeScript compiler (runs in browser)

**Deployment:**
```bash
cd website
npm run build
# Deploy to Cloudflare Pages (manual or via GitHub Actions)
```

**Costs:** FREE (Cloudflare Pages free tier)

---

### 2. **Blockchain API** - `blockchain.tana.network`

**What it is:**
- REST API for blockchain data (users, balances, transactions)
- Backend for both playground AND production contracts

**What it does:**
- Stores and retrieves user accounts
- Manages currency balances (USD, BTC, custom currencies)
- Records all transactions
- Provides read-only access for playground
- Provides read/write access for actual blockchain (future)

**Technology:**
- **Runtime:** Bun (fast JavaScript runtime)
- **Framework:** Hono (lightweight HTTP framework)
- **Database:** Drizzle ORM + PostgreSQL
- **Validation:** Zod schemas

**API Endpoints:**
```
GET  /                        # Service info
GET  /health                  # Health check
GET  /users                   # List users
GET  /users/:id              # Get user
POST /users                   # Create user
GET  /balances               # List balances
GET  /balances?ownerId=...   # Get specific balance
POST /balances               # Set balance
GET  /transactions           # List transactions
POST /transactions           # Create transaction
```

**Deployment:**
- Runs in Docker container
- Port 8080 (behind Nginx)
- Auto-restarts on failure

**Costs:** Included in VPS cost

---

### 3. **Database** - PostgreSQL

**What it is:**
- Persistent storage for all blockchain data
- Not exposed to internet (internal only)

**What it stores:**

**Users Table:**
```sql
id            TEXT PRIMARY KEY
username      TEXT UNIQUE
display_name  TEXT
public_key    TEXT
bio           TEXT
created_at    TIMESTAMP
```

**Balances Table:**
```sql
owner_id       TEXT (user/team ID)
owner_type     TEXT (user or team)
currency_code  TEXT (USD, BTC, etc.)
amount         TEXT (stored as string for precision)
updated_at     TIMESTAMP
```

**Transactions Table:**
```sql
id             TEXT PRIMARY KEY
from_id        TEXT
to_id          TEXT
amount         TEXT
currency_code  TEXT
type           TEXT (transfer, deposit, withdrawal)
status         TEXT (pending, confirmed, failed)
metadata       JSONB
created_at     TIMESTAMP
```

**Currencies Table:**
```sql
code           TEXT PRIMARY KEY (USD, BTC)
name           TEXT
symbol         TEXT ($ , ₿)
decimals       INTEGER
is_active      BOOLEAN
created_at     TIMESTAMP
```

**Deployment:**
- Runs in Docker container
- Port 5432 (internal only)
- Data persisted to volume

**Backup Strategy:**
```bash
# Daily backup at 2 AM
docker-compose exec postgres pg_dump -U tana tana | gzip > backup.sql.gz
```

**Costs:** Included in VPS cost

---

### 4. **Cache** - Redis

**What it is:**
- In-memory data store (currently unused, ready for future)

**Future uses:**
- Session management
- API response caching
- Rate limiting counters
- WebSocket pub/sub

**Deployment:**
- Runs in Docker container
- Port 6379 (internal only)

**Costs:** Included in VPS cost

---

### 5. **Rust Runtime** - Local CLI

**What it is:**
- Deno-based JavaScript runtime written in Rust
- Executes smart contracts locally or on server
- Full parity with playground sandbox

**What it does:**
- Executes TypeScript smart contracts
- Provides same APIs as playground
- Makes REAL state changes to blockchain
- Gas metering for execution costs

**Modules Provided:**
- `tana:core` - console, version info
- `tana:data` - key-value storage (staging + commit)
- `tana:utils` - whitelisted fetch API
- `tana:block` - block context (height, timestamp, executor, gas)
- `tana:tx` - transaction staging and execution

**Usage:**
```bash
# Local development
cd runtime
cargo run

# Test contract
cargo run --release < examples/default.ts

# From root
bun run chaintest
```

**Deployment:**
- Used by Contracts Service (future)
- Not directly exposed to internet

**Costs:** Included in VPS cost (when deployed)

---

### 6. **Reverse Proxy** - Nginx

**What it is:**
- Entry point for all external traffic
- Handles SSL, rate limiting, security

**What it does:**
```
HTTP (port 80) → Redirect to HTTPS
    ↓
HTTPS (port 443) → SSL termination
    ↓
Rate limiting (10 req/s per IP)
    ↓
Security headers (HSTS, X-Frame-Options, etc.)
    ↓
CORS headers (allow tana.network)
    ↓
Proxy to Ledger Service (port 8080)
```

**Configuration:**
- SSL certificates via Let's Encrypt (auto-renewal)
- Rate limiting to prevent abuse
- Access logs for monitoring
- Error logs for debugging

**Costs:** FREE (included in VPS)

---

## Data Flow Examples

### Example 1: Playground User Queries Balance

```
User types:              const balance = await block.getBalance('alice', 'USD')
   ↓
Browser sandbox executes
   ↓
Makes HTTPS request to: GET https://blockchain.tana.network/balances?ownerId=alice&currencyCode=USD
   ↓
Nginx (rate limit check)
   ↓
Ledger Service receives request
   ↓
Queries PostgreSQL:     SELECT * FROM balances WHERE owner_id = 'alice' AND currency_code = 'USD'
   ↓
Returns:                { "ownerId": "alice", "amount": "1000.00", "currencyCode": "USD" }
   ↓
Browser displays:       Balance: 1000 USD
```

### Example 2: Playground Simulates Transaction

```
User types:              tx.transfer('alice', 'bob', 100, 'USD')
                        await tx.execute()
   ↓
Browser sandbox executes LOCALLY (no API call)
   ↓
Validates:
  - alice != bob ✓
  - amount > 0 ✓
  - gas limit not exceeded ✓
   ↓
Returns mock result:    { success: true, gasUsed: 100, changes: [...] }
   ↓
Browser displays:       ✓ Transaction executed successfully!
                       (NOTE: This is simulation only, no actual state change)
```

### Example 3: Production Contract Execution (Future)

```
Contract submitted:     curl -X POST https://blockchain.tana.network/contracts/execute
   ↓
Contracts Service receives
   ↓
Loads Rust runtime
   ↓
Runtime executes contract:
  - Reads state via Ledger API
  - Stages transactions
  - Validates all changes
  - Calculates gas
   ↓
If valid:
  - Commits changes to PostgreSQL
  - Records in transactions table
  - Updates balances
   ↓
Returns:                { blockHeight: 12346, txHash: "0x...", gasUsed: 1234 }
```

---

## Single VPS Deployment Guide

### Minimum VPS Requirements
- **CPU:** 2-4 cores
- **RAM:** 4-8 GB
- **Storage:** 50-100 GB SSD
- **Network:** 2-4 TB bandwidth
- **OS:** Ubuntu 22.04 LTS

### Recommended DigitalOcean Droplet
**Basic Plan:** $24/month (2 vCPUs, 4 GB RAM, 80 GB SSD)
**Premium Plan:** $48/month (4 vCPUs, 8 GB RAM, 160 GB SSD) ← Recommended

### What Runs on the VPS

```
Resource Usage (Estimated):

Nginx:             50 MB RAM, 0.1 CPU
Ledger Service:    256-512 MB RAM, 0.25-1.0 CPU
PostgreSQL:        512 MB-1 GB RAM, 0.5-1.0 CPU
Redis:             100-200 MB RAM, 0.1 CPU
Docker Overhead:   200-400 MB RAM, 0.1 CPU
────────────────────────────────────────────
Total (min):       ~1.2 GB RAM, ~1.0 CPU
Total (max):       ~2.6 GB RAM, ~2.5 CPU

Comfortable with 4 GB RAM + 2-4 CPUs
```

### Installation Steps

1. **Provision Droplet**
```bash
# Ubuntu 22.04 LTS
# 4 GB RAM, 2 vCPUs, 80 GB SSD
# $24-48/month
```

2. **Initial Setup**
```bash
# Update system
apt update && apt upgrade -y

# Install Docker
curl -fsSL https://get.docker.com | sh

# Install Docker Compose
apt install docker-compose-plugin -y

# Install Nginx
apt install nginx certbot python3-certbot-nginx -y
```

3. **Clone Repository**
```bash
git clone https://github.com/yourusername/tana.git
cd tana
```

4. **Configure Environment**
```bash
cp .env.example .env.production
nano .env.production
# Set POSTGRES_PASSWORD to strong password
# Set ALLOWED_ORIGINS to https://tana.network
```

5. **Start Services**
```bash
docker-compose -f docker-compose.prod.yml up -d
```

6. **Configure SSL**
```bash
certbot --nginx -d blockchain.tana.network
```

7. **Done!**
```bash
curl https://blockchain.tana.network/health
# Returns: {"status": "ok"}
```

---

## Monitoring & Maintenance

### Daily Checks (Automated)
```bash
# Health check (every 5 minutes)
curl https://blockchain.tana.network/health

# Log rotation (automatic)
# Backups (2 AM daily via cron)
```

### Weekly Checks (Manual)
```bash
# Check disk space
df -h

# Check container stats
docker stats

# Review logs for errors
docker-compose -f docker-compose.prod.yml logs --tail=100 tana-ledger
```

### Monthly Maintenance
```bash
# Update system
apt update && apt upgrade -y

# Update containers
docker-compose -f docker-compose.prod.yml pull
docker-compose -f docker-compose.prod.yml up -d

# Clean old images
docker system prune -a
```

---

## Costs Breakdown

### Initial Setup
- Domain (tana.network): $12/year
- SSL Certificate: FREE (Let's Encrypt)
- Setup time: ~2 hours

### Monthly Recurring
- **DigitalOcean VPS:** $24-48/month
- **Cloudflare Pages:** FREE
- **Bandwidth:** Included in VPS
- **Backups:** Included in VPS (DigitalOcean snapshots +$1-2/month optional)

**Total:** **$24-48/month** to start

### Scaling Costs (Future)
If you outgrow single VPS:
- Load balancer: +$12/month
- Additional VPS: +$24-48/month each
- Managed PostgreSQL: +$15/month (DigitalOcean)
- CDN bandwidth: ~$1/TB (if needed beyond Cloudflare)

---

## Growth Path

### Phase 1: MVP (Current) - Single VPS
```
Services: Ledger + Database + Redis
Traffic: < 1000 users
Cost: $24-48/month
```

### Phase 2: Production Launch - Single VPS
```
Services: + Contracts Service
Traffic: < 10,000 users
Cost: $48/month (upgrade to 4 vCPU)
```

### Phase 3: Scale Up - Multiple VPS
```
Services: + Node Service + Load Balancer
Traffic: < 100,000 users
Cost: $100-150/month (2-3 VPS + LB)
```

### Phase 4: Scale Out - Managed Services
```
Services: + Managed DB + CDN
Traffic: 100,000+ users
Cost: $300-500/month
```

---

## What's Ready RIGHT NOW

✅ **Fully Ready:**
1. Ledger Service (API) - Dockerfile created, tested locally
2. Database schema - Migrations ready
3. Docker Compose configs - Dev & prod
4. Playground - Environment detection, API integration
5. Documentation - Complete deployment guides

✅ **Can Deploy Today:**
- Provision VPS
- Install Docker
- Configure DNS
- Deploy Ledger API
- Playground already on Cloudflare Pages

✅ **Works Out of the Box:**
- Playground queries real blockchain data
- Users can test contracts safely (read-only)
- Full TypeScript autocomplete
- Storage simulation (localStorage)

⏳ **Future Work (Not blocking launch):**
- Contracts Service (actual contract execution)
- Node Service (consensus)
- Advanced monitoring

---

## Summary

**You have a working blockchain API ready to deploy today.**

**Architecture:**
- Simple: One VPS running Docker containers
- Scalable: Add more VPS as you grow
- Cost-effective: $24-48/month to start
- Professional: SSL, monitoring, backups included

**What Users Can Do Now:**
- Visit playground at tana.network
- Write TypeScript smart contracts
- Query real blockchain data (balances, users, transactions)
- Test contract validity
- Learn Tana APIs

**What's Next:**
1. Provision $24/month DigitalOcean VPS
2. Follow `/docs/DEPLOYMENT.md`
3. Deploy in ~2 hours
4. You're live!

**Questions or need clarification on any component?**
