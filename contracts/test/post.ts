import { Request, Response } from 'tana/net'
import { console } from 'tana/core'

export function Post(req: Request, body: any) {
  console.log('POST request received:', req.path)
  console.log('Body:', body)

  return Response.json({
    message: 'POST received!',
    echo: body,
    timestamp: Date.now()
  })
}
