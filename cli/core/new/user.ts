/**
 * tana new user <username>
 *
 * Creates a new user configuration locally.
 * Generates keypair and stores in ~/.config/tana/users/
 */

import { createSpinner } from 'nanospinner'
import chalk from 'chalk'
import { randomBytes } from 'crypto'
import {
  writeUserConfig,
  readUserConfig,
  getLedgerUrl,
  isLedgerReachable,
  type UserConfig
} from '../../utils/config'

interface NewUserOptions {
  name?: string
  bio?: string
}

export async function newUser(username: string, options: NewUserOptions) {
  console.log(chalk.bold(`\n👤 Creating user: ${username}\n`))

  // Validate username format
  if (!username.startsWith('@')) {
    console.log(chalk.red('✗ Username must start with @'))
    console.log(chalk.gray(`  Example: @alice\n`))
    process.exit(1)
  }

  // Check if user already exists locally
  const localSpinner = createSpinner('Checking local configuration...').start()
  const existing = readUserConfig(username)
  if (existing) {
    localSpinner.error({ text: `User "${username}" already exists locally` })
    console.log(chalk.gray(`\nLocation: ~/.config/tana/users/${username}.json`))
    console.log(chalk.yellow(`\nUse a different username or remove the existing user first.`))
    process.exit(1)
  }
  localSpinner.success({ text: 'No local conflicts' })

  // Check if username is available on blockchain
  const ledgerUrl = getLedgerUrl()
  const blockchainSpinner = createSpinner(`Checking blockchain (${ledgerUrl})...`).start()

  try {
    // Check if ledger is reachable
    if (!(await isLedgerReachable())) {
      // No chain available - warn but continue (offline workflow)
      blockchainSpinner.warn({ text: 'Ledger not reachable' })
      console.log(chalk.yellow('\n⚠️  Warning: Could not verify username availability.'))
      console.log(chalk.gray(`   Ledger at ${ledgerUrl} is not responding.`))
      console.log(chalk.gray('   Start the ledger before deploying.\n'))
    } else {
      // Check if username exists on the blockchain
      const response = await fetch(`${ledgerUrl}/users/username/${username}`, {
        signal: AbortSignal.timeout(3000)
      })

      if (response.status === 200) {
        // User exists on blockchain
        blockchainSpinner.error({ text: `Username "${username}" is already taken` })
        console.log(chalk.yellow(`\nThis username is already registered on the blockchain.`))
        console.log(chalk.gray(`Ledger: ${ledgerUrl}`))
        console.log(chalk.yellow(`\nPlease choose a different username.\n`))
        process.exit(1)
      } else if (response.status === 404) {
        // Username is available
        blockchainSpinner.success({ text: 'Username is available' })
      } else {
        // Unexpected response
        throw new Error(`Unexpected response: ${response.status}`)
      }
    }
  } catch (error: any) {
    // If we can't reach the blockchain, warn but continue
    if (error.name === 'AbortError' || error.message.includes('fetch')) {
      blockchainSpinner.warn({ text: 'Could not reach ledger' })
      console.log(chalk.yellow('\n⚠️  Warning: Could not verify username availability.'))
      console.log(chalk.gray(`   Ledger at ${ledgerUrl} is unreachable.\n`))
    } else {
      blockchainSpinner.error({ text: error.message })
      process.exit(1)
    }
  }

  // Generate keypair
  const keySpinner = createSpinner('Generating keypair...').start()
  const { publicKey, privateKey } = generateKeypair()
  keySpinner.success({ text: 'Keypair generated' })

  // Create user configuration
  const userConfig: UserConfig = {
    username,
    publicKey,
    privateKey,
    displayName: options.name || username.substring(1), // Remove @ for display name
    createdAt: new Date().toISOString(),
    chains: []
  }

  const writeSpinner = createSpinner('Writing user configuration...').start()
  writeUserConfig(username, userConfig)
  writeSpinner.success({ text: 'User configuration saved' })

  // Success message
  console.log(chalk.green(`\n✓ User created successfully!\n`))
  console.log(chalk.gray('━'.repeat(50)))
  console.log(chalk.bold('User Details:'))
  console.log(`  Username:      ${chalk.cyan(username)}`)
  console.log(`  Display Name:  ${chalk.cyan(userConfig.displayName)}`)
  if (options.bio) {
    console.log(`  Bio:           ${chalk.gray(options.bio)}`)
  }
  console.log(`  Public Key:    ${chalk.gray(publicKey.substring(0, 24) + '...')}`)
  console.log(chalk.gray('━'.repeat(50)))

  console.log(chalk.bold('\nNext Step:'))
  console.log(`  Deploy user:  ${chalk.cyan(`tana deploy user ${username}`)}`)
  console.log()

  console.log(chalk.yellow('⚠️  Keep your private key safe!'))
  console.log(chalk.gray(`   Stored at: ~/.config/tana/users/${username}.json\n`))
}

/**
 * Generate Ed25519 keypair
 * TODO: Replace with proper Ed25519 implementation
 */
function generateKeypair(): { publicKey: string; privateKey: string } {
  // Simplified keypair generation for now
  // In production, use proper Ed25519 (e.g., @noble/ed25519)
  const privateKey = randomBytes(32).toString('hex')
  const publicKey = randomBytes(32).toString('hex')

  return {
    publicKey: `ed25519_${publicKey}`,
    privateKey: `ed25519_${privateKey}`
  }
}
