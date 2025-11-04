# Tana Blockchain - Production Deployment Guide

## Overview

This guide covers deploying the Tana blockchain infrastructure to production at `https://blockchain.tana.network`.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     tana.network                            │
│                  (Main Website/Playground)                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ↓ HTTPS
┌─────────────────────────────────────────────────────────────┐
│            blockchain.tana.network (Load Balancer)          │
└─────────────────────────────────────────────────────────────┘
                            │
           ┌────────────────┼────────────────┐
           ↓                ↓                ↓
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ Ledger   │    │ Contracts│    │   Node   │
    │ Service  │    │ Service  │    │ Service  │
    │  :8080   │    │  :8081   │    │  :9933   │
    └──────────┘    └──────────┘    └──────────┘
           │                │                │
           └────────────────┼────────────────┘
                            ↓
                    ┌──────────────┐
                    │  PostgreSQL  │
                    │     :5432    │
                    └──────────────┘
                            │
                    ┌──────────────┐
                    │    Redis     │
                    │     :6379    │
                    └──────────────┘
```

## Services

### 1. Ledger Service (Port 8080)
**Repository:** `/ledger`
**Purpose:** Blockchain data API - users, balances, transactions
**Endpoints:**
- `GET /` - Service health and info
- `GET /health` - Health check
- `GET /users` - List all users
- `GET /users/:id` - Get specific user
- `POST /users` - Create user
- `GET /balances` - Get all balances
- `POST /balances` - Set balance
- `GET /transactions` - Get all transactions
- `POST /transactions` - Create transaction

### 2. Contracts Service (Port 8081)
**Repository:** `/contracts`
**Purpose:** Smart contract execution using Rust runtime
**Features:**
- Contract deployment
- Contract execution with gas metering
- Transaction validation
- State management

### 3. Node Service (Port 9933)
**Repository:** `/node`
**Purpose:** Blockchain node and consensus
**Features:**
- JSON-RPC API
- P2P networking
- Block production
- Transaction processing

## Prerequisites

### Server Requirements
- **CPU:** 4+ cores
- **RAM:** 8GB+ minimum, 16GB recommended
- **Storage:** 100GB+ SSD
- **OS:** Ubuntu 22.04 LTS (recommended) or similar
- **Network:** Static IP, ports 80/443/8080/8081/9933/30333 accessible

### Software Requirements
- Docker 24.0+
- Docker Compose 2.0+
- Bun 1.1.38+ (for local development)
- PostgreSQL 16 (via Docker)
- Redis 7 (via Docker)

### Domain Configuration
Configure DNS records:
```
A     blockchain.tana.network    → <SERVER_IP>
AAAA  blockchain.tana.network    → <SERVER_IPv6>  (optional)
```

## Installation

### 1. Clone Repository
```bash
git clone https://github.com/yourusername/tana.git
cd tana
```

### 2. Create Production Environment File
```bash
cp .env.example .env.production
```

Edit `.env.production`:
```bash
# Database Configuration
POSTGRES_DB=tana
POSTGRES_USER=tana
POSTGRES_PASSWORD=<STRONG_PASSWORD_HERE>

# Service Configuration
NODE_ENV=production
LOG_LEVEL=info

# CORS Origins
ALLOWED_ORIGINS=https://tana.network,https://blockchain.tana.network,https://playground.tana.network

# Optional: External monitoring
# SENTRY_DSN=https://...
# METRICS_ENDPOINT=https://...
```

### 3. Build Docker Images
```bash
# Build all services
docker-compose -f docker-compose.prod.yml build

# Or build specific service
docker-compose -f docker-compose.prod.yml build tana-ledger
```

### 4. Initialize Database
```bash
# Start database services
docker-compose -f docker-compose.prod.yml up -d postgres redis

# Wait for postgres to be ready
docker-compose -f docker-compose.prod.yml exec postgres pg_isready

