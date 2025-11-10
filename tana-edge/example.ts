// welcome to tana playground!
//
// visit tana on the web @ https://tana.network
//
// copyright (c) 2025 sami fouad http://samifou.ad
//
import { console, version } from 'tana/core'
import { block } from 'tana/block'
import { tx } from 'tana/tx'
import { data } from 'tana/data'

console.log("hello. this is the tana playground.")
console.log("tana's core is a blockchain written in rust.")
console.log("state changes are done with smart contracts written in typescript.")
console.log("version:", version)

// 1. BLOCK CONTEXT
// every contract execution has access to the current block
console.log("\n--- Block Context ---")
console.log(`Block Height: ${block.height}`)
console.log(`Timestamp: ${new Date(block.timestamp).toISOString()}`)
console.log(`Executor: ${block.executor}`)
console.log(`Gas Limit: ${block.gasLimit.toLocaleString()}`)

// 2. QUERY BLOCKCHAIN STATE
// read current state as of this block (max 10 items per query)
console.log("\n--- Querying Blockchain State ---")

// query a single user
const alice = await block.getUser('alice')
if (alice) {
  console.log(`User: ${alice.username} (${alice.displayName})`)

  // get alice's USD balance
  const balance = await block.getBalance(alice.id, 'USD')
  console.log(`Balance: ${balance} USD`)
}

// batch query multiple users (max 10)
const usernames = ['alice', 'bob', 'charlie']
const batchUsers = await block.getUser(usernames)
const validUsers = batchUsers.filter(u => u !== null)
console.log(`Found ${validUsers.length} users from batch query`)

// 3. TRANSACTION EXECUTION
// propose state changes based on current blockchain state
console.log("\n--- Transaction Execution ---")

// propose a transfer from executor to treasury
tx.transfer(block.executor, 'treasury', 5, 'USD')
console.log("Proposed transfer: 5 USD to treasury")

// store transaction in contract state
await data.set('lastTransfer', {
  from: block.executor,
  to: 'treasury',
  amount: 5,
  timestamp: block.timestamp,
  blockHeight: block.height
})
await data.commit()

// check pending changes before execution
const pendingChanges = tx.getChanges()
console.log(`Pending changes: ${pendingChanges.length}`)

// execute the transaction (validate and commit)
const result = await tx.execute()

if (result.success) {
  console.log("✓ Transaction executed successfully!")
  console.log(`Gas used: ${result.gasUsed.toLocaleString()}`)
  console.log("Changes:", result.changes)
} else {
  console.log("✗ Transaction failed:", result.error)
}

console.log("\nevery run of your contract yields the same data.")
console.log("this is deterministic execution on the blockchain.")
