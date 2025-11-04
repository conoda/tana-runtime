// Batch Query Example
// Demonstrates querying multiple users/balances at once (max 10)

import { console } from 'tana:core'
import { block } from 'tana:block'

console.log("=== Batch Query Example ===\n")

console.log(`Max batch query size: ${block.MAX_BATCH_QUERY}`)

// Query multiple users at once
const usernames = ['alice', 'bob', 'charlie', 'dave', 'eve']
console.log(`\nQuerying ${usernames.length} users...`)

const users = await block.getUser(usernames)
console.log(`Found ${users.filter(u => u !== null).length} users`)

// Display results
users.forEach((user, i) => {
  if (user) {
    console.log(`  ${i + 1}. ${user.username} - ${user.displayName}`)
  } else {
    console.log(`  ${i + 1}. ${usernames[i]} - NOT FOUND`)
  }
})

// Query balances for found users
const validUsers = users.filter(u => u !== null)
if (validUsers.length > 0) {
  console.log(`\nQuerying USD balances for ${validUsers.length} users...`)

  const userIds = validUsers.map(u => u.id)
  const balances = await block.getBalance(userIds, 'USD')

  validUsers.forEach((user, i) => {
    console.log(`  ${user.username}: ${balances[i]} USD`)
  })
}

// Demonstrate limit enforcement
console.log("\n--- Testing Query Limit ---")
try {
  const tooMany = Array.from({ length: 11 }, (_, i) => `user_${i}`)
  console.log(`Attempting to query ${tooMany.length} users (exceeds limit)...`)
  await block.getUser(tooMany)
  console.log("✗ This should not happen!")
} catch (error) {
  console.log(`✓ Correctly rejected: ${error.message}`)
}
