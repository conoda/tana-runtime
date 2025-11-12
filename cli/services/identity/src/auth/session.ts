/**
 * Auth Session Management
 *
 * Handles creation, validation, and lifecycle of authentication sessions
 * for mobile QR code login
 */

import { randomBytes } from 'crypto'
import { customAlphabet } from 'nanoid'
import { db } from '../db'
import { authSessions } from '../db/schema'
import { eq, and, lt } from 'drizzle-orm'
import * as ed from '@noble/ed25519'

const nanoid = customAlphabet('abcdefghijklmnopqrstuvwxyz0123456789', 16)

// Session expiration time (5 minutes)
const SESSION_EXPIRATION_MS = 5 * 60 * 1000

// Session token expiration (30 days)
const SESSION_TOKEN_EXPIRATION_MS = 30 * 24 * 60 * 60 * 1000

/**
 * Generate a unique session ID
 */
export function generateSessionId(): string {
  return `sess_${nanoid()}`
}

/**
 * Generate a random challenge for signature verification
 */
export function generateChallenge(): string {
  return `auth_chal_${randomBytes(32).toString('hex')}`
}

/**
 * Generate a secure session token
 */
export function generateSessionToken(): string {
  return `tana_session_${randomBytes(32).toString('base64url')}`
}

/**
 * Verify Ed25519 signature
 */
async function verifySignature(
  message: string,
  signature: string,
  publicKey: string
): Promise<boolean> {
  try {
    const messageBytes = new TextEncoder().encode(message)
    const signatureBytes = Buffer.from(signature, 'hex')
    const publicKeyBytes = Buffer.from(publicKey, 'hex')

    return await ed.verify(signatureBytes, messageBytes, publicKeyBytes)
  } catch (error) {
    console.error('Signature verification error:', error)
    return false
  }
}

/**
 * Create a new authentication session
 */
export async function createSession(params: {
  returnUrl: string
  appName?: string
  appIcon?: string
}) {
  const sessionId = generateSessionId()
  const challenge = generateChallenge()
  const expiresAt = new Date(Date.now() + SESSION_EXPIRATION_MS)

  await db.insert(authSessions).values({
    id: sessionId,
    challenge,
    status: 'waiting',
    returnUrl: params.returnUrl,
    appName: params.appName,
    appIcon: params.appIcon,
    expiresAt,
  })

  return {
    sessionId,
    challenge,
    expiresAt: expiresAt.getTime(),
    expiresIn: SESSION_EXPIRATION_MS / 1000, // seconds
  }
}

/**
 * Get session by ID
 */
export async function getSession(sessionId: string) {
  const [session] = await db
    .select()
    .from(authSessions)
    .where(eq(authSessions.id, sessionId))
    .limit(1)

  return session
}

/**
 * Mark session as scanned
 */
export async function markSessionScanned(sessionId: string) {
  await db
    .update(authSessions)
    .set({
      status: 'scanned',
      scannedAt: new Date(),
    })
    .where(eq(authSessions.id, sessionId))
}

/**
 * Approve session with signed challenge
 */
export async function approveSession(params: {
  sessionId: string
  userId: string
  username: string
  publicKey: string
  signature: string
  message: string
}) {
  const session = await getSession(params.sessionId)

  if (!session) {
    throw new Error('Session not found')
  }

  if (session.status !== 'waiting' && session.status !== 'scanned') {
    throw new Error(`Session cannot be approved (status: ${session.status})`)
  }

  if (Date.now() > session.expiresAt.getTime()) {
    await expireSession(params.sessionId)
    throw new Error('Session expired')
  }

  // Verify signature
  const isValid = await verifySignature(
    params.message,
    params.signature,
    params.publicKey
  )

  if (!isValid) {
    throw new Error('Invalid signature')
  }

  // Verify challenge is included in message
  if (!params.message.includes(session.challenge)) {
    throw new Error('Challenge mismatch')
  }

  // Generate session token
  const sessionToken = generateSessionToken()

  // Update session
  await db
    .update(authSessions)
    .set({
      status: 'approved',
      userId: params.userId,
      username: params.username,
      publicKey: params.publicKey,
      sessionToken,
      approvedAt: new Date(),
    })
    .where(eq(authSessions.id, params.sessionId))

  return {
    sessionToken,
    returnUrl: session.returnUrl,
  }
}

/**
 * Reject session
 */
export async function rejectSession(sessionId: string) {
  await db
    .update(authSessions)
    .set({ status: 'rejected' })
    .where(eq(authSessions.id, sessionId))
}

/**
 * Expire session
 */
export async function expireSession(sessionId: string) {
  await db
    .update(authSessions)
    .set({ status: 'expired' })
    .where(eq(authSessions.id, sessionId))
}

/**
 * Validate session token and get user info
 */
export async function validateSessionToken(sessionToken: string) {
  const [session] = await db
    .select()
    .from(authSessions)
    .where(
      and(
        eq(authSessions.sessionToken, sessionToken),
        eq(authSessions.status, 'approved')
      )
    )
    .limit(1)

  if (!session) {
    return null
  }

  // Check if session is expired (30 days after approval)
  if (session.approvedAt) {
    const expirationTime = session.approvedAt.getTime() + SESSION_TOKEN_EXPIRATION_MS
    if (Date.now() > expirationTime) {
      await expireSession(session.id)
      return null
    }
  }

  return {
    sessionId: session.id,
    userId: session.userId,
    username: session.username,
    publicKey: session.publicKey,
  }
}

/**
 * Revoke session (logout)
 */
export async function revokeSession(sessionId: string) {
  await db
    .update(authSessions)
    .set({
      status: 'expired',
      sessionToken: null, // Clear token
    })
    .where(eq(authSessions.id, sessionId))
}

/**
 * Clean up expired sessions (garbage collection)
 */
export async function cleanupExpiredSessions() {
  const now = new Date()

  const result = await db
    .update(authSessions)
    .set({ status: 'expired' })
    .where(
      and(
        eq(authSessions.status, 'waiting'),
        lt(authSessions.expiresAt, now)
      )
    )

  return result
}

/**
 * Get active sessions for a user
 */
export async function getUserSessions(userId: string) {
  const sessions = await db
    .select({
      id: authSessions.id,
      appName: authSessions.appName,
      approvedAt: authSessions.approvedAt,
      scannedAt: authSessions.scannedAt,
    })
    .from(authSessions)
    .where(
      and(
        eq(authSessions.userId, userId),
        eq(authSessions.status, 'approved')
      )
    )
    .orderBy(authSessions.approvedAt)

  return sessions
}
