import { createCanisterWalletClient } from "../src/index.js";

const canisterId = process.env.CANISTER_ID_BACKEND;
const host = process.env.IC_HOST ?? "http://127.0.0.1:4943";

if (!canisterId) {
  throw new Error("Missing CANISTER_ID_BACKEND");
}

const client = createCanisterWalletClient({
  canisterId,
  host,
});

const serviceInfo = await client.serviceInfo();
console.log("service_info:", serviceInfo);

const walletNetworks = await client.walletNetworks();
console.log(
  "wallet_networks:",
  walletNetworks.map((n) => ({
    id: n.id,
    primary_symbol: n.primary_symbol,
    address_family: n.address_family,
    shared_address_group: n.shared_address_group,
  })),
);

const sharedGroups = await client.sharedAddressGroups();
console.log("shared_address_groups:", sharedGroups);

const ethAddr = await client.requestAddress("ethereum");
console.log("ethereum address:", ethAddr.address);

const icpAddr = await client.requestAddress("internet_computer");
console.log("internet_computer address:", icpAddr.address);

