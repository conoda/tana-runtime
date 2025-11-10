import { Request, Response } from 'tana/net'
import { console } from 'tana/core'
import { block } from 'tana/block'
import { data } from 'tana/data'
import { tx } from 'tana/tx'

export async function Get(req: Request) {
  console.log('Testing all tana modules...')

  // Test tana/core
  console.log('✓ tana/core works')

  // Test tana/block (blockchain context - no external API calls)
  const height = Number(block.getHeight())
  const timestamp = block.getTimestamp()
  const hash = block.getHash()
  const executor = block.getExecutor()
  const gasLimit = Number(block.getGasLimit())
  const gasUsed = Number(block.getGasUsed())

  console.log('✓ tana/block works', { height, executor })

  // Test tana/data (storage)
  const visitCount = await data.get('visits') || 0
  await data.set('visits', visitCount + 1)
  await data.set('lastVisit', Date.now())
  await data.commit()

  const keys = await data.keys()
  console.log('✓ tana/data works, keys:', keys)

  // Test tana/tx (transaction staging)
  tx.transfer('alice', 'bob', 100, 'USD')
  tx.setBalance('charlie', 500, 'USD')
  const changes = tx.getChanges()

  console.log('✓ tana/tx works, changes:', changes)

  return Response.json({
    message: 'All modules working!',
    modules: {
      'tana/core': 'OK',
      'tana/net': 'OK',
      'tana/block': 'OK',
      'tana/data': 'OK',
      'tana/tx': 'OK'
    },
    blockchainContext: {
      height,
      timestamp,
      hash,
      executor,
      gasLimit,
      gasUsed
    },
    storage: {
      visitCount: visitCount + 1,
      totalKeys: keys.length
    },
    transactions: {
      staged: changes.length,
      changes
    }
  })
}
