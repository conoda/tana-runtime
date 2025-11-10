/**
 * Configuration Management
 *
 * Handles reading/writing JSON config files in ~/.config/tana/
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs'
import { homedir } from 'os'
import { join } from 'path'

// Base config directory
export const CONFIG_DIR = join(homedir(), '.config', 'tana')
export const CHAINS_DIR = join(CONFIG_DIR, 'chains')
export const NODES_DIR = join(CONFIG_DIR, 'nodes')
export const USERS_DIR = join(CONFIG_DIR, 'users')

/**
 * Initialize config directories if they don't exist
 */
export function ensureConfigDirs(): void {
  const dirs = [CONFIG_DIR, CHAINS_DIR, NODES_DIR, USERS_DIR]

  for (const dir of dirs) {
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true })
    }
  }
}

/**
 * Read global config
 */
export function readGlobalConfig(): GlobalConfig | null {
  const configPath = join(CONFIG_DIR, 'config.json')

  if (!existsSync(configPath)) {
    return null
  }

  try {
    const data = readFileSync(configPath, 'utf-8')
    return JSON.parse(data)
  } catch (error) {
    console.error('Error reading config:', error)
    return null
  }
}

/**
 * Write global config
 */
export function writeGlobalConfig(config: GlobalConfig): void {
  ensureConfigDirs()
  const configPath = join(CONFIG_DIR, 'config.json')
  writeFileSync(configPath, JSON.stringify(config, null, 2))
}

/**
 * Read chain config
 */
export function readChainConfig(name: string): ChainConfig | null {
  const chainPath = join(CHAINS_DIR, `${name}.json`)

  if (!existsSync(chainPath)) {
    return null
  }

  try {
    const data = readFileSync(chainPath, 'utf-8')
    return JSON.parse(data)
  } catch (error) {
    console.error(`Error reading chain config for ${name}:`, error)
    return null
  }
}

/**
 * Write chain config
 */
export function writeChainConfig(name: string, config: ChainConfig): void {
  ensureConfigDirs()
  const chainPath = join(CHAINS_DIR, `${name}.json`)
  writeFileSync(chainPath, JSON.stringify(config, null, 2))
}

/**
 * Read node config
 */
export function readNodeConfig(id: string): NodeConfig | null {
  const nodePath = join(NODES_DIR, `${id}.json`)

  if (!existsSync(nodePath)) {
    return null
  }

  try {
    const data = readFileSync(nodePath, 'utf-8')
    return JSON.parse(data)
  } catch (error) {
    console.error(`Error reading node config for ${id}:`, error)
    return null
  }
}

/**
 * Write node config
 */
export function writeNodeConfig(id: string, config: NodeConfig): void {
  ensureConfigDirs()
  const nodePath = join(NODES_DIR, `${id}.json`)
  writeFileSync(nodePath, JSON.stringify(config, null, 2))
}

/**
 * Read user config
 */
export function readUserConfig(username: string): UserConfig | null {
  const userPath = join(USERS_DIR, `${username}.json`)

  if (!existsSync(userPath)) {
    return null
  }

  try {
    const data = readFileSync(userPath, 'utf-8')
    return JSON.parse(data)
  } catch (error) {
    console.error(`Error reading user config for ${username}:`, error)
    return null
  }
}

/**
 * Write user config
 */
export function writeUserConfig(username: string, config: UserConfig): void {
  ensureConfigDirs()
  const userPath = join(USERS_DIR, `${username}.json`)
  writeFileSync(userPath, JSON.stringify(config, null, 2))
}

/**
 * List all chains
 */
export function listChains(): string[] {
  ensureConfigDirs()
  const { readdirSync } = require('fs')

  try {
    return readdirSync(CHAINS_DIR)
      .filter((file: string) => file.endsWith('.json'))
      .map((file: string) => file.replace('.json', ''))
  } catch {
    return []
  }
}

/**
 * List all nodes
 */
export function listNodes(): string[] {
  ensureConfigDirs()
  const { readdirSync } = require('fs')

  try {
    return readdirSync(NODES_DIR)
      .filter((file: string) => file.endsWith('.json'))
      .map((file: string) => file.replace('.json', ''))
  } catch {
    return []
  }
}

/**
 * List all users
 */
export function listUsers(): string[] {
  ensureConfigDirs()
  const { readdirSync } = require('fs')

  try {
    return readdirSync(USERS_DIR)
      .filter((file: string) => file.endsWith('.json'))
      .map((file: string) => file.replace('.json', ''))
  } catch {
    return []
  }
}

/**
 * Get ledger URL from environment or fallback to localhost
 *
 * Priority:
 * 1. TANA_LEDGER_URL environment variable
 * 2. http://localhost:8080 (default)
 *
 * Usage:
 *   Dev:  export TANA_LEDGER_URL=http://localhost:8080
 *   Prod: export TANA_LEDGER_URL=https://mainnet.tana.network
 */
export function getLedgerUrl(): string {
  return process.env.TANA_LEDGER_URL || 'http://localhost:8080'
}

/**
 * Check if ledger is reachable
 */
export async function isLedgerReachable(): Promise<boolean> {
  try {
    const ledgerUrl = getLedgerUrl()
    const response = await fetch(`${ledgerUrl}/health`, {
      signal: AbortSignal.timeout(2000)
    })
    return response.ok
  } catch {
    return false
  }
}

/**
 * Get deployment target
 * Returns the ledger URL if reachable, null otherwise
 */
export async function getDeploymentTarget(): Promise<string | null> {
  const ledgerUrl = getLedgerUrl()

  if (await isLedgerReachable()) {
    return ledgerUrl
  }

  return null
}

// Type definitions
export interface GlobalConfig {
  version: string
  defaultChain?: string
  defaultUser?: string
}

export interface ChainConfig {
  name: string
  type: 'local' | 'remote'
  url: string
  port?: number
  isGenesis: boolean
  createdAt: string
  genesisBlock?: string
}

export interface NodeConfig {
  id: string
  chain: string
  type: 'validator' | 'follower'
  createdAt: string
  port?: number
}

export interface UserConfig {
  username: string
  publicKey: string
  privateKey: string
  displayName: string
  createdAt: string
  chains: string[]
}
