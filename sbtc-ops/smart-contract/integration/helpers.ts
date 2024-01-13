import { Accounts, Contracts } from './constants';
import {
  StacksBlockMetadata,
  StacksChainUpdate,
  DevnetNetworkOrchestrator,
  StacksTransactionMetadata,
  getIsolatedNetworkConfigUsingNetworkId,
} from '@hirosystems/stacks-devnet-js';
import { StacksNetwork } from '@stacks/network';
import {
  AnchorMode,
  broadcastTransaction,
  makeContractCall,
  PostConditionMode,
  TxBroadcastResult,
  uintCV,
  callReadOnlyFunction,
  cvToJSON,
  principalCV,
  standardPrincipalCV,
  bufferCV,
  tupleCV,
} from '@stacks/transactions';
import { Constants } from './constants';
const fetch = require('node-fetch');

interface Account {
  stxAddress: string;
  btcAddress: string;
  secretKey: string;
}

interface EpochTimeline {
  epoch_2_0: number;
  epoch_2_05: number;
  epoch_2_1: number;
  pox_2_activation: number;
}

export const DEFAULT_EPOCH_TIMELINE = {
  epoch_2_0: Constants.DEVNET_DEFAULT_EPOCH_2_0,
  epoch_2_05: Constants.DEVNET_DEFAULT_EPOCH_2_05,
  epoch_2_1: Constants.DEVNET_DEFAULT_EPOCH_2_1,
  pox_2_activation: Constants.DEVNET_DEFAULT_POX_2_ACTIVATION,
};

export const delay = () => new Promise((resolve) => setTimeout(resolve, 3000));

export function buildDevnetNetworkOrchestrator(
  networkId: number,
  timeline: EpochTimeline = DEFAULT_EPOCH_TIMELINE,
  logs = true
) {
  let uuid = Date.now();
  let working_dir = `/tmp/stacks-test-${uuid}-${networkId}`;
  let config = {
    logs,
    devnet: {
      name: `ephemeral-devnet-${uuid}`,
      bitcoin_controller_block_time: Constants.BITCOIN_BLOCK_TIME,
      bitcoin_controller_automining_disabled: false,
      working_dir,
      use_docker_gateway_routing: process.env.GITHUB_ACTIONS ? true : false,
    },
  };
  let consolidatedConfig = getIsolatedNetworkConfigUsingNetworkId(networkId, config);
  let orchestrator = new DevnetNetworkOrchestrator(consolidatedConfig, 2500);
  return orchestrator;
}

export const getBitcoinBlockHeight = (chainUpdate: StacksChainUpdate): number => {
  let metadata = chainUpdate.new_blocks[0].block.metadata! as StacksBlockMetadata;
  return metadata.bitcoin_anchor_block_identifier.index;
};

export const waitForStacksTransaction = async (
  orchestrator: DevnetNetworkOrchestrator,
  txid: string
): Promise<[StacksBlockMetadata, StacksTransactionMetadata]> => {
  let { chainUpdate, transaction } = await orchestrator.waitForStacksBlockIncludingTransaction(txid);
  return [
    <StacksBlockMetadata>chainUpdate.new_blocks[0].block.metadata,
    <StacksTransactionMetadata>transaction.metadata,
  ];
};

export const getNetworkIdFromEnv = (): number => {
  let networkId = process.env.JEST_WORKER_ID
    ? parseInt(process.env.JEST_WORKER_ID!)
    : process.env.VITEST_WORKER_ID
    ? parseInt(process.env.VITEST_WORKER_ID!)
    : 1;
  return networkId;
};

export const getChainInfo = async (network: StacksNetwork, retry?: number): Promise<any> => {
  let retryCountdown = retry ? retry : 20;
  if (retryCountdown == 0) return Promise.reject();
  try {
    let response = await fetch(network.getInfoUrl(), {});
    let info = await response.json();
    return info;
  } catch (e) {
    await delay();
    return await getChainInfo(network, retryCountdown - 1);
  }
};

export const getAccount = async (network: StacksNetwork, address: string, retry?: number): Promise<any> => {
  let retryCountdown = retry ? retry : 20;
  if (retryCountdown == 0) return Promise.reject();
  try {
    let response = await fetch(network.getAccountApiUrl(address), {});
    let payload: any = await response.json();
    return {
      balance: BigInt(payload.balance),
      locked: BigInt(payload.locked),
      unlock_height: payload.unlock_height,
      nonce: payload.nonce,
    };
  } catch (e) {
    await delay();
    return await getAccount(network, address, retryCountdown - 1);
  }
};

export const balanceDepositSTX = async (
  amount: number,
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'deposit-stx',
    functionArgs: [uintCV(amount * 1000000)],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const getBalanceSTX = async (network: StacksNetwork, participant: string) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'get-balance',
    functionArgs: [principalCV(participant)],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  return json;
};

export const getRewardAtBlock = async (network: StacksNetwork, blockHeight: number) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'get-reward-at-block-read',
    functionArgs: [uintCV(blockHeight)],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  return json;
};

export const askToJoin = async (
  btcAddressVersion: Uint8Array,
  btcAddressHashbytesUintArray: Uint8Array,
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'ask-to-join',
    functionArgs: [
      tupleCV({ version: bufferCV(btcAddressVersion), hashbytes: bufferCV(btcAddressHashbytesUintArray) }),
    ],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  console.log(tx);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const votePositive = async (
  minerAddress: string,
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'vote-positive-join-request',
    functionArgs: [principalCV(minerAddress)],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const tryEnterPool = async (
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'try-enter-pool',
    functionArgs: [],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const addPendingMinersToPool = async (
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'add-pending-miners-to-pool',
    functionArgs: [],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const distributeRewards = async (
  blockHeight: number,
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'mining-pool-5-blocks',
    functionName: 'reward-distribution',
    functionArgs: [uintCV(blockHeight)],
    fee,
    nonce,
    network,
    anchorMode: AnchorMode.OnChainOnly,
    postConditionMode: PostConditionMode.Allow,
    senderKey: account.secretKey,
  };
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};