# Run migrations
cd ledger
bun run db:migrate

# Seed default data (currencies, etc.)
curl -X POST http://localhost:8080/balances/currencies/seed
```

### 5. Start Services
```bash
# Start all services
docker-compose -f docker-compose.prod.yml up -d

# Check status
docker-compose -f docker-compose.prod.yml ps

# View logs
docker-compose -f docker-compose.prod.yml logs -f

# View specific service logs
docker-compose -f docker-compose.prod.yml logs -f tana-ledger
```

### 6. Verify Deployment
```bash
# Test ledger service
curl http://localhost:8080/health
curl http://localhost:8080/

# Test endpoints
curl http://localhost:8080/users
curl http://localhost:8080/balances
curl http://localhost:8080/transactions
```

## Reverse Proxy Configuration

### Using Nginx

Create `/etc/nginx/sites-available/blockchain.tana.network`:

```nginx
# HTTP -> HTTPS redirect
server {
    listen 80;
    listen [::]:80;
    server_name blockchain.tana.network;

    location / {
        return 301 https://$server_name$request_uri;
    }
}

# HTTPS
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name blockchain.tana.network;

    # SSL certificates (Let's Encrypt recommended)
    ssl_certificate /etc/letsencrypt/live/blockchain.tana.network/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/blockchain.tana.network/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # CORS headers
    add_header Access-Control-Allow-Origin "https://tana.network" always;
    add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
    add_header Access-Control-Allow-Headers "Content-Type, Authorization" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
    limit_req zone=api_limit burst=20 nodelay;

    # Proxy to ledger service
    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;

        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # Health check endpoint (no rate limiting)
    location /health {
        proxy_pass http://localhost:8080/health;
        access_log off;
    }
}
```

Enable and test:
```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/blockchain.tana.network /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Reload nginx
sudo systemctl reload nginx
```

### SSL Certificate (Let's Encrypt)
```bash
# Install certbot
sudo apt update
sudo apt install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d blockchain.tana.network

# Auto-renewal is enabled by default
# Test renewal
sudo certbot renew --dry-run
```

## Monitoring & Maintenance

### View Logs
```bash
# All services
docker-compose -f docker-compose.prod.yml logs -f

# Specific service
docker-compose -f docker-compose.prod.yml logs -f tana-ledger

# Last 100 lines
docker-compose -f docker-compose.prod.yml logs --tail=100 tana-ledger
```

### Database Backups
```bash
# Backup script
#!/bin/bash
BACKUP_DIR="/var/backups/tana"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR

# Backup database
docker-compose -f docker-compose.prod.yml exec -T postgres \
  pg_dump -U tana tana | gzip > "$BACKUP_DIR/tana_$TIMESTAMP.sql.gz"

# Keep only last 7 days
find $BACKUP_DIR -name "tana_*.sql.gz" -mtime +7 -delete
```

Add to crontab:
```bash
# Daily backup at 2 AM
0 2 * * * /path/to/backup-script.sh
```

### Health Monitoring
```bash
# Check service health
curl https://blockchain.tana.network/health

# Monitor container stats
docker stats

# Check service status
docker-compose -f docker-compose.prod.yml ps
```

### Updates & Upgrades
```bash
# Pull latest code
git pull origin main

# Rebuild images
docker-compose -f docker-compose.prod.yml build

# Run database migrations
cd ledger && bun run db:migrate && cd ..

# Restart services (zero downtime with rolling restart)
docker-compose -f docker-compose.prod.yml up -d --no-deps --build tana-ledger
docker-compose -f docker-compose.prod.yml up -d --no-deps --build tana-contracts
docker-compose -f docker-compose.prod.yml up -d --no-deps --build tana-node
```

## Troubleshooting

### Service Won't Start
```bash
# Check logs
docker-compose -f docker-compose.prod.yml logs tana-ledger

# Check container status
docker-compose -f docker-compose.prod.yml ps

