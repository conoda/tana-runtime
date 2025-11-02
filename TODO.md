# Tana Blockchain - Architecture & TODO

> Created: 2025-11-02
> Status: Planning Phase
> Project: Multi-currency blockchain ledger with TypeScript smart contracts

---

## 🎯 Core Vision

A **blockchain ledger system** that:
- Stores multi-currency balances (fiat + crypto)
- Executes TypeScript smart contracts in sandboxed runtime
- Supports users, teams, channels with dynamic landing pages
- Deploys code directly to blockchain as running applications

---

## 📊 Architecture Decisions

### Q1: What Actually Goes On-Chain?

**YES - Stored on Blockchain:**
- ✅ User accounts (ID, metadata, public key)
- ✅ Team memberships
- ✅ Channel definitions
- ✅ Account balances (multi-currency ledger)
- ✅ Transactions (with signatures)
- ✅ Smart contract code (hashed and stored)
- ✅ Smart contract state (key-value data)
- ✅ Block headers (Merkle roots, timestamps)

**NO - Off-Chain (Database/Cache):**
- ❌ Large media files (IPFS/CDN instead)
- ❌ Rendered HTML (generated on-demand)
- ❌ Temporary session data
- ❌ Search indexes (PostgreSQL)

**Hybrid (Hash on-chain, data off-chain):**
- 🔀 Landing page code (hash on-chain, HTML in IPFS/storage)
- 🔀 Profile images (hash on-chain, image in CDN)

---

## 🗂️ Core Data Model

### Primary Objects

```typescript
// 1. USER
interface User {
  id: string                    // Unique ID (address)
  publicKey: string             // Ed25519 public key
  username: string              // @alice
  displayName: string           // "Alice Johnson"
  metadata: {
    bio?: string
    avatar?: string             // IPFS hash
    landingPageCode?: string    // Hash of deployed code
  }
  balances: Map<Currency, Decimal>  // Multi-currency
  createdAt: timestamp
  stateHash: string             // Merkle root of account state
}

// 2. TEAM
interface Team {
  id: string                    // Unique team ID
  name: string                  // "Acme Corp"
  slug: string                  // @acme
  members: Array<{
    userId: string
    role: 'owner' | 'admin' | 'member'
    joinedAt: timestamp
  }>
  balances: Map<Currency, Decimal>  // Team treasury
  metadata: {
    description?: string
    avatar?: string
    landingPageCode?: string
  }
  createdAt: timestamp
}

// 3. CHANNEL
interface Channel {
  id: string                    // Unique channel ID
  name: string                  // "general"
  slug: string                  // #general
  teamId?: string               // Optional team ownership
  visibility: 'public' | 'private' | 'team'
  members: string[]             // User IDs with access
  messages: Array<{
    id: string
    authorId: string
    content: string
    timestamp: timestamp
    signature: string           // Ed25519
  }>
  metadata: {
    description?: string
    landingPageCode?: string
  }
  createdAt: timestamp
}

// 4. TRANSACTION
interface Transaction {
  id: string                    // Tx hash
  from: string                  // User/Team ID
  to: string                    // User/Team ID
  amount: Decimal
  currency: Currency            // USD, BTC, ETH, etc.
  type: 'transfer' | 'deposit' | 'withdraw' | 'contract_call'
  contractId?: string           // If contract execution
  contractInput?: any           // Arguments
  signature: string             // Ed25519
  timestamp: timestamp
  blockId: string               // Block inclusion
  status: 'pending' | 'confirmed' | 'failed'
}

// 5. CURRENCY
interface Currency {
  code: string                  // "USD", "BTC", "ETH"
  type: 'fiat' | 'crypto'
  decimals: number              // Precision (e.g., 2 for USD, 8 for BTC)
  verified: boolean             // Is this a recognized currency?
}

// 6. SMART CONTRACT
interface SmartContract {
  id: string                    // Contract address
  codeHash: string              // SHA-256 of code
  code: string                  // TypeScript source
  author: string                // User ID
  deployedAt: timestamp
  storage: Map<string, string>  // Key-value state (tana:data)
  metadata: {
    name?: string
    description?: string
    version?: string
  }
}

// 7. LANDING PAGE (Deployed Code)
interface LandingPage {
  owner: string                 // User/Team/Channel ID
  codeHash: string              // Hash of HTML/TS
  staticHTML: string            // Base template
  islandComponents: Array<{     // Dynamic islands
    id: string
    contractId: string          // Smart contract for data
    position: string            // CSS selector
  }>
  deployedAt: timestamp
}

// 8. BLOCK
interface Block {
  id: string                    // Block hash
  height: number                // Block number
  timestamp: timestamp
  previousHash: string          // Previous block
  transactions: string[]        // Tx hashes
  stateRoot: string             // Merkle root of all account states
  validatorSignature: string    // Block producer signature
}
```

