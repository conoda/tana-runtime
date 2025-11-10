#!/usr/bin/env bun
/**
 * CLI Contract Executor
 *
 * Execute Tana smart contracts from the terminal with the same
 * environment as the playground sandbox.
 *
 * Usage:
 *   bun scripts/run-contract.ts <contract-file.ts>
 *   bun scripts/run-contract.ts examples/transfer.ts
 */

import { readFileSync } from 'fs';
import { resolve } from 'path';

// ANSI color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
};

// Get ledger URL from environment or default to localhost
const LEDGER_URL = process.env.TANA_LEDGER_URL || 'http://localhost:8080';

// Mock block context (same as sandbox)
const mockBlock = {
  height: 12345,
  timestamp: Date.now(),
  hash: '0x' + Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join(''),
  previousHash: '0x' + Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join(''),
  executor: 'user_cli_test',
  contractId: 'contract_cli',
  gasLimit: 1000000,
  gasUsed: 0,
  MAX_BATCH_QUERY: 10,

  async getBalance(userIds: string | string[], currencyCode: string) {
    const ids = Array.isArray(userIds) ? userIds : [userIds];

    if (ids.length > this.MAX_BATCH_QUERY) {
      throw new Error(`Cannot query more than ${this.MAX_BATCH_QUERY} balances at once`);
    }

    try {
      const response = await fetch(`${LEDGER_URL}/balances`);
      const allBalances = await response.json();

      const results = ids.map(userId => {
        const balance = allBalances.find((b: any) =>
          b.ownerId === userId && b.currencyCode === currencyCode
        );
        return balance ? parseFloat(balance.amount) : 0;
      });

      return Array.isArray(userIds) ? results : results[0];
    } catch (error) {
      console.error('Failed to fetch balance:', error);
      return Array.isArray(userIds) ? ids.map(() => 0) : 0;
    }
  },

  async getUser(userIds: string | string[]) {
    const ids = Array.isArray(userIds) ? userIds : [userIds];

    if (ids.length > this.MAX_BATCH_QUERY) {
      throw new Error(`Cannot query more than ${this.MAX_BATCH_QUERY} users at once`);
    }

    try {
      const response = await fetch(`${LEDGER_URL}/users`);
      const allUsers = await response.json();

      const results = ids.map(userId => {
        return allUsers.find((u: any) => u.id === userId || u.username === userId) || null;
      });

      return Array.isArray(userIds) ? results : results[0];
    } catch (error) {
      console.error('Failed to fetch user:', error);
      return Array.isArray(userIds) ? ids.map(() => null) : null;
    }
  },

  async getTransaction(txIds: string | string[]) {
    const ids = Array.isArray(txIds) ? txIds : [txIds];

    if (ids.length > this.MAX_BATCH_QUERY) {
      throw new Error(`Cannot query more than ${this.MAX_BATCH_QUERY} transactions at once`);
    }

    try {
      const response = await fetch(`${LEDGER_URL}/transactions`);
      const allTransactions = await response.json();

      const results = ids.map(txId => {
        return allTransactions.find((tx: any) => tx.id === txId) || null;
      });

      return Array.isArray(txIds) ? results : results[0];
    } catch (error) {
      console.error('Failed to fetch transaction:', error);
      return Array.isArray(txIds) ? ids.map(() => null) : null;
    }
  }
};

// Whitelisted fetch
const ALLOWED_DOMAINS = [
  'pokeapi.co',
  'tana.dev',
  'tana.network',
  'tana-runtime.pages.dev',
  'api.tana.dev',
  'blockchain.tana.dev',
  'localhost',
  '127.0.0.1'
];

function whitelistedFetch(url: string, options?: RequestInit) {
  let parsedUrl;
  try {
    parsedUrl = new URL(url);
  } catch (e) {
    return Promise.reject(new Error(`Invalid URL: ${url}`));
  }

  const hostname = parsedUrl.hostname;
  const isAllowed = ALLOWED_DOMAINS.some(domain => {
    return hostname === domain || hostname.endsWith('.' + domain);
  });

  if (!isAllowed) {
    return Promise.reject(new Error(
      `fetch blocked: domain "${hostname}" not in whitelist. ` +
      `Allowed domains: ${ALLOWED_DOMAINS.join(', ')}`
    ));
  }

  return fetch(url, options);
}

