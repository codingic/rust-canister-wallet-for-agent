import { createCanisterWalletClient } from "../src/index.js";

// Example:
//   CANISTER_ID_BACKEND=... NETWORK=ethereum TOKEN_ADDRESS=0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48 node js-sdk/examples/add-token.js
//
// Note: add/remove configured token are admin-style endpoints and may be rejected
// if the canister later enables strict auth.

const canisterId = process.env.CANISTER_ID_BACKEND;
const host = process.env.IC_HOST ?? "http://127.0.0.1:4943";
const network = process.env.NETWORK ?? "ethereum";
const tokenAddress = process.env.TOKEN_ADDRESS;

if (!canisterId) throw new Error("Missing CANISTER_ID_BACKEND");
if (!tokenAddress) throw new Error("Missing TOKEN_ADDRESS");

const client = createCanisterWalletClient({
  canisterId,
  host,
});

const added = await client.addConfiguredToken({
  network,
  tokenAddress,
});
console.log("added token:", added);

const tokens = await client.configuredTokens(network);
const found = tokens.find(
  (t) => t.token_address.toLowerCase() === tokenAddress.toLowerCase(),
);
console.log("found in configured_tokens:", found ?? null);

