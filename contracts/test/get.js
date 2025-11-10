// test/get.ts
import { Response } from "tana/net";
import { console } from "tana/core";
import { block } from "tana/block";
async function Get(req) {
  console.log("GET request received:", req.path);
  try {
    const height = block.getHeight();
    const timestamp = block.getTimestamp();
    const hash = block.getHash();
    const gasUsed = block.getGasUsed();
    let balance = null;
    try {
      balance = await block.getBalance("usr_alice", "USD");
    } catch (e) {
      console.log("Balance query skipped (no user found)");
    }
    return Response.json({
      message: "Hello from tana-edge with blockchain query!",
      path: req.path,
      method: req.method,
      timestamp: Date.now(),
      blockchain: {
        height: Number(height),
        timestamp,
        hash,
        gasUsed: Number(gasUsed),
        balance
      }
    });
  } catch (error) {
    console.error("Error querying blockchain:", error);
    return Response.json({
      error: "Failed to query blockchain",
      details: error.message || String(error)
    }, 500);
  }
}
export {
  Get
};
