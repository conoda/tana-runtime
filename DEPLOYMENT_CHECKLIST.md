# Tana Blockchain - Deployment Checklist

## ✅ What's Ready

### Infrastructure
- [x] **Ledger Service** - API for users, balances, transactions
  - Location: `/ledger`
  - Dockerfile: ✅ Created
  - Docker Compose: ✅ Configured
  - Endpoints: GET/POST for users, balances, transactions
  - Port: 8080

- [x] **Database Schema**
  - PostgreSQL with Drizzle ORM
  - Migrations ready
  - Tables: users, balances, transactions, currencies

- [x] **Container Configuration**
  - `docker-compose.yml` for development
  - `docker-compose.prod.yml` for production
  - Health checks configured
  - Resource limits set

### Playground Integration
- [x] **Environment Detection**
  - Automatically detects production vs development
  - Production API: `https://blockchain.tana.network`
  - Development API: `http://localhost:8080`

- [x] **Read-Only Mode**
  - State queries make real API calls (GET requests)
  - Transaction execution is simulated (no writes)
  - Safe for production deployment

- [x] **Domain Whitelist**
  - `tana.network` - Main site
  - `blockchain.tana.network` - API
  - `localhost` / `127.0.0.1` - Development

### Documentation
- [x] **Deployment Guide** (`/docs/DEPLOYMENT.md`)
- [x] **Security Model** (`/docs/PLAYGROUND_SECURITY.md`)
- [x] **Feature Parity** (`/docs/FEATURE_PARITY.md`)

## 🚧 Before First Deployment

### 1. Server Setup
- [ ] Provision server (4+ cores, 8GB+ RAM, 100GB+ SSD)
- [ ] Install Docker and Docker Compose
- [ ] Configure firewall (ports 80, 443, 8080, 8081, 9933, 30333)
- [ ] Set up monitoring (optional but recommended)

### 2. DNS Configuration
- [ ] Point `blockchain.tana.network` to server IP
- [ ] Verify DNS propagation: `dig blockchain.tana.network`

### 3. SSL Certificate
- [ ] Install certbot: `sudo apt install certbot python3-certbot-nginx`
- [ ] Get certificate: `sudo certbot --nginx -d blockchain.tana.network`
- [ ] Verify auto-renewal: `sudo certbot renew --dry-run`

### 4. Environment Configuration
- [ ] Create `.env.production` from `.env.example`
- [ ] Set strong `POSTGRES_PASSWORD`
- [ ] Configure `ALLOWED_ORIGINS`
- [ ] Review all environment variables

### 5. Database Setup
- [ ] Start database: `docker-compose -f docker-compose.prod.yml up -d postgres redis`
- [ ] Run migrations: `cd ledger && bun run db:migrate`
- [ ] Seed currencies: `curl -X POST http://localhost:8080/balances/currencies/seed`
- [ ] Create initial test users (optional)

### 6. Service Deployment
- [ ] Build images: `docker-compose -f docker-compose.prod.yml build`
- [ ] Start services: `docker-compose -f docker-compose.prod.yml up -d`
- [ ] Check status: `docker-compose -f docker-compose.prod.yml ps`
- [ ] Check logs: `docker-compose -f docker-compose.prod.yml logs -f`

### 7. Reverse Proxy
- [ ] Install Nginx: `sudo apt install nginx`
- [ ] Configure `/etc/nginx/sites-available/blockchain.tana.network`
- [ ] Enable site: `sudo ln -s /etc/nginx/sites-available/blockchain.tana.network /etc/nginx/sites-enabled/`
- [ ] Test config: `sudo nginx -t`
- [ ] Reload: `sudo systemctl reload nginx`

### 8. Testing
- [ ] Test health: `curl https://blockchain.tana.network/health`
- [ ] Test users endpoint: `curl https://blockchain.tana.network/users`
- [ ] Test balances endpoint: `curl https://blockchain.tana.network/balances`
- [ ] Test transactions endpoint: `curl https://blockchain.tana.network/transactions`
- [ ] Test CORS from playground

