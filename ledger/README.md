# Tana Ledger

Account and balance management service for the Tana blockchain.

## Features

- **User Accounts**: Create and manage user accounts
- **Team Management**: Team creation, membership, and treasury
- **Multi-Currency Balances**: Track balances across multiple currencies
- **Transaction Processing**: Validate and process transactions
- **REST API**: HTTP API for account operations

## Development

```bash
# Install dependencies
bun install

# Run database migrations
bun run db:migrate

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
├── accounts/         # User/Team account management
├── balances/         # Multi-currency balance tracking
├── transactions/     # Transaction validation & processing
└── api/              # REST API routes
```

## API Endpoints

```
GET  /accounts/:id              # Get account details
POST /accounts                  # Create account
GET  /accounts/:id/balances     # Get balances
POST /transactions              # Submit transaction
GET  /transactions/:id          # Get transaction details
```

## Environment Variables

```env
PORT=8080
DATABASE_URL=postgres://...
NODE_ENV=development
```
