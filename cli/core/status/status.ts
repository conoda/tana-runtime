import chalk from 'chalk'
import {
  readGlobalConfig,
  listChains,
  listUsers,
  isLedgerReachable,
  getLedgerUrl
} from '../../utils/config'

export async function status() {
  console.log(chalk.bold('\n📊 Tana Status\n'))

  // Check if ledger is running
  const ledgerUrl = getLedgerUrl()
  const isRunning = await isLedgerReachable()

  console.log(chalk.bold('Services:'))
  if (isRunning) {
    console.log(`  Ledger:        ${chalk.green('●')} Running on ${chalk.cyan(ledgerUrl)}`)
  } else {
    console.log(`  Ledger:        ${chalk.gray('○')} Not running`)
    console.log(`  Target URL:    ${chalk.gray(ledgerUrl)}`)
  }
  console.log()

  // Global config
  const config = readGlobalConfig()
  console.log(chalk.bold('Configuration:'))
  console.log(`  Default Chain: ${config?.defaultChain ? chalk.cyan(config.defaultChain) : chalk.gray('(none)')}`)
  console.log(`  Default User:  ${config?.defaultUser ? chalk.cyan(config.defaultUser) : chalk.gray('(none)')}`)
  console.log()

  // Chains
  const chains = listChains()
  console.log(chalk.bold('Chains:'))
  if (chains.length === 0) {
    console.log(chalk.gray('  (no chains configured)'))
  } else {
    chains.forEach(chain => {
      const marker = chain === config?.defaultChain ? chalk.green('●') : chalk.gray('○')
      console.log(`  ${marker} ${chalk.cyan(chain)}`)
    })
  }
  console.log()

  // Users
  const users = listUsers()
  console.log(chalk.bold('Users:'))
  if (users.length === 0) {
    console.log(chalk.gray('  (no users configured)'))
  } else {
    users.forEach(user => {
      const marker = user === config?.defaultUser ? chalk.green('●') : chalk.gray('○')
      console.log(`  ${marker} ${chalk.cyan(user)}`)
    })
  }
  console.log()

  // Quick commands
  if (!isRunning) {
    console.log(chalk.gray('💡 Start the ledger with: ') + chalk.cyan('tana start'))
    console.log()
  }
}