### 9. Monitoring
- [ ] Set up log rotation
- [ ] Configure database backups (cron job)
- [ ] Set up uptime monitoring (UptimeRobot, Pingdom, etc.)
- [ ] Configure alerts for service failures
- [ ] Monitor disk space usage

### 10. Security
- [ ] Review firewall rules: `sudo ufw status`
- [ ] Verify services not exposed: `netstat -tulpn | grep LISTEN`
- [ ] Check SSL rating: `https://www.ssllabs.com/ssltest/`
- [ ] Review Nginx security headers
- [ ] Test rate limiting
- [ ] Scan for vulnerabilities (optional)

## 📊 Post-Deployment

### Immediate (Day 1)
- [ ] Monitor logs for errors
- [ ] Check API response times
- [ ] Verify playground integration works
- [ ] Test all endpoints from production playground
- [ ] Document any issues

### Short-term (Week 1)
- [ ] Monitor resource usage (CPU, RAM, disk)
- [ ] Review access logs for unusual patterns
- [ ] Optimize database queries if needed
- [ ] Set up automated backups
- [ ] Create restore procedure documentation

### Long-term (Month 1)
- [ ] Review and optimize database indexes
- [ ] Consider CDN for static assets (if applicable)
- [ ] Implement caching strategy (Redis for hot data)
- [ ] Set up application performance monitoring
- [ ] Plan for scaling (if traffic grows)

## 🛠️ Missing / Future Work

### Optional Enhancements
- [ ] **Contracts Service** (Port 8081) - Smart contract execution
  - Needs Dockerfile
  - Integrates with Rust runtime
  - For actual blockchain state changes (not playground simulation)

- [ ] **Node Service** (Port 9933) - Blockchain consensus
  - Needs Dockerfile
  - P2P networking
  - Block production

- [ ] **Separate Sandbox Domain**
  - Deploy playground sandbox to `sandbox.tana.network`
  - Stronger isolation from main site
  - Recommended for production

- [ ] **Rate Limiting at Application Level**
  - Currently only Nginx rate limiting
  - Add per-user/per-IP limits in Ledger service
  - Implement API keys for heavy users

- [ ] **Caching Layer**
  - Redis cache for frequently accessed data
  - Reduce database load
  - Faster response times

- [ ] **Metrics & Analytics**
  - Prometheus + Grafana setup
  - Application metrics
  - Database metrics
  - API request analytics

## 📝 Quick Commands

### Development
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f tana-ledger

# Stop all services
docker-compose down
```

### Production
```bash
# Deploy/update
git pull origin main
docker-compose -f docker-compose.prod.yml build
docker-compose -f docker-compose.prod.yml up -d

# View logs
docker-compose -f docker-compose.prod.yml logs -f tana-ledger

# Restart service
docker-compose -f docker-compose.prod.yml restart tana-ledger

# Stop all
docker-compose -f docker-compose.prod.yml down
```

### Database
```bash
# Backup
docker-compose -f docker-compose.prod.yml exec postgres \
  pg_dump -U tana tana | gzip > backup_$(date +%Y%m%d).sql.gz

# Restore
gunzip < backup.sql.gz | \
  docker-compose -f docker-compose.prod.yml exec -T postgres \
  psql -U tana tana
```

## 🎯 Deployment Priority

**High Priority (Required for launch):**
1. ✅ Ledger Service with Dockerfile
2. Server provisioning
3. DNS configuration
4. SSL certificates
5. Production deployment

**Medium Priority (Nice to have):**
1. Monitoring and alerting
2. Automated backups
3. Separate sandbox domain
4. Application-level rate limiting

**Low Priority (Future improvements):**
1. Contracts service deployment
2. Node service deployment
3. Advanced caching
4. Metrics dashboard
5. Performance optimizations

---

**Current Status:** ✅ **LEDGER SERVICE READY TO DEPLOY**

The core blockchain API (`tana-ledger`) is production-ready with:
- Complete Dockerfile
- Production docker-compose configuration
- Health checks
- Resource limits
- CORS configuration
- Environment-based configuration
- Complete deployment documentation

**Next Steps:**
1. Provision server
2. Follow `/docs/DEPLOYMENT.md`
3. Deploy to `https://blockchain.tana.network`
