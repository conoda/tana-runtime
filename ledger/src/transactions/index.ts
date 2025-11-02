/**
 * Transaction Service
 *
 * Create and manage blockchain transactions
 */

import { eq } from 'drizzle-orm'
import { db, transactions } from '../db'
import { transferBalance, getBalance } from '../balances'

export interface CreateTransactionInput {
  from: string // User or Team ID
  to: string // User or Team ID
  amount: string
  currencyCode: string
  type: 'transfer' | 'deposit' | 'withdraw' | 'contract_call'
  signature: string
  contractId?: string
  contractInput?: Record<string, any>
}

export interface ConfirmTransactionInput {
  id: string
  blockId?: string
}

/**
 * Create a new transaction
 */
export async function createTransaction(input: CreateTransactionInput) {
  const [transaction] = await db
    .insert(transactions)
    .values({
      from: input.from,
      to: input.to,
      amount: input.amount,
      currencyCode: input.currencyCode.toUpperCase(),
      type: input.type,
      signature: input.signature,
      contractId: input.contractId,
      contractInput: input.contractInput as any,
      status: 'pending',
    })
    .returning()

  return transaction
}

/**
 * Get transaction by ID
 */
export async function getTransaction(id: string) {
  const [transaction] = await db.select().from(transactions).where(eq(transactions.id, id)).limit(1)
  return transaction || null
}

/**
 * Get transactions for an account
 */
export async function getAccountTransactions(accountId: string, limit = 50, offset = 0) {
  // This is simplified - would need OR condition for from/to
  return await db.select().from(transactions).where(eq(transactions.from, accountId)).limit(limit).offset(offset)
}

/**
 * Confirm a transaction (execute the transfer)
 */
export async function confirmTransaction(input: ConfirmTransactionInput) {
  const transaction = await getTransaction(input.id)
  if (!transaction) {
    throw new Error('Transaction not found')
  }

  if (transaction.status !== 'pending') {
    throw new Error(`Transaction already ${transaction.status}`)
  }

  try {
    // TODO: Verify signature

    // Execute the transfer based on type
    if (transaction.type === 'transfer') {
      await transferBalance(
        { ownerId: transaction.from, ownerType: 'user' }, // TODO: Handle teams
        { ownerId: transaction.to, ownerType: 'user' },
        transaction.currencyCode,
        transaction.amount
      )
    } else if (transaction.type === 'deposit') {
      // TODO: Implement deposit logic
      throw new Error('Deposit not yet implemented')
    } else if (transaction.type === 'withdraw') {
      // TODO: Implement withdraw logic
      throw new Error('Withdraw not yet implemented')
    } else if (transaction.type === 'contract_call') {
      // TODO: Implement contract call logic
      throw new Error('Contract call not yet implemented')
    }

    // Update transaction status
    const [confirmed] = await db
      .update(transactions)
      .set({
        status: 'confirmed',
        blockId: input.blockId,
        confirmedAt: new Date(),
      })
      .where(eq(transactions.id, input.id))
      .returning()

    return confirmed
  } catch (error) {
    // Mark as failed
    await db
      .update(transactions)
      .set({
        status: 'failed',
      })
      .where(eq(transactions.id, input.id))

    throw error
  }
}

/**
 * Validate a transaction before submission
 */
export async function validateTransaction(input: CreateTransactionInput): Promise<{ valid: boolean; error?: string }> {
  // Check if sender has sufficient balance
  const balance = await getBalance({
    ownerId: input.from,
    ownerType: 'user', // TODO: Handle teams
    currencyCode: input.currencyCode,
  })

  if (!balance) {
    return { valid: false, error: 'Sender has no balance for this currency' }
  }

  const currentBalance = parseFloat(balance.amount)
  const transferAmount = parseFloat(input.amount)

  if (currentBalance < transferAmount) {
    return { valid: false, error: `Insufficient balance: ${currentBalance} ${input.currencyCode}` }
  }

  // TODO: Verify signature
  // TODO: Check if accounts exist
  // TODO: Validate transaction type specific rules

  return { valid: true }
}

/**
 * Get all transactions
 */
export async function getAllTransactions(limit = 100, offset = 0) {
  return await db.select().from(transactions).limit(limit).offset(offset)
}

/**
 * List all pending transactions
 */
export async function getPendingTransactions(limit = 100) {
  return await db.select().from(transactions).where(eq(transactions.status, 'pending')).limit(limit)
}
