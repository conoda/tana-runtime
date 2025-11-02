# Tana Node

Blockchain node service for the Tana network.

## Features

- **P2P Networking**: libp2p-based peer discovery and communication
- **Consensus**: Block validation and consensus mechanism
- **Storage**: PostgreSQL-backed block and transaction storage
- **JSON-RPC API**: Standard blockchain node API

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
├── index.ts          # Main entry point
├── p2p/              # P2P networking layer
├── consensus/        # Block validation & consensus
├── storage/          # Database layer (PostgreSQL)
└── api/              # JSON-RPC API server
```

## Environment Variables

```env
PORT=9933              # JSON-RPC port
P2P_PORT=30333         # P2P networking port
DATABASE_URL=postgres://...
NODE_ENV=development
```
