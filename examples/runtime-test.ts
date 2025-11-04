// Runtime test - uses only currently implemented modules
// This works in the Rust runtime right now
import { console, version } from 'tana:core'
import { data } from 'tana:data'
import { fetch } from 'tana:utils'

console.log("=== Tana Rust Runtime Test ===\n")
console.log("Version:", version)

// 1. DATA STORAGE TEST
console.log("\n--- Data Storage Test ---")

// Read current state
const counter = await data.get('counter')
const count = counter ? parseInt(counter as string) : 0
console.log(`Current counter: ${count}`)

// Update state
await data.set('counter', String(count + 1))
await data.set('timestamp', new Date().toISOString())
await data.set('metadata', {
  run: count + 1,
  runtime: 'rust',
  test: true
})

// Commit changes
await data.commit()
console.log("✓ State committed")

// Verify
const newCounter = await data.get('counter')
const timestamp = await data.get('timestamp')
const metadata = await data.get('metadata')

console.log(`New counter: ${newCounter}`)
console.log(`Timestamp: ${timestamp}`)
console.log(`Metadata:`, metadata)

// 2. FETCH TEST
console.log("\n--- Fetch Test ---")

try {
  const response = await fetch('https://pokeapi.co/api/v2/pokemon/pikachu')
  const pokemon = JSON.parse(response)
  console.log(`Fetched: ${pokemon.name}`)
  console.log(`Height: ${pokemon.height}`)
  console.log(`Weight: ${pokemon.weight}`)
} catch (error) {
  console.log("Fetch failed:", error)
}

// 3. PATTERN MATCHING TEST
console.log("\n--- Pattern Matching Test ---")

await data.set('user:1:name', 'Alice')
await data.set('user:2:name', 'Bob')
await data.set('user:3:name', 'Charlie')
await data.commit()

const userKeys = await data.keys('user:*')
console.log(`Found ${userKeys.length} user keys:`, userKeys)

console.log("\n=== Test Complete ===")
console.log("\nNOTE: tana:block and tana:tx modules need to be added to Rust runtime")
console.log("See docs/CONTRACT_EXECUTION.md for implementation details")