---

## 🏗️ Service Architecture

### Repository Structure

```
conoda/
├── tana-runtime/          # THIS REPO - Core runtime + website
│   ├── src/               # Rust runtime (V8 + TypeScript execution)
│   ├── website/           # Main website (Astro/Svelte)
│   ├── types/             # Shared TypeScript definitions
│   └── examples/          # Example smart contracts
│
├── tana-cli/              # SEPARATE REPO - CLI tools
│   ├── commands/          # User-facing commands (deploy, query, etc.)
│   └── lib/               # Shared client library
│
├── tana-node/             # NEW REPO - Blockchain node
│   ├── validator/         # Block validation & consensus
│   ├── p2p/               # Network layer
│   ├── storage/           # Block/transaction storage
│   └── api/               # JSON-RPC API server
│
├── tana-ledger/           # NEW REPO - Ledger service
│   ├── accounts/          # User/Team account management
│   ├── balances/          # Multi-currency balance tracking
│   ├── transactions/      # Transaction processing
│   └── migrations/        # PostgreSQL schema
│
└── tana-contracts/        # NEW REPO - Contract executor service
    ├── executor/          # Sandboxed contract execution
    ├── storage/           # Contract state (Redis)
    └── api/               # Contract deployment & calls
```

---

## 🔧 Service Responsibilities

### 1. **tana-runtime** (This Repo)
**Purpose:** Sandboxed TypeScript execution engine + project website

**Responsibilities:**
- Execute smart contracts in isolated V8 runtime
- Provide `tana:core`, `tana:data`, `tana:utils` APIs
- Host main project website at `/website`
- TypeScript type definitions for contract development
- Browser playground for testing contracts

**Stack:** Rust (deno_core), Astro, Svelte, Monaco Editor

**NOT responsible for:**
- Block validation (→ tana-node)
- Balance tracking (→ tana-ledger)
- Network communication (→ tana-node)

---

### 2. **tana-cli** (Existing Separate Repo)
**Purpose:** Command-line tools for developers & users

**Responsibilities:**
- Deploy smart contracts (`tana deploy contract.ts`)
- Query balances (`tana balance @alice`)
- Send transactions (`tana send @bob 10 USD`)
- Manage keys (`tana keys generate`)
- Interact with node API

**Stack:** TypeScript/Bun or Rust

**NOT responsible for:**
- Running nodes (→ tana-node)
- Executing contracts (→ tana-contracts)

---

### 3. **tana-node** (New Service)
**Purpose:** Blockchain node (validator/observer)

**Responsibilities:**
- P2P network communication
- Block production & validation
- Transaction mempool
- Consensus mechanism (PoS, PoA, etc.)
- JSON-RPC API for clients
- Sync with network

**Stack:** Rust (libp2p, tokio), PostgreSQL

**Docker Services:**
- `tana-node` (main binary)
- `postgres` (block/tx storage)

---

### 4. **tana-ledger** (New Service)
**Purpose:** Account & balance management

**Responsibilities:**
- User/Team account CRUD
- Multi-currency balance tracking
- Transaction validation (sufficient funds, etc.)
- Account state hashing
- Currency registry

**Stack:** Rust or Go, PostgreSQL

**Database Tables:**
- `accounts` (users, teams, balances)
- `transactions` (pending & confirmed)
- `currencies` (supported currencies)

---

### 5. **tana-contracts** (New Service)
**Purpose:** Smart contract deployment & execution

**Responsibilities:**
- Deploy contracts (store code + hash)
- Execute contract calls (via tana-runtime)
- Manage contract state (Redis KV store)
- Gas metering & limits
- Contract versioning

**Stack:** Rust, Redis, tana-runtime (as library)

**Docker Services:**
- `tana-contracts` (executor)
- `redis` (contract state storage)

---

## 🚀 Development Roadmap

### Phase 1: Foundation (Current)
- [x] V8 runtime with TypeScript support
- [x] `tana:core`, `tana:data`, `tana:utils` modules
- [x] Browser playground
- [x] Storage API with localStorage
- [ ] Landing page concept design
- [ ] Data model finalization

### Phase 2: Core Ledger
- [ ] Create `tana-ledger` service
- [ ] PostgreSQL schema for accounts/balances
- [ ] User account creation & management
- [ ] Multi-currency balance tracking
- [ ] Transaction submission & validation
- [ ] RESTful API for ledger operations

### Phase 3: Smart Contracts
- [ ] Create `tana-contracts` service
- [ ] Redis integration for contract state
- [ ] Contract deployment API
- [ ] Contract execution via tana-runtime
- [ ] Gas metering system
- [ ] Contract state inspection tools

