// Example smart contract: Simple counter with persistent storage
// This demonstrates the planned tana:data API

import { console } from 'tana:core'
import { data } from 'tana:data'

console.log("Counter Contract\n")

// Read current counter value
const currentValue = await data.get('counter')
const count = currentValue ? parseInt(currentValue) : 0

console.log('Current count:', count)

// Increment counter
const newCount = count + 1
await data.set('counter', String(newCount))

// Store metadata
await data.set('lastUpdate', new Date().toISOString())
await data.set('totalUpdates', String(newCount))

// Show staged changes before commit
const staged = await data.entries()
console.log('\nStaged changes:', staged)

// Validate and commit to blockchain
try {
  await data.commit()
  console.log('\n✓ Changes committed successfully')
  console.log('New count:', newCount)
} catch (error) {
  console.error('\n✗ Commit failed:', (error as Error).message)
}

// Show storage usage
const allKeys = await data.keys()
console.log('\nTotal keys:', allKeys.length)
