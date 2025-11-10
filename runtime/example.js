// example.ts
import { console, version } from "tana/core";
import { block } from "tana/block";
import { tx } from "tana/tx";
import { data } from "tana/data";
console.log("hello. this is the tana playground.");
console.log("tana's core is a blockchain written in rust.");
console.log("state changes are done with smart contracts written in typescript.");
console.log("version:", version);
console.log(`
--- Block Context ---`);
console.log(`Block Height: ${block.height}`);
console.log(`Timestamp: ${new Date(block.timestamp).toISOString()}`);
console.log(`Executor: ${block.executor}`);
console.log(`Gas Limit: ${block.gasLimit.toLocaleString()}`);
console.log(`
--- Querying Blockchain State ---`);
var alice = await block.getUser("alice");
if (alice) {
  console.log(`User: ${alice.username} (${alice.displayName})`);
  const balance = await block.getBalance(alice.id, "USD");
  console.log(`Balance: ${balance} USD`);
}
var usernames = ["alice", "bob", "charlie"];
var batchUsers = await block.getUser(usernames);
var validUsers = batchUsers.filter((u) => u !== null);
console.log(`Found ${validUsers.length} users from batch query`);
console.log(`
--- Transaction Execution ---`);
tx.transfer(block.executor, "treasury", 5, "USD");
console.log("Proposed transfer: 5 USD to treasury");
await data.set("lastTransfer", {
  from: block.executor,
  to: "treasury",
  amount: 5,
  timestamp: block.timestamp,
  blockHeight: block.height
});
await data.commit();
var pendingChanges = tx.getChanges();
console.log(`Pending changes: ${pendingChanges.length}`);
var result = await tx.execute();
if (result.success) {
  console.log("✓ Transaction executed successfully!");
  console.log(`Gas used: ${result.gasUsed.toLocaleString()}`);
  console.log("Changes:", result.changes);
} else {
  console.log("✗ Transaction failed:", result.error);
}
console.log(`
every run of your contract yields the same data.`);
console.log("this is deterministic execution on the blockchain.");
