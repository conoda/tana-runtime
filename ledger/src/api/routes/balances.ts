/**
 * Balance API Routes
 */

import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import * as balanceService from '../../balances'
import { setBalanceSchema, createCurrencySchema } from '../schemas'

const app = new Hono()

// GET /balances - Get balance for specific owner and currency
app.get('/', async (c) => {
  const ownerId = c.req.query('ownerId')
  const ownerType = c.req.query('ownerType') as 'user' | 'team'
  const currencyCode = c.req.query('currencyCode')

  if (!ownerId || !ownerType || !currencyCode) {
    return c.json({ error: 'Missing required query parameters: ownerId, ownerType, currencyCode' }, 400)
  }

  const balance = await balanceService.getBalance({
    ownerId,
    ownerType,
    currencyCode,
  })

  if (!balance) {
    return c.json({ balance: '0', currencyCode }, 200)
  }

  return c.json(balance)
})

// POST /balances - Set balance
app.post('/', zValidator('json', setBalanceSchema), async (c) => {
  const body = c.req.valid('json')

  try {
    const balance = await balanceService.setBalance(body)
    return c.json(balance, 201)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// GET /currencies - List all currencies
app.get('/currencies', async (c) => {
  const currencies = await balanceService.listCurrencies()
  return c.json(currencies)
})

// POST /currencies - Create currency
app.post('/currencies', zValidator('json', createCurrencySchema), async (c) => {
  const body = c.req.valid('json')

  try {
    const currency = await balanceService.createCurrency(body)
    return c.json(currency, 201)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// POST /currencies/seed - Seed default currencies
app.post('/currencies/seed', async (c) => {
  try {
    await balanceService.seedCurrencies()
    return c.json({ message: 'Default currencies seeded' })
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

export default app
