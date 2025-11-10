import { Request, Response } from 'tana/net'
import { console } from 'tana/core'
import { block } from 'tana/block'
import { data } from 'tana/data'

export async function Get(req: Request) {
  console.log('Blockchain query contract starting...')

  try {
    // Get blockchain context
    const height = block.getHeight()
    const timestamp = block.getTimestamp()
    const gasLimit = block.getGasLimit()
    const gasUsed = block.getGasUsed()

    console.log('Block info:', { height, timestamp, gasLimit, gasUsed })

    // Query a user's balance (using ledger API)
    const balance = await block.getBalance('alice', 'USD')
    console.log('Alice balance:', balance)

    // Store query count in data
    const count = await data.get('query_count') || 0
    await data.set('query_count', count + 1)
    await data.commit()

    return Response.json({
      blockchain: {
        height,
        timestamp,
        gasLimit,
        gasUsed
      },
      userBalance: {
        user: 'alice',
        currency: 'USD',
        amount: balance
      },
      queryCount: count + 1,
      message: 'Successfully queried blockchain!'
    })
  } catch (error) {
    console.error('Error:', error)
    return Response.json({
      error: error.message || String(error)
    }, 500)
  }
}