# Restart specific service
docker-compose -f docker-compose.prod.yml restart tana-ledger
```

### Database Connection Issues
```bash
# Check postgres is running
docker-compose -f docker-compose.prod.yml ps postgres

# Check connection from ledger service
docker-compose -f docker-compose.prod.yml exec tana-ledger \
  sh -c 'bun run -e "console.log(process.env.DATABASE_URL)"'

# Test connection manually
docker-compose -f docker-compose.prod.yml exec postgres \
  psql -U tana -d tana -c "SELECT version();"
```

### High Memory Usage
```bash
# Check resource usage
docker stats

# Adjust resource limits in docker-compose.prod.yml
# services:
#   tana-ledger:
#     deploy:
#       resources:
#         limits:
#           memory: 512M
```

### CORS Errors
Check `ALLOWED_ORIGINS` in `.env.production` includes the playground domain:
```bash
ALLOWED_ORIGINS=https://tana.network,https://blockchain.tana.network
```

## Security Checklist

- [ ] Strong PostgreSQL password set
- [ ] SSL certificates installed and valid
- [ ] Firewall configured (UFW or iptables)
- [ ] Rate limiting enabled in Nginx
- [ ] Services running as non-root user
- [ ] Database not exposed to public internet
- [ ] Redis not exposed to public internet
- [ ] CORS properly configured
- [ ] Security headers enabled
- [ ] Regular backups configured
- [ ] Monitoring and alerting set up
- [ ] Secrets stored securely (not in git)

## Performance Tuning

### PostgreSQL
Edit `docker-compose.prod.yml` to add PostgreSQL performance settings:
```yaml
postgres:
  command: >
    postgres
    -c shared_buffers=256MB
    -c effective_cache_size=1GB
    -c maintenance_work_mem=128MB
    -c checkpoint_completion_target=0.9
    -c wal_buffers=16MB
    -c default_statistics_target=100
    -c random_page_cost=1.1
    -c effective_io_concurrency=200
    -c work_mem=16MB
    -c min_wal_size=1GB
    -c max_wal_size=4GB
```

### Connection Pooling
Consider adding PgBouncer for connection pooling:
```yaml
pgbouncer:
  image: pgbouncer/pgbouncer:latest
  environment:
    DATABASES_HOST: postgres
    DATABASES_PORT: 5432
    DATABASES_USER: tana
    DATABASES_PASSWORD: ${POSTGRES_PASSWORD}
    DATABASES_DBNAME: tana
    PGBOUNCER_POOL_MODE: transaction
    PGBOUNCER_MAX_CLIENT_CONN: 1000
    PGBOUNCER_DEFAULT_POOL_SIZE: 20
```

## Cost Estimation

### Minimum Setup (1-2 core VPS)
- **DigitalOcean:** $12-24/month (Basic Droplet)
- **Hetzner:** €5-15/month (CX21-CX31)
- **Linode:** $12-24/month (Nanode-Linode 2GB)

### Recommended Setup (4 core VPS)
- **DigitalOcean:** $48/month (Premium Droplet)
- **Hetzner:** €20/month (CX41)
- **AWS Lightsail:** $40/month

### Additional Costs
- Domain: $10-20/year
- Backups: $5-10/month (optional)
- Monitoring: Free (self-hosted) or $20/month (external)

## Support

For deployment issues:
1. Check logs: `docker-compose logs -f`
2. Review documentation in `/docs`
3. Open GitHub issue with deployment details
4. Contact: [your-email@domain.com]

## Related Documentation

- `/docs/FEATURE_PARITY.md` - Runtime vs Playground feature parity
- `/docs/PLAYGROUND_SECURITY.md` - Playground security model
- `/docs/CONTRACT_EXECUTION.md` - Contract execution details
- `/ledger/README.md` - Ledger service documentation
- `/runtime/README.md` - Rust runtime documentation