// Create tana modules
const tanaModules = {
  'tana/core': {
    console: {
      log(...args: any[]) {
        const msg = args.map(v => {
          if (typeof v === 'object') {
            try { return JSON.stringify(v, null, 2); }
            catch { return String(v); }
          }
          return String(v);
        }).join(' ');
        console.log(`${colors.cyan}[LOG]${colors.reset}`, msg);
      },
      error(...args: any[]) {
        const msg = args.map(v => {
          if (typeof v === 'object') {
            try { return JSON.stringify(v, null, 2); }
            catch { return String(v); }
          }
          return String(v);
        }).join(' ');
        console.error(`${colors.red}[ERROR]${colors.reset}`, msg);
      }
    },
    version: {
      tana: '0.1.0',
      deno_core: '0.338.0',
      v8: '134.5.0'
    }
  },
  'tana/utils': {
    fetch: whitelistedFetch
  },
  'tana/block': {
    block: mockBlock
  },
  'tana/tx': {
    tx: {
      _changes: [] as any[],

      transfer(from: string, to: string, amount: number, currency: string) {
        if (from === to) {
          throw new Error('Cannot transfer to self');
        }
        if (amount <= 0) {
          throw new Error('Amount must be positive');
        }
        this._changes.push({
          type: 'transfer',
          from,
          to,
          amount,
          currency
        });
      },

      setBalance(userId: string, amount: number, currency: string) {
        if (amount < 0) {
          throw new Error('Balance cannot be negative');
        }
        this._changes.push({
          type: 'balance_update',
          userId,
          amount,
          currency
        });
      },

      getChanges() {
        return [...this._changes];
      },

      async execute() {
        const changes = [...this._changes];
        const gasUsed = mockBlock.gasUsed + (changes.length * 100);

        if (gasUsed > mockBlock.gasLimit) {
          const error = 'Out of gas';
          this._changes = [];
          return {
            success: false,
            changes: [],
            gasUsed: mockBlock.gasLimit,
            error
          };
        }

        mockBlock.gasUsed = gasUsed;
        this._changes = [];

        // Display transaction result
        console.log(`\n${colors.green}${colors.bright}✓ Transaction Executed${colors.reset}`);
        console.log(`${colors.gray}Gas Used: ${gasUsed.toLocaleString()}${colors.reset}`);

        if (changes.length > 0) {
          console.log(`\n${colors.bright}State Changes (${changes.length}):${colors.reset}`);
          changes.forEach((change, i) => {
            if (change.type === 'transfer') {
              console.log(`  ${i + 1}. Transfer: ${colors.yellow}${change.from}${colors.reset} → ${colors.yellow}${change.to}${colors.reset} (${change.amount} ${change.currency})`);
            } else if (change.type === 'balance_update') {
              console.log(`  ${i + 1}. Balance Update: ${colors.yellow}${change.userId}${colors.reset} = ${change.amount} ${change.currency}`);
            }
          });
        }

        return {
          success: true,
          changes,
          gasUsed,
          error: null
        };
      }
    }
  },
  'tana/data': {
    data: {
      MAX_KEY_SIZE: 256,
      MAX_VALUE_SIZE: 10240,
      MAX_TOTAL_SIZE: 102400,
      MAX_KEYS: 1000,

      _staging: new Map<string, any>(),
      _storage: new Map<string, any>(),

      _serialize(value: any) {
        if (typeof value === 'string') {
          return value;
        }
        return JSON.stringify(value);
      },

      _deserialize(value: any) {
        if (value === null) return null;
        try {
          return JSON.parse(value);
        } catch {
          return value;
        }
      },

      async set(key: string, value: any) {
        if (typeof key !== 'string') {
          throw new Error('Key must be a string');
        }
        if (key.length > this.MAX_KEY_SIZE) {
          throw new Error(`Key too large: ${key.length} bytes (max ${this.MAX_KEY_SIZE})`);
        }

        const serialized = this._serialize(value);
        if (serialized.length > this.MAX_VALUE_SIZE) {
          throw new Error(`Value too large: ${serialized.length} bytes (max ${this.MAX_VALUE_SIZE})`);
        }

        this._staging.set(key, serialized);
      },

      async get(key: string) {
        if (this._staging.has(key)) {
          return this._deserialize(this._staging.get(key));
        }
        return this._deserialize(this._storage.get(key));
      },

      async delete(key: string) {
        this._staging.set(key, null);
      },

      async has(key: string) {
        if (this._staging.has(key)) {
          return this._staging.get(key) !== null;
        }
        return this._storage.has(key);
      },

      async keys(pattern?: string) {
        const allKeys = new Set<string>();

        for (const key of this._storage.keys()) {
          allKeys.add(key);
        }

        for (const [key, value] of this._staging) {
          if (value === null) {
            allKeys.delete(key);
          } else {
            allKeys.add(key);
          }
        }

        const keysArray = Array.from(allKeys);

        if (pattern) {
          const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
          return keysArray.filter(k => regex.test(k));
        }

        return keysArray;
      },

      async entries() {
        const result: Record<string, any> = {};
        const allKeys = await this.keys();

        for (const key of allKeys) {
          result[key] = await this.get(key);
        }

        return result;
      },

      async clear() {
        this._storage.clear();
        this._staging.clear();
      },

      async commit() {
        for (const [key, value] of this._staging) {
          if (value === null) {
            this._storage.delete(key);
          } else {
            this._storage.set(key, value);
          }
        }
        this._staging.clear();
      }
    }
  }
};

