/**
 * API Validation Schemas (Zod)
 */

import { z } from 'zod'

// ============================================================================
// USER SCHEMAS
// ============================================================================

export const createUserSchema = z.object({
  publicKey: z.string().min(1),
  username: z.string().regex(/^@[a-z0-9_-]+$/i, 'Username must start with @ and contain only alphanumeric characters'),
  displayName: z.string().min(1).max(100),
  bio: z.string().max(1000).optional(),
  avatarData: z.string().optional(),
})

export const updateUserSchema = z.object({
  displayName: z.string().min(1).max(100).optional(),
  bio: z.string().max(1000).optional(),
  avatarData: z.string().optional(),
  landingPageId: z.string().uuid().optional(),
})

// ============================================================================
// TEAM SCHEMAS
// ============================================================================

export const createTeamSchema = z.object({
  name: z.string().min(1).max(100),
  slug: z.string().regex(/^@[a-z0-9_-]+$/i, 'Team slug must start with @ and contain only alphanumeric characters'),
  description: z.string().max(1000).optional(),
  ownerId: z.string().uuid(),
})

export const updateTeamSchema = z.object({
  name: z.string().min(1).max(100).optional(),
  description: z.string().max(1000).optional(),
  avatarData: z.string().optional(),
  landingPageId: z.string().uuid().optional(),
})

export const addTeamMemberSchema = z.object({
  teamId: z.string().uuid(),
  userId: z.string().uuid(),
  role: z.enum(['owner', 'admin', 'member']).optional(),
})

// ============================================================================
// CURRENCY SCHEMAS
// ============================================================================

export const createCurrencySchema = z.object({
  code: z.string().min(1).max(10).toUpperCase(),
  type: z.enum(['fiat', 'crypto']),
  decimals: z.number().int().min(0).max(18),
  name: z.string().max(100).optional(),
  symbol: z.string().max(10).optional(),
  verified: z.boolean().optional(),
})

// ============================================================================
// TRANSACTION SCHEMAS
// ============================================================================

export const createTransactionSchema = z.object({
  from: z.string().uuid(),
  to: z.string().uuid(),
  amount: z.string().regex(/^\d+(\.\d+)?$/, 'Amount must be a valid decimal number'),
  currencyCode: z.string().min(1).max(10).toUpperCase(),
  type: z.enum(['transfer', 'deposit', 'withdraw', 'contract_call']),
  signature: z.string().min(1),
  contractId: z.string().uuid().optional(),
  contractInput: z.record(z.any()).optional(),
})

export const confirmTransactionSchema = z.object({
  id: z.string().uuid(),
  blockId: z.string().uuid().optional(),
})

// ============================================================================
// BALANCE SCHEMAS
// ============================================================================

export const setBalanceSchema = z.object({
  ownerId: z.string().uuid(),
  ownerType: z.enum(['user', 'team']),
  currencyCode: z.string().min(1).max(10).toUpperCase(),
  amount: z.string().regex(/^\d+(\.\d+)?$/, 'Amount must be a valid decimal number'),
})
