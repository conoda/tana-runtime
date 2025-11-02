# Tana CLI

Command-line interface for interacting with the Tana blockchain.

## Installation

```bash
# From source
cd cli
bun install
bun link

# Now you can use `tana` command globally
tana --help
```

## Commands

```bash
# Account management
tana account create              # Create new account
tana account balance @alice      # Check balance
tana account info @alice         # Get account details

# Transactions
tana send @bob 10 USD            # Send money
tana send --from @alice @bob 5 BTC

# Smart contracts
tana deploy contract.ts          # Deploy contract
tana call @contract/counter increment  # Call contract function
tana inspect @contract/counter   # View contract code & state

# Keys
tana keys generate               # Generate new keypair
tana keys list                   # List all keys
tana keys export @alice          # Export private key

# Node operations
tana node start                  # Start local node
tana node status                 # Check node status
tana node sync                   # Sync with network

# Development
tana dev init                    # Initialize new project
tana dev playground              # Open browser playground
```

## Configuration

```bash
# Default config location: ~/.tana/config.json
{
  "network": "mainnet",
  "node": "https://node.tana.dev",
  "ledger": "https://ledger.tana.dev",
  "contracts": "https://contracts.tana.dev"
}
```

## Development

```bash
# Install dependencies
bun install

# Run CLI in development
bun run dev -- account balance @alice

# Build for distribution
bun run build

# Run tests
bun test
```

## Architecture

```
src/
├── index.ts              # Main CLI entry point
├── commands/             # Command implementations
│   ├── account.ts
│   ├── send.ts
│   ├── deploy.ts
│   ├── keys.ts
│   └── node.ts
├── lib/                  # Shared utilities
│   ├── api.ts           # API client
│   ├── crypto.ts        # Key management
│   └── config.ts        # Configuration
└── utils/               # Helpers
    ├── logger.ts
    └── prompts.ts
```
