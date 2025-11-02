# Tana Contracts

Smart contract deployment and execution service.

## Features

- **Contract Deployment**: Store and version TypeScript smart contracts
- **Sandboxed Execution**: Execute contracts via tana-runtime (Rust/V8)
- **State Management**: Redis-backed contract state storage
- **Gas Metering**: Track and limit execution resources
- **API**: HTTP API for contract deployment and calls

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun run dev

# Build for production
bun run build

# Run tests
bun test
```

## Architecture

```
src/
├── index.ts          # Main API server (Hono)
├── executor/         # Contract execution via tana-runtime
├── storage/          # Contract state (Redis)
└── api/              # REST API routes
```

## How It Works

1. **Deploy**: Store contract code on-chain (PostgreSQL)
2. **Execute**: Call tana-runtime (Rust/V8) to execute TypeScript
3. **State**: Read/write contract state via Redis
4. **Return**: Send results back to caller

## API Endpoints

```
POST /contracts              # Deploy contract
GET  /contracts/:id          # Get contract code
POST /contracts/:id/call     # Execute contract function
GET  /contracts/:id/state    # Get contract state
```

## Environment Variables

```env
PORT=8081
REDIS_URL=redis://localhost:6379
DATABASE_URL=postgres://...
RUNTIME_PATH=../runtime/target/release/tana-runtime
NODE_ENV=development
```

## Integration with tana-runtime

This service calls the Rust runtime as a subprocess:

```typescript
import { spawn } from 'bun'

const runtime = spawn(['../runtime/target/release/tana-runtime', contractCode])
const result = await runtime.text()
```

Alternatively, we could compile tana-runtime as a library and use FFI.
