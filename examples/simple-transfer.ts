// Simple Transfer Contract
// Demonstrates querying blockchain state and proposing a transfer

import { console } from 'tana:core'
import { block } from 'tana:block'
import { tx } from 'tana:tx'

console.log("=== Simple Transfer Contract ===\n")

// Query current balance
const currentBalance = await block.getBalance(block.executor, 'USD')
console.log(`Current balance: ${currentBalance} USD`)

// Only transfer if sufficient funds
if (currentBalance >= 10) {
  console.log("Sufficient funds available")

  // Propose transfer
  tx.transfer(block.executor, 'treasury', 10, 'USD')
  console.log("Proposed: Transfer 10 USD to treasury")

  // Execute transaction
  const result = await tx.execute()

  if (result.success) {
    console.log("\n✓ Transfer successful!")
  } else {
    console.log(`\n✗ Transfer failed: ${result.error}`)
  }
} else {
  console.log("✗ Insufficient funds")
  console.log(`Need: 10 USD, Have: ${currentBalance} USD`)
}
