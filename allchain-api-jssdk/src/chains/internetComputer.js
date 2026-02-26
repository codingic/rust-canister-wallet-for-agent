import { Actor, HttpAgent } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { formatUnits, normalizeAmountResult, parseBigIntLoose } from '../utils/format.js';

function getIcrcIdlFactory() {
  return ({ IDL }) => {
    const Account = IDL.Record({ owner: IDL.Principal, subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)) });
    return IDL.Service({
      icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ['query']),
      icrc1_decimals: IDL.Func([], [IDL.Nat8], ['query'])
    });
  };
}

export async function icrcBalance({ host, canisterId, account, fetchRootKey = false, identity }) {
  if (!canisterId) throw new Error('ICRC ledger canister id is required');
  const agent = new HttpAgent({ host, identity });
  if (fetchRootKey && typeof agent.fetchRootKey === 'function') {
    await agent.fetchRootKey();
  }
  const actor = Actor.createActor(getIcrcIdlFactory(), { agent, canisterId });
  const owner = Principal.fromText(account);
  const [raw, decimals] = await Promise.all([
    actor.icrc1_balance_of({ owner, subaccount: [] }),
    actor.icrc1_decimals().catch(() => 8)
  ]);
  const d = Number(decimals ?? 8);
  return { raw: parseBigIntLoose(raw), decimals: d };
}

export async function internetComputerGetIcpBalance(ctx, account) {
  const ledgerId = ctx.icp.ledgerCanisterId || 'ryjl3-tyaaa-aaaaa-aaaba-cai';
  const { raw, decimals } = await icrcBalance({
    host: ctx.icp.host || ctx.rpc('internet_computer'),
    canisterId: ledgerId,
    account,
    fetchRootKey: Boolean(ctx.icp.fetchRootKey),
    identity: ctx.icp.identity
  });
  return normalizeAmountResult({
    network: 'internet_computer',
    account,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'icrc1_balance_of (ICP ledger)'
  });
}

export async function internetComputerGetIcrcBalance(ctx, account, token) {
  if (!token) throw new Error('token canister id is required for ICRC balance');
  const { raw, decimals } = await icrcBalance({
    host: ctx.icp.host || ctx.rpc('internet_computer'),
    canisterId: token,
    account,
    fetchRootKey: Boolean(ctx.icp.fetchRootKey),
    identity: ctx.icp.identity
  });
  return normalizeAmountResult({
    network: 'internet_computer',
    account,
    token,
    amount: formatUnits(raw, decimals),
    decimals,
    message: 'icrc1_balance_of'
  });
}
