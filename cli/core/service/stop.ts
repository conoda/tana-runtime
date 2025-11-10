/**
 * tana stop
 *
 * Stops running services
 */

import chalk from 'chalk'
import { isLedgerReachable } from '../../utils/config'

export async function stop() {
  console.log(chalk.bold('\n🛑 Stopping Tana services...\n'))

  // Check if anything is running
  if (!(await isLedgerReachable())) {
    console.log(chalk.gray('No services are currently running.\n'))
    return
  }

  console.log(chalk.yellow('⚠️  Stop functionality coming soon'))
  console.log()
  console.log(chalk.gray('For now, stop the ledger process manually with Ctrl+C'))
  console.log()
}
