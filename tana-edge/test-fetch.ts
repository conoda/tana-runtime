// Test file to verify feature parity between Rust CLI and playground
// This code should work identically in both environments

import { console } from 'tana:core'
import { fetch } from 'tana:utils'

console.log("Testing fetch API...\n")

// Test 1: Fetch from whitelisted domain (pokeapi.co)
try {
  const response = await fetch('https://pokeapi.co/api/v2/pokemon/pikachu')
  const data = await response.json()
  console.log("✓ Fetch successful:", data.name, "- height:", data.height)
} catch (error) {
  console.error("✗ Fetch failed:", error.message)
}

// Test 2: Try to fetch from non-whitelisted domain (should fail)
console.log("\nTesting domain whitelist...")
try {
  await fetch('https://google.com')
  console.error("✗ Whitelist bypass - this should have been blocked!")
} catch (error) {
  console.log("✓ Whitelist working:", error.message)
}

console.log("\nFeature parity test complete!")
