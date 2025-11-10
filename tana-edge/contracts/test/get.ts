import { Request, Response } from 'tana/net'
import { console } from 'tana/core'

export function Get(req: Request) {
  console.log('GET request received!')

  return Response.json({
    message: 'Hello from tana-edge!',
    timestamp: Date.now()
  })
}
