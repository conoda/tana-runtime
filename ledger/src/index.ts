/**
 * Tana Ledger Service
 *
 * Main entry point for the ledger API server
 */

import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { logger } from 'hono/logger'
import { prettyJSON } from 'hono/pretty-json'

// Import routes
import usersRoutes from './api/routes/users'
import balancesRoutes from './api/routes/balances'
import transactionsRoutes from './api/routes/transactions'

const app = new Hono()

// ============================================================================
// MIDDLEWARE
// ============================================================================

app.use('*', logger())
app.use('*', prettyJSON())
app.use('*', cors({
  origin: process.env.ALLOWED_ORIGINS?.split(',') || ['*'],
}))

// ============================================================================
// HEALTH CHECK
// ============================================================================

app.get('/', (c) => {
  return c.json({
    service: 'tana-ledger',
    version: '0.1.0',
    status: 'healthy',
    timestamp: new Date().toISOString(),
  })
})

app.get('/health', (c) => {
  return c.json({ status: 'ok' })
})

// ============================================================================
// API ROUTES
// ============================================================================

app.route('/users', usersRoutes)
app.route('/balances', balancesRoutes)
app.route('/transactions', transactionsRoutes)

// ============================================================================
// ERROR HANDLING
// ============================================================================

app.onError((err, c) => {
  console.error(`Error: ${err.message}`)
  console.error(err.stack)

  return c.json({
    error: err.message || 'Internal Server Error',
    timestamp: new Date().toISOString(),
  }, 500)
})

// ============================================================================
// START SERVER
// ============================================================================

const port = parseInt(process.env.PORT || '8080')

console.log(`🚀 Tana Ledger Service starting on port ${port}`)
console.log(`📊 Database: ${process.env.DATABASE_URL ? 'Connected' : 'Using default connection'}`)

export default {
  port,
  fetch: app.fetch,
}
