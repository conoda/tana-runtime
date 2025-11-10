/**
 * tana start
 *
 * Starts the ledger HTTP server directly within the CLI process.
 * Runs in foreground on port 8080.
 */

import chalk from 'chalk'
import {
  readGlobalConfig,
  isLedgerReachable,
  getLedgerUrl
} from '../../utils/config'
import { startLedger } from '../ledger/server'

export async function start(chainName?: string) {
  console.log(chalk.bold('\n🚀 Starting Tana...\n'))

  // Check if already running
  const ledgerUrl = getLedgerUrl()
  if (await isLedgerReachable()) {
    console.log(chalk.yellow(`✗ Ledger already running at ${ledgerUrl}`))
    console.log(chalk.gray(`\nStop the other instance first, or use a different port.\n`))
    process.exit(1)
  }

  // Determine which chain to start
  let targetChain = chainName
  if (!targetChain) {
    const config = readGlobalConfig()
    targetChain = config?.defaultChain || 'local'
  }

  console.log(chalk.gray(`Chain: ${chalk.cyan(targetChain)}`))
  console.log()

  // Start ledger server
  // DATABASE_URL will be loaded from .env file by startLedger if not set
  await startLedger({
    port: 8080,
    chain: targetChain,
    databaseUrl: process.env.DATABASE_URL || ''
  })
}
