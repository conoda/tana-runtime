/**
 * User API Routes
 */

import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import * as userService from '../../accounts/users'
import * as balanceService from '../../balances'
import { createUserSchema, updateUserSchema } from '../schemas'

const app = new Hono()

// GET /users - List all users
app.get('/', async (c) => {
  const limit = parseInt(c.req.query('limit') || '50')
  const offset = parseInt(c.req.query('offset') || '0')

  const users = await userService.listUsers(limit, offset)
  return c.json(users)
})

// POST /users - Create new user
app.post('/', zValidator('json', createUserSchema), async (c) => {
  const body = c.req.valid('json')

  try {
    const user = await userService.createUser(body)
    return c.json(user, 201)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// GET /users/:id - Get user by ID
app.get('/:id', async (c) => {
  const { id } = c.req.param()
  const user = await userService.getUserById(id)

  if (!user) {
    return c.json({ error: 'User not found' }, 404)
  }

  return c.json(user)
})

// GET /users/username/:username - Get user by username
app.get('/username/:username', async (c) => {
  const { username } = c.req.param()
  const user = await userService.getUserByUsername(username)

  if (!user) {
    return c.json({ error: 'User not found' }, 404)
  }

  return c.json(user)
})

// PATCH /users/:id - Update user
app.patch('/:id', zValidator('json', updateUserSchema), async (c) => {
  const { id } = c.req.param()
  const body = c.req.valid('json')

  try {
    const user = await userService.updateUser(id, body)

    if (!user) {
      return c.json({ error: 'User not found' }, 404)
    }

    return c.json(user)
  } catch (error: any) {
    return c.json({ error: error.message }, 400)
  }
})

// DELETE /users/:id - Delete user
app.delete('/:id', async (c) => {
  const { id } = c.req.param()
  const user = await userService.deleteUser(id)

  if (!user) {
    return c.json({ error: 'User not found' }, 404)
  }

  return c.json({ message: 'User deleted', user })
})

// GET /users/:id/balances - Get user balances
app.get('/:id/balances', async (c) => {
  const { id } = c.req.param()

  // Verify user exists
  const user = await userService.getUserById(id)
  if (!user) {
    return c.json({ error: 'User not found' }, 404)
  }

  const balances = await balanceService.getBalances(id, 'user')
  return c.json(balances)
})

export default app
