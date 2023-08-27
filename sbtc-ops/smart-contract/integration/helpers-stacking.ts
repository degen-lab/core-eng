import { Accounts, Contracts } from './constants-stacking';
import {
  StacksBlockMetadata,
  StacksChainUpdate,
  DevnetNetworkOrchestrator,
  StacksTransactionMetadata,
  getIsolatedNetworkConfigUsingNetworkId,
} from '@hirosystems/stacks-devnet-js';
import { StacksNetwork, StacksTestnet } from '@stacks/network';
import {
  AnchorMode,
  broadcastTransaction,
  bufferCV,
  contractPrincipalCV,
  falseCV,
  getNonce,
  makeContractCall,
  PostConditionMode,
  stringAsciiCV,
  trueCV,
  tupleCV,
  TxBroadcastResult,
  uintCV,
  callReadOnlyFunction,
  cvToJSON,
  principalCV,
  cvToHex,
  hexToCV,
  ClarityType,
  SomeCV,
  TupleCV,
  OptionalCV,
  PrincipalCV,
  UIntCV,
  cvToString,
  ClarityValue,
} from '@stacks/transactions';
import { Constants } from './constants-stacking';

import { decodeBtcAddress } from '@stacks/stacking';
import { toBytes } from '@stacks/common';
import { expect } from 'vitest';
import { mainContract } from './contracts';
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
  epoch_2_2: number;
  epoch_2_3: number;
  epoch_2_4: number;
}

export const DEFAULT_EPOCH_TIMELINE = {
  epoch_2_0: Constants.DEVNET_DEFAULT_EPOCH_2_0,
  epoch_2_05: Constants.DEVNET_DEFAULT_EPOCH_2_05,
  epoch_2_1: Constants.DEVNET_DEFAULT_EPOCH_2_1,
  pox_2_activation: Constants.DEVNET_DEFAULT_POX_2_ACTIVATION,
  epoch_2_2: 106,
  epoch_2_3: 108,
  epoch_2_4: 112,
};

export const FAST_FORWARD_TO_EPOCH_2_4 = {
  epoch_2_0: 100,
  epoch_2_05: 102,
  epoch_2_1: 104,
  pox_2_activation: 105,
  epoch_2_2: 106,
  epoch_2_3: 108,
  epoch_2_4: 112,
};

const delay = () => new Promise((resolve) => setTimeout(resolve, 3000));

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
      stacks_api_url: 'hirosystems/stacks-blockchain-api:latest',
      stacks_explorer_url: 'hirosystems/explorer:latest',
    },
  };
  let consolidatedConfig = getIsolatedNetworkConfigUsingNetworkId(networkId, config);
  console.log('isolated Config!', consolidatedConfig);
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

export async function asyncExpectCurrentCycleIdToBe(cycleId: number, network: StacksTestnet) {
  let poxInfo = await getPoxInfo(network);
  console.log('PoxInfo', poxInfo);
  expect(poxInfo.current_cycle.id).toBe(cycleId);
}

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