// Import resolver
function createTanaImport() {
  return function __tanaImport(spec: string) {
    // Convert tana: to tana/ for module lookup
    const normalizedSpec = spec.replace('tana:', 'tana/');
    const mod = (tanaModules as any)[normalizedSpec];
    if (!mod) throw new Error(`Unknown module: ${spec}`);
    return mod;
  };
}

// Main execution
async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error(`${colors.red}Error: No contract file specified${colors.reset}`);
    console.log(`\nUsage: bun scripts/run-contract.ts <contract-file.ts>`);
    console.log(`Example: bun scripts/run-contract.ts examples/transfer.ts\n`);
    process.exit(1);
  }

  const contractPath = resolve(args[0]);

  console.log(`${colors.bright}Tana Contract Executor${colors.reset}`);
  console.log(colors.gray + '━'.repeat(50) + colors.reset + '\n');
  console.log(`${colors.blue}Contract:${colors.reset} ${contractPath}`);
  console.log(`${colors.blue}Block:${colors.reset} ${mockBlock.height}`);
  console.log(`${colors.blue}Executor:${colors.reset} ${mockBlock.executor}`);
  console.log(`${colors.blue}Gas Limit:${colors.reset} ${mockBlock.gasLimit.toLocaleString()}\n`);
  console.log(colors.gray + '━'.repeat(50) + colors.reset + '\n');

  let code: string;
  try {
    code = readFileSync(contractPath, 'utf-8');
  } catch (error) {
    console.error(`${colors.red}Error: Could not read contract file${colors.reset}`);
    console.error(error);
    process.exit(1);
  }

  // Rewrite import statements
  const rewrittenCode = code
    .split('\n')
    .map(line => {
      const match = line.match(/^\s*import\s+\{([^}]+)\}\s+from\s+["'](tana:[^"']+)["'];?\s*$/);
      if (!match) return line;
      const names = match[1].trim();
      const spec = match[2].trim();
      return `const {${names}} = __tanaImport('${spec}');`;
    })
    .join('\n');

  try {
    // Create execution context
    const __tanaImport = createTanaImport();

    // Execute contract
    const AsyncFunction = (async function () {}).constructor;
    const fn = new AsyncFunction('__tanaImport', rewrittenCode);
    await fn(__tanaImport);

    console.log('\n' + colors.gray + '━'.repeat(50) + colors.reset);
    console.log(`${colors.green}✓ Execution complete${colors.reset}\n`);

  } catch (error: any) {
    console.log('\n' + colors.gray + '━'.repeat(50) + colors.reset);
    console.error(`${colors.red}${colors.bright}✗ Execution failed${colors.reset}`);
    console.error(`${colors.red}${error.message}${colors.reset}`);
    if (error.stack) {
      console.error(`${colors.gray}${error.stack}${colors.reset}`);
    }
    console.log();
    process.exit(1);
  }
}

main();
