/**
 * Tana Identity Service
 *
 * Handles user authentication, QR code login sessions, and mobile device management
 * Separate from blockchain data (ledger service)
 */

import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { logger } from 'hono/logger'
import authRoutes from './api/routes/auth'
import { cleanupExpiredSessions } from './auth/session'

const app = new Hono()

// ============================================================================
// MIDDLEWARE
// ============================================================================

// CORS - Allow requests from web apps
app.use('/*', cors({
  origin: [
    'http://localhost:3000',
    'http://localhost:4321',
    'http://localhost:5173',
  ],
  credentials: true,
}))

// Request logging
app.use('*', logger())

// ============================================================================
// ROUTES
// ============================================================================

// Health check
app.get('/health', (c) => {
  return c.json({
    service: 'tana-identity',
    status: 'healthy',
    timestamp: new Date().toISOString(),
  })
})

// Auth routes
app.route('/auth', authRoutes)

// ============================================================================
// SESSION CLEANUP
// ============================================================================

// Clean up expired sessions every minute
setInterval(async () => {
  try {
    await cleanupExpiredSessions()
  } catch (error) {
    console.error('Failed to cleanup expired sessions:', error)
  }
}, 60 * 1000)

// ============================================================================
// SERVER
// ============================================================================

const PORT = parseInt(process.env.IDENTITY_PORT || '8090')

console.log(`🔐 Tana Identity Service`)
console.log(`📡 Starting server on port ${PORT}...`)

export default {
  port: PORT,
  fetch: app.fetch,
}
