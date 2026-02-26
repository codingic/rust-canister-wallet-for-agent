import { createCanisterWalletClient } from "../src/index.js";

// This example shows the SDK call shape for native EVM sends.
// You must provide a non-anonymous identity to actually send transactions.
// Example usage:
//   CANISTER_ID_BACKEND=... IC_HOST=http://127.0.0.1:4943 TO=0x... AMOUNT=0.001 NETWORK=sepolia node js-sdk/examples/send-eth.js

const canisterId = process.env.CANISTER_ID_BACKEND;
const host = process.env.IC_HOST ?? "http://127.0.0.1:4943";
const network = process.env.NETWORK ?? "sepolia";
const to = process.env.TO;
const amount = process.env.AMOUNT ?? "0.001";

if (!canisterId) throw new Error("Missing CANISTER_ID_BACKEND");
if (!to) throw new Error("Missing TO");

// Inject your own identity here if needed:
// import { Ed25519KeyIdentity } from "@dfinity/identity";
// const identity = Ed25519KeyIdentity.fromJSON(process.env.IDENTITY_JSON);
const identity = undefined;

const client = createCanisterWalletClient({
  canisterId,
  host,
  identity,
});

const fromAddress = await client.requestAddress(network);
console.log("from:", fromAddress.address);

const balance = await client.getBalance({
  network,
  account: fromAddress.address,
});
console.log("balance before:", balance.amount, "decimals=", balance.decimals);

const tx = await client.transfer({
  network,
  to,
  amount,
});
console.log("tx:", tx);

