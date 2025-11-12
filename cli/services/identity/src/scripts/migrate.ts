/**
 * Run database migrations for identity service
 */

import postgres from 'postgres'
import { readFileSync } from 'fs'
import { join } from 'path'

const databaseUrl = process.env.IDENTITY_DB_URL || process.env.DATABASE_URL

if (!databaseUrl) {
  console.error('Error: IDENTITY_DB_URL or DATABASE_URL must be set')
  process.exit(1)
}

const sql = postgres(databaseUrl)

async function migrate() {
  try {
    console.log('🔄 Running identity service migrations...')

    // Read migration file
    const migrationPath = join(import.meta.dir, '../../migrations/0001_initial.sql')
    const migrationSql = readFileSync(migrationPath, 'utf-8')

    // Run migration
    await sql.unsafe(migrationSql)

    console.log('✅ Migrations completed successfully')
  } catch (error) {
    console.error('❌ Migration failed:', error)
    process.exit(1)
  } finally {
    await sql.end()
  }
}

migrate()
