/**
 * Tana Identity Service Database Schema
 *
 * Handles user authentication, sessions, and device management
 * Separate from blockchain data (ledger service)
 */

import { pgTable, text, timestamp, pgEnum } from 'drizzle-orm/pg-core'

// ============================================================================
// ENUMS
// ============================================================================

export const authSessionStatusEnum = pgEnum('auth_session_status', [
  'waiting',
  'scanned',
  'approved',
  'rejected',
  'expired'
])

export const transactionRequestStatusEnum = pgEnum('transaction_request_status', [
  'pending',
  'approved',
  'rejected',
  'expired'
])

// ============================================================================
// AUTH SESSIONS (QR Code Login)
// ============================================================================

export const authSessions = pgTable('auth_sessions', {
  id: text('id').primaryKey(), // sess_abc123
  challenge: text('challenge').notNull().unique(), // Random challenge to be signed
  status: authSessionStatusEnum('status').notNull().default('waiting'),

  // User info from blockchain (set after approval)
  userId: text('user_id'), // UUID from blockchain
  username: text('username'),
  publicKey: text('public_key'),

  // Session token (generated after approval)
  sessionToken: text('session_token').unique(),

  // App info
  returnUrl: text('return_url').notNull(),
  appName: text('app_name'),
  appIcon: text('app_icon'),

  // Timestamps
  createdAt: timestamp('created_at').notNull().defaultNow(),
  expiresAt: timestamp('expires_at').notNull(),
  approvedAt: timestamp('approved_at'),
  scannedAt: timestamp('scanned_at'),
})

// ============================================================================
// TRANSACTION REQUESTS (Mobile Transaction Approval)
// ============================================================================

export const transactionRequests = pgTable('transaction_requests', {
  id: text('id').primaryKey(), // txreq_abc123
  sessionId: text('session_id').notNull().references(() => authSessions.id),

  // User from blockchain
  userId: text('user_id').notNull(),

  // Transaction details
  transactionType: text('transaction_type').notNull(), // transfer, contract_call, etc.
  transactionData: text('transaction_data').notNull(), // JSON string of tx data

  // Status
  status: transactionRequestStatusEnum('status').notNull().default('pending'),

  // Result (after approval)
  transactionId: text('transaction_id'), // Actual blockchain tx ID

  // Timestamps
  createdAt: timestamp('created_at').notNull().defaultNow(),
  expiresAt: timestamp('expires_at').notNull(),
  respondedAt: timestamp('responded_at'),
})

// ============================================================================
// DEVICE TOKENS (Push Notifications)
// ============================================================================

export const deviceTokens = pgTable('device_tokens', {
  id: text('id').primaryKey(),
  userId: text('user_id').notNull(), // Blockchain user ID

  // Push notification token
  pushToken: text('push_token').notNull().unique(),

  // Device info
  deviceName: text('device_name'),
  platform: text('platform'), // ios, android
  appVersion: text('app_version'),

  // Active status
  isActive: text('is_active').notNull().default('true'),

  // Timestamps
  createdAt: timestamp('created_at').notNull().defaultNow(),
  lastUsedAt: timestamp('last_used_at').notNull().defaultNow(),
})
