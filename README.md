# Tana Blockchain

A multi-currency blockchain ledger with TypeScript smart contracts and deterministic on-chain deployments.

## 🎯 Vision

Tana is a blockchain system that stores **everything deterministic on-chain**: code, compiled assets, balances, and state. Users can deploy full applications directly to the blockchain with immutable versioning and provable execution.

**Core Features:**
- 💰 Multi-currency ledger (fiat + crypto)
- 🔒 TypeScript smart contracts in sandboxed V8 runtime
- 👥 Users, teams, and channels
- 🌐 Deploy full web apps on-chain (HTML/CSS/JS)
- 🔍 Deterministic builds and time-travel debugging

---

## 📁 Monorepo Structure

```
tana-runtime/                    # Monorepo root
├── runtime/                     # Rust - V8 TypeScript execution engine
│   ├── src/                     # Rust source (deno_core)
│   └── Cargo.toml
│
├── node/                        # TypeScript/Bun - Blockchain node
│   ├── src/                     # P2P, consensus, storage
│   └── package.json
│
├── ledger/                      # TypeScript/Bun - Account & balance service
│   ├── src/                     # Users, teams, transactions
│   └── package.json
│
├── contracts/                   # TypeScript/Bun - Contract executor
│   ├── src/                     # Deployment & execution
│   └── package.json
│
├── cli/                         # TypeScript/Bun - Command-line tools
│   ├── src/                     # User-facing commands
│   └── package.json
│
├── website/                     # Astro/Svelte - Main website & playground
│   └── src/
│
├── types/                       # Shared TypeScript type definitions
│   ├── tana-core.d.ts
│   ├── tana-data.d.ts
│   └── tana-utils.d.ts
│
├── docs/                        # Documentation
│   ├── DATA_STORAGE.md
│   ├── FEATURE_PARITY.md
│   └── STORAGE_*.md
│
├── TODO.md                      # Project roadmap & architecture
├── docker-compose.yml           # All services orchestration
└── package.json                 # Workspace management
```

---

## 🚀 Quick Start

### Prerequisites

- [Bun](https://bun.sh) >= 1.0 (for TypeScript services)
- [Rust](https://rustup.rs) >= 1.70 (for runtime only)
- [Docker](https://docker.com) (optional, for databases)

### Installation

```bash
# Install all dependencies
bun install

# Build Rust runtime
cd runtime && cargo build --release && cd ..
```

### Development

```bash
# Start all services with Docker
docker compose up

# Or run services individually:
bun run dev:ledger      # Account service (port 8080)
bun run dev:contracts   # Contract executor (port 8081)
bun run dev:node        # Blockchain node (port 9933)
bun run dev:website     # Website (port 4322)
bun run dev:runtime     # Rust runtime (CLI)

# Or run all TypeScript services at once
bun run dev
```

### Testing

```bash
# Run all tests
bun test

# Test specific service
bun run --filter @tana/ledger test

# Test Rust runtime
cd runtime && cargo test
```

---

## 🏗️ Service Overview

### Runtime (Rust)
**Purpose:** Sandboxed V8 TypeScript execution engine

- Execute smart contracts in isolated environment
- Provide `tana:core`, `tana:data`, `tana:utils` APIs
- No network access, filesystem, or system calls
- Deterministic execution

📖 [Full Runtime Documentation](./runtime/README.md)

### Node (TypeScript/Bun)
**Purpose:** Blockchain node with P2P networking

- Block production & validation
- P2P networking (libp2p)
- JSON-RPC API
- Consensus mechanism

📖 [Node Documentation](./node/README.md)

### Ledger (TypeScript/Bun)
**Purpose:** Account and balance management

- User/Team account CRUD
- Multi-currency balances
- Transaction validation
- REST API

📖 [Ledger Documentation](./ledger/README.md)

### Contracts (TypeScript/Bun)
**Purpose:** Smart contract deployment & execution

- Deploy contracts on-chain
- Execute via runtime (subprocess or FFI)
- Redis state storage
- Gas metering

📖 [Contracts Documentation](./contracts/README.md)

### CLI (TypeScript/Bun)
**Purpose:** Command-line tools for users

```bash
tana account create
tana send @bob 10 USD
tana deploy contract.ts
tana call @contract/counter increment
```

📖 [CLI Documentation](./cli/README.md)

### Website (Astro/Svelte)
**Purpose:** Main website & browser playground

- Interactive code editor (Monaco)
- Run contracts in browser
- Documentation
- Block explorer (future)

---

## 📚 Documentation

- [TODO.md](./TODO.md) - Project roadmap and architecture decisions
- [Data Storage](./docs/DATA_STORAGE.md) - Storage API design
- [Feature Parity](./docs/FEATURE_PARITY.md) - Cross-environment compatibility

---

## 🔧 Development Workflow

### Working on a Service

```bash
# Navigate to service
cd ledger

# Install dependencies (if needed)
bun install

# Run in development mode
bun run dev

# Run tests
bun test

# Build for production
bun run build
```

### Adding a New Dependency

```bash
# Add to specific service
cd ledger
bun add postgres

# Add to root (shared dev tools)
cd ..
bun add -D typescript
```

### Database Migrations

```bash
# Ledger service (PostgreSQL)
cd ledger
bun run db:generate   # Generate migration
bun run db:migrate    # Run migrations

# Contracts service (Redis)
# No migrations needed - key-value store
```

---

## 🐳 Docker Setup

```bash
# Start all services
docker compose up

# Start in background
docker compose up -d

# View logs
docker compose logs -f

# Stop all services
docker compose down

# Reset everything (including volumes)
docker compose down -v
```

**Services:**
- `postgres` - PostgreSQL database (port 5432)
- `redis` - Redis cache (port 6379)
- `tana-ledger` - Ledger API (port 8080)
- `tana-contracts` - Contracts API (port 8081)
- `tana-node` - Node RPC (port 9933)
- `tana-website` - Website (port 4322)

---

## 🧪 Example Smart Contract

```typescript
import { console } from 'tana:core'
import { data } from 'tana:data'

// Simple counter contract
const current = await data.get('counter')
const count = current ? parseInt(current) : 0

console.log('Current count:', count)

await data.set('counter', String(count + 1))
await data.commit()

console.log('Counter incremented!')
```

**Run it:**

```bash
# Via CLI
tana deploy examples/counter.ts

# Via Rust runtime
cd runtime
cargo run -- example.ts

# Via browser playground
open http://localhost:4322
```

---

## 🤝 Contributing

This is an experimental project. Contributions welcome!

1. Pick an issue or feature from [TODO.md](./TODO.md)
2. Create a branch
3. Make changes and test
4. Submit a PR

---

## 📝 License

MIT (or your chosen license)

---

## 🔗 Links

- [Architecture & Roadmap](./TODO.md)
- [Runtime Documentation](./runtime/README.md)
- [Data Storage Design](./docs/DATA_STORAGE.md)

---

**Status:** Early development - Not production ready

Built with Rust (deno_core), TypeScript, Bun, PostgreSQL, Redis, and Astro.
