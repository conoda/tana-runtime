/**
 * Database connection for Identity Service
 */

import postgres from 'postgres'
import { drizzle } from 'drizzle-orm/postgres-js'
import * as schema from './schema'

// Database URL from environment
const databaseUrl = process.env.IDENTITY_DB_URL || process.env.DATABASE_URL

if (!databaseUrl) {
  throw new Error('IDENTITY_DB_URL or DATABASE_URL must be set')
}

// Create postgres connection
const client = postgres(databaseUrl)

// Create drizzle instance
export const db = drizzle(client, { schema })
