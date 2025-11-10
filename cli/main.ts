#!/usr/bin/env bun
import { Command } from 'commander'
import * as core from './core/index'

const program = new Command()

program
  .name('tana')
  .version('0.1.0')
  .description('Tana blockchain CLI - https://tana.network')

// ============================================================================
// NEW COMMANDS - Create new entities
// ============================================================================

const newCommand = program
  .command('new')
  .description('Create new chain, node, user, or contract')
  .action(() => {
    newCommand.help()
  })

newCommand
  .command('chain <name>')
  .description('Create a new blockchain (you become the genesis leader)')
  .action(async (name: string) => {
    await core.newChain(name)
  })

newCommand
  .command('node')
  .description('Create a new node to join an existing chain')
  .option('--connect <url>', 'Chain URL to connect to')
  .action(async (options) => {
    await core.newNode(options.connect)
  })

newCommand
  .command('user <username>')
  .description('Create a new user account')
  .option('-n, --name <name>', 'Display name')
  .option('-b, --bio <bio>', 'User bio')
  .action(async (username: string, options) => {
    await core.newUser(username, options)
  })

newCommand
  .command('contract [name]')
  .description('Scaffold a new smart contract')
  .action(async (name?: string) => {
    await core.newContract(name)
  })

// ============================================================================
// DEPLOY COMMANDS - Deploy entities to blockchain
// ============================================================================

const deployCommand = program
  .command('deploy')
  .description('Deploy user, contract, or blockchain to network')
  .action(() => {
    deployCommand.help()
  })

deployCommand
  .command('user <username>')
  .description('Deploy user account to blockchain')
  .option('-t, --target <url>', 'Target chain URL')
  .action(async (username: string, options) => {
    await core.deployUser(username, options.target)
  })

deployCommand
  .command('contract <path>')
  .description('Deploy smart contract to blockchain')
  .option('-t, --target <url>', 'Target chain URL')
  .action(async (path: string, options) => {
    await core.deployContract(path, options.target)
  })

// ============================================================================
// SERVICE COMMANDS - Manage running services
// ============================================================================

program
  .command('start')
  .description('Start local chain/node services')
  .option('-c, --chain <name>', 'Specific chain to start')
  .action(async (options) => {
    await core.start(options.chain)
  })

program
  .command('stop')
  .description('Stop running services')
  .action(async () => {
    await core.stop()
  })

program
  .command('status')
  .description('Show status of running services and chains')
  .action(async () => {
    await core.status()
  })

// ============================================================================
// UTILITY COMMANDS
// ============================================================================

program
  .command('run <contract>')
  .description('Test run a contract locally')
  .action(async (contract: string) => {
    await core.runContract(contract)
  })

program
  .command('balance <user>')
  .description('Check user balance')
  .option('-c, --currency <code>', 'Currency code (default: USD)', 'USD')
  .action(async (user: string, options) => {
    await core.checkBalance(user, options.currency)
  })

program
  .command('transfer <from> <to> <amount> <currency>')
  .description('Transfer funds between users')
  .action(async (from: string, to: string, amount: string, currency: string) => {
    await core.transfer(from, to, amount, currency)
  })

program
  .command('check')
  .description('Validate system requirements')
  .action(async () => {
    await core.check()
  })

program.parse()