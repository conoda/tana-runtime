/**
 * User Account Service
 *
 * CRUD operations for user accounts
 */

import { eq } from 'drizzle-orm'
import { db, users } from '../db'
import { createHash } from 'crypto'

export interface CreateUserInput {
  publicKey: string
  username: string
  displayName: string
  bio?: string
  avatarData?: string
}

export interface UpdateUserInput {
  displayName?: string
  bio?: string
  avatarData?: string
  landingPageId?: string
}

/**
 * Create a new user account
 */
export async function createUser(input: CreateUserInput) {
  // Validate username format (@alice)
  if (!input.username.startsWith('@')) {
    throw new Error('Username must start with @')
  }

  // Calculate initial state hash
  const stateHash = calculateStateHash({
    publicKey: input.publicKey,
    username: input.username,
  })

  const [user] = await db
    .insert(users)
    .values({
      publicKey: input.publicKey,
      username: input.username,
      displayName: input.displayName,
      bio: input.bio,
      avatarData: input.avatarData,
      stateHash,
    })
    .returning()

  return user
}

/**
 * Get user by ID
 */
export async function getUserById(id: string) {
  const [user] = await db.select().from(users).where(eq(users.id, id)).limit(1)
  return user || null
}

/**
 * Get user by username
 */
export async function getUserByUsername(username: string) {
  const [user] = await db.select().from(users).where(eq(users.username, username)).limit(1)
  return user || null
}

/**
 * Get user by public key
 */
export async function getUserByPublicKey(publicKey: string) {
  const [user] = await db.select().from(users).where(eq(users.publicKey, publicKey)).limit(1)
  return user || null
}

/**
 * Update user
 */
export async function updateUser(id: string, input: UpdateUserInput) {
  const [updated] = await db
    .update(users)
    .set({
      ...input,
      updatedAt: new Date(),
    })
    .where(eq(users.id, id))
    .returning()

  return updated || null
}

/**
 * Delete user
 */
export async function deleteUser(id: string) {
  const [deleted] = await db.delete(users).where(eq(users.id, id)).returning()
  return deleted || null
}

/**
 * List all users (paginated)
 */
export async function listUsers(limit = 50, offset = 0) {
  return await db.select().from(users).limit(limit).offset(offset)
}

/**
 * Calculate state hash for user
 * This is a simplified version - in production would include all account state
 */
function calculateStateHash(data: Record<string, any>): string {
  const hash = createHash('sha256')
  hash.update(JSON.stringify(data))
  return hash.digest('hex')
}
