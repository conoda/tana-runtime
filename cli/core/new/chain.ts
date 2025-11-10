/**
 * tana new chain <name>
 *
 * Creates a new blockchain configuration.
 * The user becomes the genesis/leader of this chain.
 */

import { createSpinner } from 'nanospinner'
import chalk from 'chalk'
import {
  writeChainConfig,
  writeGlobalConfig,
  readGlobalConfig,
  readChainConfig,
  getLedgerUrl,
  type ChainConfig,
  type GlobalConfig
} from '../../utils/config'

export async function newChain(name: string) {
  console.log(chalk.bold(`\n🔗 Creating new blockchain: ${name}\n`))

  const spinner = createSpinner('Checking for existing chain...').start()

  // Check if chain already exists
  const existing = readChainConfig(name)
  if (existing) {
    spinner.error({ text: `Chain "${name}" already exists` })
    console.log(chalk.gray(`\nLocation: ~/.config/tana/chains/${name}.json`))
    console.log(chalk.yellow(`\nUse a different name or remove the existing chain first.`))
    process.exit(1)
  }

  spinner.success({ text: 'No conflicts found' })

  // Create genesis block hash
  const genesisHash = generateGenesisHash(name)

  // Create chain configuration
  const ledgerUrl = getLedgerUrl()
  const chainConfig: ChainConfig = {
    name,
    type: 'local',
    url: ledgerUrl,
    port: 8080,
    isGenesis: true,
    createdAt: new Date().toISOString(),
    genesisBlock: genesisHash
  }

  const writeSpinner = createSpinner('Writing chain configuration...').start()
  writeChainConfig(name, chainConfig)
  writeSpinner.success({ text: 'Chain configuration saved' })

  // Set as default chain if no default exists
  const globalConfig = readGlobalConfig()
  if (!globalConfig?.defaultChain) {
    const setDefaultSpinner = createSpinner('Setting as default chain...').start()

    const newGlobalConfig: GlobalConfig = {
      version: '0.1.0',
      defaultChain: name,
      ...(globalConfig || {})
    }

    writeGlobalConfig(newGlobalConfig)
    setDefaultSpinner.success({ text: 'Set as default chain' })
  }

  // Success message
  console.log(chalk.green(`\n✓ Blockchain created successfully!\n`))
  console.log(chalk.gray('━'.repeat(50)))
  console.log(chalk.bold('Chain Details:'))
  console.log(`  Name:          ${chalk.cyan(name)}`)
  console.log(`  Type:          ${chalk.cyan('Genesis (Leader)')}`)
  console.log(`  URL:           ${chalk.cyan(chainConfig.url)}`)
  console.log(`  Genesis Block: ${chalk.gray(genesisHash.substring(0, 16) + '...')}`)
  console.log(chalk.gray('━'.repeat(50)))

  console.log(chalk.bold('\nNext Steps:'))
  console.log(`  1. Start your chain:  ${chalk.cyan(`tana start`)}`)
  console.log(`  2. Create a user:     ${chalk.cyan(`tana new user <username>`)}`)
  console.log(`  3. Check status:      ${chalk.cyan(`tana status`)}`)
  console.log()
}

/**
 * Generate a deterministic genesis block hash
 */
function generateGenesisHash(name: string): string {
  const crypto = require('crypto')
  const data = `${name}-${Date.now()}`
  return '0x' + crypto.createHash('sha256').update(data).digest('hex')
}
