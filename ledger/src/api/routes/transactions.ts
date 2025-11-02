/**
 * Transaction API Routes
 */

import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import * as transactionService from '../../transactions'
import { createTransactionSchema, confirmTransactionSchema } from '../schemas'

const app = new Hono()

// GET /transactions - Get all transactions
app.get('/', async (c) => {
  const limit = parseInt(c.req.query('limit') || '100')
  const offset = parseInt(c.req.query('offset') || '0')

  const transactions = await transactionService.getAllTransactions(limit, offset)
  return c.json(transactions)
})

// POST /transactions - Create new transaction
app.post('/', zValidator('json', createTransactionSchema), async (c) => {
  const body = c.req.valid('json')

  try {
    // Validate transaction
    const validation = await transactionService.validateTransaction(body)
    if (!validation.valid) {
      return c.json({ error: validation.error }, 400)
    }

    const transaction = await transactionService.createTransaction(body)
    return c.json(transaction, 201)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// GET /transactions/:id - Get transaction
app.get('/:id', async (c) => {
  const { id } = c.req.param()
  const transaction = await transactionService.getTransaction(id)

  if (!transaction) {
    return c.json({ error: 'Transaction not found' }, 404)
  }

  return c.json(transaction)
})

// POST /transactions/:id/confirm - Confirm transaction
app.post('/:id/confirm', zValidator('json', confirmTransactionSchema), async (c) => {
  const body = c.req.valid('json')

  try {
    const transaction = await transactionService.confirmTransaction(body)
    return c.json(transaction)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// GET /transactions/pending - Get pending transactions
app.get('/pending', async (c) => {
  const limit = parseInt(c.req.query('limit') || '100')
  const transactions = await transactionService.getPendingTransactions(limit)
  return c.json(transactions)
})

// GET /transactions/account/:accountId - Get account transactions
app.get('/account/:accountId', async (c) => {
  const { accountId } = c.req.param()
  const limit = parseInt(c.req.query('limit') || '50')
  const offset = parseInt(c.req.query('offset') || '0')

  const transactions = await transactionService.getAccountTransactions(accountId, limit, offset)
  return c.json(transactions)
})

export default app
