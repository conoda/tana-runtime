/**
 * Authentication Routes
 *
 * Handles QR code login sessions and mobile approval
 */

import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import {
  createSession,
  getSession,
  markSessionScanned,
  approveSession,
  rejectSession,
} from '../../auth/session'
import { streamSSE } from 'hono/streaming'

const auth = new Hono()

// ============================================================================
// CREATE SESSION (Website initiates QR code login)
// ============================================================================

const createSessionSchema = z.object({
  returnUrl: z.string().url(),
  appName: z.string().optional(),
  appIcon: z.string().url().optional(),
})

auth.post(
  '/session/create',
  zValidator('json', createSessionSchema),
  async (c) => {
    const body = c.req.valid('json')

    try {
      const session = await createSession({
        returnUrl: body.returnUrl,
        appName: body.appName,
        appIcon: body.appIcon,
      })

      return c.json({
        sessionId: session.sessionId,
        challenge: session.challenge,
        expiresAt: session.expiresAt,
        expiresIn: session.expiresIn,
        qrData: JSON.stringify({
          sessionId: session.sessionId,
          challenge: session.challenge,
          service: 'tana-identity',
        }),
      }, 201)
    } catch (error: any) {
      console.error('Failed to create session:', error)
      return c.json({ error: 'Failed to create session' }, 500)
    }
  }
)

// ============================================================================
// SESSION EVENTS (SSE stream for real-time updates)
// ============================================================================

auth.get('/session/:id/events', async (c) => {
  const sessionId = c.req.param('id')

  return streamSSE(c, async (stream) => {
    let isOpen = true

    // Clean up when connection closes
    c.req.raw.signal.addEventListener('abort', () => {
      isOpen = false
    })

    // Send initial connection event
    await stream.writeSSE({
      data: JSON.stringify({ type: 'connected', sessionId }),
      event: 'connected',
    })

    // Poll session status every 500ms
    while (isOpen) {
      try {
        const session = await getSession(sessionId)

        if (!session) {
          await stream.writeSSE({
            data: JSON.stringify({ type: 'error', error: 'Session not found' }),
            event: 'error',
          })
          break
        }

        // Send status update
        await stream.writeSSE({
          data: JSON.stringify({
            type: 'status',
            status: session.status,
            scannedAt: session.scannedAt,
            approvedAt: session.approvedAt,
          }),
          event: 'status',
        })

        // If session reached a terminal state, send result and close
        if (session.status === 'approved') {
          await stream.writeSSE({
            data: JSON.stringify({
              type: 'approved',
              sessionToken: session.sessionToken,
              userId: session.userId,
              username: session.username,
              returnUrl: session.returnUrl,
            }),
            event: 'approved',
          })
          break
        }

        if (session.status === 'rejected') {
          await stream.writeSSE({
            data: JSON.stringify({ type: 'rejected' }),
            event: 'rejected',
          })
          break
        }

        if (session.status === 'expired') {
          await stream.writeSSE({
            data: JSON.stringify({ type: 'expired' }),
            event: 'expired',
          })
          break
        }

        // Wait before next poll
        await new Promise(resolve => setTimeout(resolve, 500))
      } catch (error) {
        console.error('SSE stream error:', error)
        await stream.writeSSE({
          data: JSON.stringify({ type: 'error', error: 'Stream error' }),
          event: 'error',
        })
        break
      }
    }
  })
})

// ============================================================================
// GET SESSION (Mobile app scans QR code)
// ============================================================================

auth.get('/session/:id', async (c) => {
  const sessionId = c.req.param('id')

  try {
    const session = await getSession(sessionId)

    if (!session) {
      return c.json({ error: 'Session not found' }, 404)
    }

    // Mark as scanned if still waiting
    if (session.status === 'waiting') {
      await markSessionScanned(sessionId)
    }

    return c.json({
      sessionId: session.id,
      challenge: session.challenge,
      status: session.status,
      appName: session.appName,
      appIcon: session.appIcon,
      expiresAt: session.expiresAt.getTime(),
    })
  } catch (error: any) {
    console.error('Failed to get session:', error)
    return c.json({ error: 'Failed to get session' }, 500)
  }
})

// ============================================================================
// APPROVE SESSION (Mobile app approves login)
// ============================================================================

const approveSessionSchema = z.object({
  userId: z.string().uuid(),
  username: z.string(),
  publicKey: z.string(),
  signature: z.string(),
  message: z.string(),
})

auth.post(
  '/session/:id/approve',
  zValidator('json', approveSessionSchema),
  async (c) => {
    const sessionId = c.req.param('id')
    const body = c.req.valid('json')

    try {
      const result = await approveSession({
        sessionId,
        userId: body.userId,
        username: body.username,
        publicKey: body.publicKey,
        signature: body.signature,
        message: body.message,
      })

      return c.json({
        success: true,
        sessionToken: result.sessionToken,
        returnUrl: result.returnUrl,
      })
    } catch (error: any) {
      console.error('Failed to approve session:', error)
      return c.json({ error: error.message || 'Failed to approve session' }, 400)
    }
  }
)

// ============================================================================
// REJECT SESSION (Mobile app rejects login)
// ============================================================================

auth.post('/session/:id/reject', async (c) => {
  const sessionId = c.req.param('id')

  try {
    await rejectSession(sessionId)
    return c.json({ success: true })
  } catch (error: any) {
    console.error('Failed to reject session:', error)
    return c.json({ error: 'Failed to reject session' }, 500)
  }
})

// ============================================================================
// VALIDATE SESSION TOKEN (For subsequent authenticated requests)
// ============================================================================

const validateTokenSchema = z.object({
  sessionToken: z.string(),
})

auth.post(
  '/session/validate',
  zValidator('json', validateTokenSchema),
  async (c) => {
    const body = c.req.valid('json')

    try {
      const { validateSessionToken } = await import('../../auth/session')
      const session = await validateSessionToken(body.sessionToken)

      if (!session) {
        return c.json({ error: 'Invalid or expired session token' }, 401)
      }

      return c.json({
        valid: true,
        userId: session.userId,
        username: session.username,
      })
    } catch (error: any) {
      console.error('Failed to validate session:', error)
      return c.json({ error: 'Failed to validate session' }, 500)
    }
  }
)

export default auth