### Phase 4: Blockchain Node
- [ ] Create `tana-node` service
- [ ] Block structure & validation
- [ ] Simple consensus (single validator → PoA later)
- [ ] P2P networking (libp2p)
- [ ] Merkle tree for state roots
- [ ] Block explorer UI

### Phase 5: Teams & Channels
- [ ] Team creation & membership
- [ ] Team treasury (shared balances)
- [ ] Channel creation (public/private)
- [ ] Message signing & verification
- [ ] Access control system

### Phase 6: Landing Pages
- [ ] Landing page deployment API
- [ ] Static HTML + dynamic islands architecture
- [ ] On-demand rendering service
- [ ] IPFS integration for static assets
- [ ] Custom domain mapping

### Phase 7: CLI Integration
- [ ] Update `tana-cli` for all new features
- [ ] Key management
- [ ] Deployment commands
- [ ] Query commands
- [ ] Interactive setup wizard

### Phase 8: Production Ready
- [ ] Multi-node network
- [ ] Consensus upgrade (PoS or PoA)
- [ ] Web wallet UI
- [ ] Mobile apps
- [ ] Monitoring & alerting
- [ ] Security audit

---

## 📝 Immediate Next Steps (Week 1)

### 1. Finalize Data Model
- [ ] Review and approve the data structures above
- [ ] Create SQL schema for accounts/balances
- [ ] Design API endpoints for ledger service
- [ ] Document currency support requirements

### 2. Service Scaffolding
- [ ] Create `tana-ledger` repository
- [ ] Setup PostgreSQL with Docker Compose
- [ [ ] Implement basic User CRUD
- [ ] Implement basic balance tracking

### 3. Landing Page Proof of Concept
- [ ] Design example landing page (static HTML + islands)
- [ ] Create proof-of-concept in `/website`
- [ ] Document architecture pattern
- [ ] Test dynamic data loading from contract

### 4. Documentation
- [ ] Architecture diagram (services + data flow)
- [ ] API specification (OpenAPI/Swagger)
- [ ] Developer guide for smart contracts
- [ ] Deployment guide for running nodes

---

## 🤔 Open Questions

### Technical Decisions Needed

1. **Consensus Mechanism:**
   - Start with single validator (centralized)?
   - Proof of Authority (PoA) with trusted validators?
   - Proof of Stake (PoS) eventually?

2. **Currency Support:**
   - How to verify fiat balances? (Oracle integration?)
   - Support for ERC-20 tokens?
   - Bridge to other blockchains?

3. **Landing Pages:**
   - IPFS for static assets or own CDN?
   - Server-side rendering or client-side only?
   - Caching strategy?

4. **Smart Contract Limits:**
   - Max execution time? (gas)
   - Max storage per contract?
   - Versioning & upgrades?

5. **Channels:**
   - Store all messages on-chain (expensive)?
   - Use IPFS for message history?
   - Retention policy?

6. **Node Requirements:**
   - Minimum hardware specs?
   - Incentives for running nodes?
   - Validator rewards?

---

## 🔄 Docker Compose Architecture

```yaml
# docker-compose.yml (future state)
version: '3.8'

services:
  # Database
  postgres:
    image: postgres:16
    volumes:
      - ledger-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: tana_ledger

  redis:
    image: redis:7-alpine
    volumes:
      - contract-data:/data

  # Services
  tana-node:
    build: ./tana-node
    ports:
      - "9933:9933"  # JSON-RPC
      - "30333:30333"  # P2P
    depends_on:
      - postgres

  tana-ledger:
    build: ./tana-ledger
    ports:
      - "8080:8080"  # REST API
    depends_on:
      - postgres

  tana-contracts:
    build: ./tana-contracts
    ports:
      - "8081:8081"  # Contract API
    depends_on:
      - redis

  # Website (development)
  tana-website:
    build: ./tana-runtime/website
    ports:
      - "4322:4322"
    volumes:
      - ./tana-runtime/website:/app

volumes:
  ledger-data:
  contract-data:
```

---

## 📚 Resources

- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf) - For inspiration
- [Cosmos SDK](https://docs.cosmos.network/) - Modular blockchain framework
- [Substrate](https://substrate.io/) - Blockchain development framework
- [IPFS](https://ipfs.tech/) - Decentralized storage
- [libp2p](https://libp2p.io/) - P2P networking

---

## ✅ Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-11-02 | Split into 5 services (runtime, cli, node, ledger, contracts) | Separation of concerns, easier scaling |
| 2025-11-02 | Multi-currency ledger (not native token) | Flexibility for fiat + crypto |
| 2025-11-02 | Landing pages = static HTML + dynamic islands | Balance between simplicity and interactivity |
| 2025-11-02 | PostgreSQL for ledger, Redis for contracts | Right tool for each job |

---

**Last Updated:** 2025-11-02
**Status:** Ready for development - pending approval of data model and architecture
