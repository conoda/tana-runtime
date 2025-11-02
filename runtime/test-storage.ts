// Test tana:data storage API
import { console } from 'tana:core'
import { data } from 'tana:data'

console.log("=== Tana Storage API Test ===\n")

// Test 1: String storage
console.log("Test 1: String values")
await data.set('username', 'alice')
await data.set('status', 'active')
console.log("✓ Staged string values\n")

// Test 2: Object storage (JSON)
console.log("Test 2: Object values")
await data.set('user', { name: 'Bob', balance: 1000 })
await data.set('config', { theme: 'dark', notifications: true })
console.log("✓ Staged object values\n")

// Test 3: Read before commit (should see staged values)
console.log("Test 3: Read staged values")
const username = await data.get('username')
const user = await data.get('user')
console.log('username:', username)
console.log('user:', user)
console.log("")

// Test 4: Commit changes
console.log("Test 4: Commit to storage")
try {
  await data.commit()
  console.log("✓ Committed successfully\n")
} catch (error) {
  console.error("✗ Commit failed:", (error as Error).message)
}

// Test 5: Read after commit
console.log("Test 5: Read committed values")
const committedUser = await data.get('user')
console.log('user after commit:', committedUser)
console.log("")

// Test 6: List keys
console.log("Test 6: List all keys")
const allKeys = await data.keys()
console.log('All keys:', allKeys)
console.log("")

// Test 7: Pattern matching
console.log("Test 7: Pattern matching")
await data.set('user:1:name', 'Alice')
await data.set('user:2:name', 'Bob')
await data.set('user:1:balance', '500')
await data.commit()

const userKeys = await data.keys('user:*')
console.log('Keys matching "user:*":', userKeys)
console.log("")

// Test 8: Has check
console.log("Test 8: Check existence")
const exists = await data.has('username')
const missing = await data.has('nonexistent')
console.log('username exists:', exists)
console.log('nonexistent exists:', missing)
console.log("")

// Test 9: Delete
console.log("Test 9: Delete key")
await data.delete('status')
await data.commit()
const statusExists = await data.has('status')
console.log('status exists after delete:', statusExists)
console.log("")

// Test 10: Storage info
console.log("Test 10: Storage limits")
console.log('MAX_KEY_SIZE:', data.MAX_KEY_SIZE)
console.log('MAX_VALUE_SIZE:', data.MAX_VALUE_SIZE)
console.log('MAX_TOTAL_SIZE:', data.MAX_TOTAL_SIZE)
console.log('MAX_KEYS:', data.MAX_KEYS)
console.log("")

// Test 11: All entries
console.log("Test 11: Get all entries")
const all = await data.entries()
console.log('Total entries:', Object.keys(all).length)
console.log('All data:', all)
console.log("")

console.log("=== All tests completed ===")