export const getPoxInfo = async (network: StacksNetwork, retry?: number): Promise<any> => {
  let retryCountdown = retry ? retry : 20;
  if (retryCountdown == 0) return Promise.reject();
  try {
    let response = await fetch(network.getPoxInfoUrl(), {});
    let poxInfo = await response.json();
    return poxInfo;
  } catch (e) {
    await delay();
    return await getPoxInfo(network, retryCountdown - 1);
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

export const getBitcoinHeightOfNextRewardPhase = async (network: StacksNetwork, retry?: number): Promise<number> => {
  let response = await getPoxInfo(network, retry);
  return response.next_cycle.reward_phase_start_block_height;
};

export const getBitcoinHeightOfNextPreparePhase = async (network: StacksNetwork, retry?: number): Promise<number> => {
  let response = await getPoxInfo(network, retry);
  return response.next_cycle.prepare_phase_start_block_height;
};

export const waitForNextPreparePhase = async (
  network: StacksNetwork,
  orchestrator: DevnetNetworkOrchestrator,
  offset?: number
): Promise<StacksChainUpdate> => {
  var height = await getBitcoinHeightOfNextPreparePhase(network);
  if (offset) {
    height = height + offset;
  }
  return await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(height);
};

export const waitForRewardCycleId = async (
  network: StacksNetwork,
  orchestrator: DevnetNetworkOrchestrator,
  id: number,
  offset?: number
): Promise<StacksChainUpdate> => {
  let response = await getPoxInfo(network);
  let height = response.first_burnchain_block_height + id * response.reward_cycle_length;
  if (offset) {
    height = height + offset;
  }
  return await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(height);
};

export const waitForStacks24 = async (orchestrator, timeline) => {
  // Wait for 2.4 to go live
  await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(timeline.epoch_2_4);
};

export const waitForNextRewardPhase = async (
  network: StacksNetwork,
  orchestrator: DevnetNetworkOrchestrator,
  offset?: number
): Promise<StacksChainUpdate> => {
  var height = await getBitcoinHeightOfNextRewardPhase(network);
  if (offset) {
    height = height + offset;
  }
  return await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(height);
};

export const expectAccountToBe = async (network: StacksNetwork, address: string, account: number, locked: number) => {
  let wallet = await getAccount(network, address);
  expect(wallet.balance).toBe(BigInt(account));
  expect(wallet.locked).toBe(BigInt(locked));
};

export async function handleContractCall({ txOptions, network }: { txOptions: any; network: StacksNetwork }) {
  // @ts-ignore
  const tx = await makeContractCall(txOptions);
  // Broadcast transaction to our Devnet stacks node
  const response = await broadcastTransaction(tx, network);
  expect(response.error, 'tx failed\n' + response.reason + ' ' + JSON.stringify(response.reason_data)).toBeUndefined();
  return response;
}

export const broadcastStackSTX = async (
  poxVersion: number,
  network: StacksNetwork,
  amount: number,
  account: Account,
  blockHeight: number,
  cycles: number,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const { version, data } = decodeBtcAddress(account.btcAddress);
  // @ts-ignore
  const address = {
    version: bufferCV(toBytes(new Uint8Array([version.valueOf()]))),
    hashbytes: bufferCV(data),
  };

  const txOptions = {
    contractAddress: Contracts.POX_1.address,
    contractName: poxVersion == 1 ? Contracts.POX_1.name : Contracts.POX_2.name,
    functionName: 'stack-stx',
    functionArgs: [uintCV(amount), tupleCV(address), uintCV(blockHeight), uintCV(cycles)],
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
  const response = await broadcastTransaction(tx, network);
  return response;
};

export const createVault = async (
  collateralAmount: number,
  usda: number,
  network: StacksNetwork,
  account: Account,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'arkadiko-freddie-v1-1',
    functionName: 'collateralize-and-mint',
    functionArgs: [
      uintCV(collateralAmount * 1000000),
      uintCV(usda * 1000000),
      tupleCV({
        'stack-pox': trueCV(),
        'auto-payoff': falseCV(),
      }),
      stringAsciiCV('STX-A'),
      contractPrincipalCV(Accounts.DEPLOYER.stxAddress, 'arkadiko-stx-reserve-v1-1'),
      contractPrincipalCV(Accounts.DEPLOYER.stxAddress, 'arkadiko-token'),
      contractPrincipalCV(Accounts.DEPLOYER.stxAddress, 'arkadiko-collateral-types-v3-1'),
      contractPrincipalCV(Accounts.DEPLOYER.stxAddress, 'arkadiko-oracle-v1-1'),
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
  // Broadcast transaction to our Devnet stacks node
  const result = await broadcastTransaction(tx, network);
  return result;
};

export const initiateStacking = async (
  network: StacksNetwork,
  account: Account,
  blockHeight: number,
  cycles: number,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const { version, data } = decodeBtcAddress(account.btcAddress);
  // @ts-ignore
  const address = {
    version: bufferCV(toBytes(new Uint8Array([version.valueOf()]))),
    hashbytes: bufferCV(data),
  };

  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'arkadiko-stacker-v2-1',
    functionName: 'initiate-stacking',
    functionArgs: [tupleCV(address), uintCV(blockHeight), uintCV(cycles)],
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

export const stackIncrease = async (
  network: StacksNetwork,
  account: Account,
  stackerName: string,
  fee: number,
  nonce: number
): Promise<TxBroadcastResult> => {
  const txOptions = {
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'arkadiko-stacker-v2-1',
    functionName: 'stack-increase',
    functionArgs: [stringAsciiCV(stackerName)],
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

export const getStackerInfo = async (network: StacksNetwork) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'arkadiko-stacker-v2-1',
    functionName: 'get-stacker-info',
    functionArgs: [],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  console.log('STACKER INFO JSON:', json);

  return json;
};

export const getScLockedBalance = async (network: StacksNetwork) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'stacking-pool-test',
    functionName: 'get-SC-locked-balance',
    functionArgs: [],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  console.log('SC Locked Balance:', json);

  return json;
};

export const getStackerWeight = async (network: StacksNetwork, stacker: string, rewardCycle: number) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: mainContract.address,
    contractName: 'stacking-pool-test',
    functionName: 'get-stacker-weight',
    functionArgs: [principalCV(stacker), uintCV(rewardCycle)],
    senderAddress: mainContract.address,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  console.log(`Stacker ${stacker} weight:`, json.value ? json.value.value : json);

  return json.value ? json.value.value : json;
};

export const getBlockPoxAddresses = async (network: StacksNetwork, stacker: string, burnHeight: number) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'stacking-pool-test',
    functionName: 'get-block-rewards',
    functionArgs: [uintCV(burnHeight)],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  if (json.value.value) {
    console.log(
      `Block ${burnHeight} pox addr 1`,
      json.value.value.value.addrs.value[0] //.value.hashbytes
    );
    console.log(
      `Block ${burnHeight} pox addr 2`,
      json.value.value.value.addrs.value[1] //.value.hashbytes
    );
  }

  return json;
};

export const getBlockRewards = async (network: StacksNetwork, stacker: string, burnHeight: number) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'stacking-pool-test',
    functionName: 'get-block-rewards',
    functionArgs: [uintCV(burnHeight)],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  console.log(`Block ${burnHeight} rewards`, json.value.value);

  return json;
};

export const getPoolMembers = async (network: StacksNetwork) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: Accounts.DEPLOYER.stxAddress,
    contractName: 'stacking-pool-test',
    functionName: 'get-pool-members',
    functionArgs: [],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  console.log(`Stackers list:`, json);

  return json;
};

export const getCheckDelegation = async (network: StacksNetwork, stacker: string) => {
  const supplyCall = await callReadOnlyFunction({
    contractAddress: 'ST000000000000000000002AMW42H',
    contractName: 'pox-3',
    functionName: 'get-check-delegation',
    functionArgs: [principalCV(stacker)],
    senderAddress: Accounts.DEPLOYER.stxAddress,
    network: network,
  });
  const json = cvToJSON(supplyCall);
  // console.log('Stacker delegation json', json);
  // if (json.value) console.log(`Stacker delegation info:`, json.value.value);

  return json;
};

export const readRewardCyclePoxAddressList = async (network: StacksNetwork, cycleId: number) => {
  const url = network.getMapEntryUrl('ST000000000000000000002AMW42H', 'pox-3', 'reward-cycle-pox-address-list-len');
  const cycleIdValue = uintCV(cycleId);
  const keyValue = tupleCV({
    'reward-cycle': cycleIdValue,
  });
  const response = await network.fetchFn(url, {
    method: 'POST',
    body: JSON.stringify(cvToHex(keyValue)),
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    const msg = await response.text().catch(() => '');
    throw new Error(
      `Error calling read-only function. Response ${response.status}: ${response.statusText}. Attempted to fetch ${url} and failed with the message: "${msg}"`
    );
  }
  let lengthJson = await response.json();
  let lengthSome = hexToCV(lengthJson.data) as OptionalCV<TupleCV>;
  if (lengthSome.type === ClarityType.OptionalNone) {
    return null;
  }
  let lengthUint = lengthSome.value.data['len'] as UIntCV;
  let length = Number(lengthUint.value);

  let poxAddrInfoList = [];
  for (let i = 0; i < length; i++) {
    let poxAddressInfo = (await readRewardCyclePoxAddressListAtIndex(network, cycleId, i)) as Record<
      string,
      ClarityValue
    >;
    poxAddrInfoList.push(poxAddressInfo);
  }

  return poxAddrInfoList;
};

export const readRewardCyclePoxAddressForAddress = async (network: StacksNetwork, cycleId: number, address: string) => {
  // TODO: There might be a better way to do this using the `stacking-state`
  //       map to get the index
  const url = network.getMapEntryUrl('ST000000000000000000002AMW42H', 'pox-3', 'reward-cycle-pox-address-list-len');
  const cycleIdValue = uintCV(cycleId);
  const keyValue = tupleCV({
    'reward-cycle': cycleIdValue,
  });
  const response = await network.fetchFn(url, {
    method: 'POST',
    body: JSON.stringify(cvToHex(keyValue)),
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    const msg = await response.text().catch(() => '');
    throw new Error(
      `Error calling read-only function. Response ${response.status}: ${response.statusText}. Attempted to fetch ${url} and failed with the message: "${msg}"`
    );
  }
  let lengthJson = await response.json();
  let lengthSome = hexToCV(lengthJson.data) as OptionalCV<TupleCV>;
  if (lengthSome.type === ClarityType.OptionalNone) {
    return null;
  }
  let lengthUint = lengthSome.value.data['len'] as UIntCV;
  let length = Number(lengthUint.value);

  for (let i = 0; i < length; i++) {
    let poxAddressInfo = await readRewardCyclePoxAddressListAtIndex(network, cycleId, i);
    if (poxAddressInfo?.['stacker']?.type === ClarityType.OptionalNone) {
      continue;
    } else if (poxAddressInfo?.['stacker']?.type === ClarityType.OptionalSome) {
      let stackerSome = poxAddressInfo['stacker'] as SomeCV<PrincipalCV>;
      if (cvToString(stackerSome.value) === address) {
        return poxAddressInfo;
      }
    }
  }

  return null;
};

export type RewardCyclePoxAddressMapEntry = {
  'total-ustx': UIntCV;
  stacker: OptionalCV<PrincipalCV>;
};

export const readRewardCyclePoxAddressListAtIndex = async (
  network: StacksNetwork,
  cycleId: number,
  index: number
): Promise<RewardCyclePoxAddressMapEntry | null | undefined> => {
  const url = network.getMapEntryUrl('ST000000000000000000002AMW42H', 'pox-3', 'reward-cycle-pox-address-list');
  const cycleIdValue = uintCV(cycleId);
  const indexValue = uintCV(index);
  const keyValue = tupleCV({
    'reward-cycle': cycleIdValue,
    index: indexValue,
  });
  const response = await network.fetchFn(url, {
    method: 'POST',
    body: JSON.stringify(cvToHex(keyValue)),
    headers: {
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    const msg = await response.text().catch(() => '');
    throw new Error(
      `Error calling read-only function. Response ${response.status}: ${response.statusText}. Attempted to fetch ${url} and failed with the message: "${msg}"`
    );
  }
  let poxAddrInfoJson = await response.json();
  let cv = hexToCV(poxAddrInfoJson.data);
  if (cv.type === ClarityType.OptionalSome) {
    let someCV = cv as SomeCV<TupleCV>;
    const tupleData = someCV.value.data as RewardCyclePoxAddressMapEntry;
    return tupleData;
  } else if (cv.type === ClarityType.OptionalNone) {
    return null;
  }
};
