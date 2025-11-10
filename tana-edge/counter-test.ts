// Simple counter contract test for Rust runtime
import { console } from 'tana:core'
import { data } from 'tana:data'

console.log("=== Counter Contract Test ===\n")

// Read current counter
const current = await data.get('counter')
const count = current ? parseInt(current as string) : 0

console.log("Current count:", count)

// Increment
await data.set('counter', String(count + 1))
await data.set('timestamp', new Date().toISOString())
await data.set('user', { name: 'alice', type: 'test' })

// Commit
console.log("\nCommitting changes...")
await data.commit()

console.log("✓ Committed successfully")
console.log("New count:", count + 1)

// Verify
const newCount = await data.get('counter')
const timestamp = await data.get('timestamp')
const user = await data.get('user')

console.log("\nVerification:")
console.log("Counter:", newCount)
console.log("Timestamp:", timestamp)
console.log("User:", user)

// List all keys
const allKeys = await data.keys()
console.log("\nAll keys:", allKeys)
