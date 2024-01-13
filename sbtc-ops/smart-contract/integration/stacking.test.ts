import {
  buildDevnetNetworkOrchestrator,
  FAST_FORWARD_TO_EPOCH_2_4,
  getAccount,
  getBlockPoxAddresses,
  getBlockRewards,
  getCheckDelegation,
  getNetworkIdFromEnv,
  getScLockedBalance,
  getStackerWeight,
  getPoxInfo,
  readRewardCyclePoxAddressForAddress,
  readRewardCyclePoxAddressListAtIndex,
  waitForRewardCycleId,
  waitForNextPreparePhase,
  waitForNextRewardPhase,
} from './helpers-stacking';
import { Accounts, Contracts, Constants } from './constants-stacking';
import { StacksTestnet } from '@stacks/network';
import { DevnetNetworkOrchestrator, StacksBlockMetadata } from '@hirosystems/stacks-devnet-js';
import { broadcastAllowContractCallerContracCall } from './allowContractCaller';
import { afterAll, beforeAll, describe, it } from 'vitest';
import {
  broadcastDelegateStackStx,
  broadcastDelegateStackStxMany,
  broadcastDelegateStx,
  broadcastDepositStxOwner,
  broadcastJoinPool,
  broadcastReserveStxOwner,
  broadcastRewardDistribution,
  broadcastUpdateScBalances,
} from './helper-fp';
import { expect } from 'chai';
import { uintCV } from '@stacks/transactions';
import { mainContract } from './contracts';

describe('testing stacking under epoch 2.4', () => {
  let orchestrator: DevnetNetworkOrchestrator;
  let timeline = FAST_FORWARD_TO_EPOCH_2_4;

  beforeAll(() => {
    orchestrator = buildDevnetNetworkOrchestrator(getNetworkIdFromEnv(), timeline);
    orchestrator.start(12000);
  });

  afterAll(() => {
    orchestrator.terminate();
  });

  it('whole flow many cycles 4 stackers + liquidity provider', async () => {
    console.log('POX-3 test beginning');
    const network = new StacksTestnet({ url: orchestrator.getStacksNodeUrl() });

    let usersList = [Accounts.WALLET_1, Accounts.WALLET_2, Accounts.WALLET_3, Accounts.WALLET_4];

    // Wait for Pox-3 activation

    await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(timeline.epoch_2_4 + 1, 5, true);
    console.log(await getPoxInfo(network));

    // Wait for the contracts to be deployed

    let chainUpdate, txs;
    // chainUpdate = await orchestrator.waitForNextStacksBlock();
    // txs = chainUpdate.new_blocks[0].block.transactions;

    // Deposit STX Liquidity Provider

    await broadcastDepositStxOwner({
      amountUstx: 11_000_000_000,
      nonce: (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce,
      network: network,
      user: Accounts.DEPLOYER,
    });

    // Check the deposit tx

    let depositTxIndex = 0;
    let blockIndex = 0;
    while (depositTxIndex < 1) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        let txMetadata = txs[1].metadata;
        let txData = txMetadata.kind.data;
        let txSC = txData['contract_identifier'];
        let txMethod = txData['method'];
        if (txMethod == 'deposit-stx-liquidity-provider') {
          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('deposit-stx-liquidity-provider');
          expect((txMetadata as any)['success']).toBe(true);
          expect((txMetadata as any)['result']).toBe(`(ok true)`);
          console.log(`Deposit STX Metadata, (block index ${blockIndex}): `, txMetadata);
          depositTxIndex++;
        }
      }
    }

    // Allow pool in Pox-3

    await broadcastAllowContractCallerContracCall({
      network,
      nonce: (await getAccount(network, usersList[0].stxAddress)).nonce,
      senderKey: usersList[0].secretKey,
    });
    await broadcastAllowContractCallerContracCall({
      network,
      nonce: (await getAccount(network, usersList[1].stxAddress)).nonce,
      senderKey: usersList[1].secretKey,
    });
    await broadcastAllowContractCallerContracCall({
      network,
      nonce: (await getAccount(network, usersList[2].stxAddress)).nonce,
      senderKey: usersList[2].secretKey,
    });

    // Check the allow txs

    let allowTxIndex = 0;
    blockIndex = 0;

    while (allowTxIndex < 3) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;

      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          if (txMethod == 'allow-contract-caller') {
            expect(txSC as any).toBe(`${Contracts.POX_3.address}.${Contracts.POX_3.name}`);
            expect(txMethod as any).toBe('allow-contract-caller');
            expect((txMetadata as any)['success']).toBe(true);
            expect((txMetadata as any)['result']).toBe(`(ok true)`);
            allowTxIndex++;
            console.log(`Allow Contract Caller Metadata ${allowTxIndex}, block index ${blockIndex}`, txMetadata);
          }
        }
      }
    }

    // Reserve STX for future rewards Liquidity Provider

    await broadcastReserveStxOwner({
      amountUstx: 11_000_000_000,
      nonce: (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce,
      network: network,
      user: Accounts.DEPLOYER,
    });

    // Check the Reserve STX tx

    for (let i = 0; i <= 15; i++) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;

      if (txs.length > 1) {
        let txMetadata = txs[1].metadata;
        let txData = txMetadata.kind.data;
        let txSC = txData['contract_identifier'];
        let txMethod = txData['method'];

        expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
        expect(txMethod as any).toBe('reserve-funds-future-rewards');
        expect((txMetadata as any)['success']).toBe(true);
        expect((txMetadata as any)['result']).toBe(`(ok true)`);
        console.log(`Reserve STX Metadata, block index ${i + 1}`, txMetadata);
        i = 15;
      }
    }

    // 3 Stackers Join Pool

    for (let i = 0; i < usersList.length - 1; i++) {
      await broadcastJoinPool({
        nonce: (await getAccount(network, usersList[i].stxAddress)).nonce,
        network,
        user: usersList[i],
      });
    }

    // Check the Join txs

    let joinTxIndex = 0;
    blockIndex = 0;
    while (joinTxIndex < 3) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('join-stacking-pool');
          expect((txMetadata as any)['success']).toBe(true);
          expect((txMetadata as any)['result']).toBe(`(ok true)`);
          joinTxIndex++;
          console.log(`Join Pool Metadata ${joinTxIndex}, block index ${blockIndex}`, txMetadata);
        }
      }
    }
    await waitForNextRewardPhase(network, orchestrator);
    chainUpdate = await orchestrator.waitForNextStacksBlock();

    console.log(
      '** BEFORE DELEGATE STX ' +
        (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata).bitcoin_anchor_block_identifier.index
    );

    // 3 Stackers Delegate STX

    await broadcastDelegateStx({
      amountUstx: 125_000_000_000,
      user: usersList[0],
      nonce: (await getAccount(network, usersList[0].stxAddress)).nonce,
      network,
    });

    await broadcastDelegateStx({
      amountUstx: 125_000_000_000,
      user: usersList[1],
      nonce: (await getAccount(network, usersList[1].stxAddress)).nonce,
      network,
    });

    await broadcastDelegateStx({
      amountUstx: 50_286_942_145_278, // pox activation threshold
      user: usersList[2],
      nonce: (await getAccount(network, usersList[2].stxAddress)).nonce,
      network,
    });

    // Check the Delegate txs

    let delegateTxIndex = 0;
    blockIndex = 0;
    while (delegateTxIndex < 3) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('delegate-stx');
          expect((txMetadata as any)['success']).toBe(true);
          // expect((txMetadata as any)['result']).toBe(`(ok true)`);
          delegateTxIndex++;
          console.log(`Delegate STX Metadata ${delegateTxIndex}, block index ${blockIndex}`, txMetadata);
        }
      }
    }

    console.log(
      'The last Delegation block: ' +
        (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata).bitcoin_anchor_block_identifier.index
    );

    // Friedger check table entry:

    let poxInfo = await getPoxInfo(network);
    console.log('pox info CURRENT CYCLE:', poxInfo.current_cycle);
    console.log('pox info NEXT CYCLE:', poxInfo.next_cycle);

    let poxAddrInfo0 = await readRewardCyclePoxAddressForAddress(
      network,
      poxInfo.current_cycle.id,
      Accounts.DEPLOYER.stxAddress
    );

    expect(poxAddrInfo0).toBeNull();

    let poxAddrInfo1 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 0);
    let poxAddrInfo2 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 1);
    let poxAddrInfo;

    if (poxAddrInfo2) {
      poxAddrInfo = poxAddrInfo2;
    } else {
      poxAddrInfo = poxAddrInfo1;
    }

    // Check the Total Stacked STX

    expect(poxAddrInfo?.['total-ustx']).toEqual(uintCV(50_536_942_145_278));

    // Check balances

    let userAccount1 = await getAccount(network, usersList[0].stxAddress);
    console.log('first user:', userAccount1);

    let userAccount2 = await getAccount(network, usersList[1].stxAddress);
    console.log('second user:', userAccount2);

    let userAccount3 = await getAccount(network, usersList[2].stxAddress);
    console.log('third user:', userAccount3);

    let userAccount4 = await getAccount(network, usersList[3].stxAddress);
    console.log('fourth user:', userAccount4);

    // Update SC balances

    await waitForNextPreparePhase(network, orchestrator);
    chainUpdate = await orchestrator.waitForNextStacksBlock();

    await broadcastUpdateScBalances({
      user: Accounts.DEPLOYER,
      nonce: (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce,
      network,
    });

    let updateBalancesTxIndex = 0;
    blockIndex = 0;

    while (updateBalancesTxIndex < 1) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;

      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];
          let updateBalancesBlock = (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata)
            .bitcoin_anchor_block_identifier.index;
          console.log('** UPDATE BALANCES BLOCK ' + updateBalancesBlock);
          if (updateBalancesBlock % 10 > 8 || updateBalancesBlock % 10 < 6)
            console.log('COULD NOT UPDATE BALANCES DUE TO BLOCK DELAYS. PLEASE RESTART TEST!');

          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('update-sc-balances');
          expect((txMetadata as any)['success']).toBe(true);
          expect((txMetadata as any)['result']).toBe(`(ok true)`);
          updateBalancesTxIndex++;
          console.log(`Update Balances Metadata ${updateBalancesTxIndex}, block index ${blockIndex}`, txMetadata);
        }
      }
    }

    let scLockedBalance = await getScLockedBalance(network);
    expect(scLockedBalance.value as any).toBe('50536942145278');

    // Check weights

    let deployerWeight = await getStackerWeight(network, Accounts.DEPLOYER.stxAddress, poxInfo.next_cycle.id);
    expect(deployerWeight as any).toBe('217');

    for (let i = 0; i <= 3; i++) {
      let userWeight = await getStackerWeight(network, usersList[i].stxAddress, poxInfo.next_cycle.id);
      i == 2
        ? expect(userWeight as any).toBe('994836')
        : i < 2
        ? expect(userWeight as any).toBe('2472')
        : expect(userWeight.value as any).toBe(null);
    }

    chainUpdate = await waitForRewardCycleId(network, orchestrator, poxInfo.next_cycle.id);
    let firstBurnBlockPastRewardCycle = (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata)
      .bitcoin_anchor_block_identifier.index;
    console.log('** First burn block to check rewards: ' + firstBurnBlockPastRewardCycle);

    for (let i = 1; i <= 7; i++) chainUpdate = await orchestrator.waitForNextStacksBlock();

    console.log(
      'Burn block when checking rewards: ' +
        (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata).bitcoin_anchor_block_identifier.index
    );

    // Print Pox Addresses for the blocks from 130 to 133

    for (let i = firstBurnBlockPastRewardCycle; i <= firstBurnBlockPastRewardCycle + Constants.REWARD_CYCLE_LENGTH; i++)
      await getBlockPoxAddresses(network, Accounts.DEPLOYER.stxAddress, i);

    // Print Block Rewards for the blocks from 130 to 133

    for (let i = firstBurnBlockPastRewardCycle; i <= firstBurnBlockPastRewardCycle + Constants.REWARD_CYCLE_LENGTH; i++)
      await getBlockRewards(network, Accounts.DEPLOYER.stxAddress, i);

    // Distribute Rewards For The Previously Verified Blocks (even if won or not)

    let userIndex = 0;
    let repeatedUsers = false;
    for (
      let i = firstBurnBlockPastRewardCycle;
      i <= firstBurnBlockPastRewardCycle + Constants.REWARD_CYCLE_LENGTH;
      i++
    ) {
      if (userIndex == 4) {
        userIndex = 0;
        repeatedUsers = true;
      }
      userIndex == 4 ? (userIndex = 0) : (userIndex = userIndex);

      await broadcastRewardDistribution({
        burnBlockHeight: i,
        network,
        user: usersList[userIndex],
        nonce: repeatedUsers
          ? (await getAccount(network, usersList[userIndex].stxAddress)).nonce + 1
          : (
              await getAccount(network, usersList[userIndex].stxAddress)
            ).nonce,
      });
      userIndex++;
    }

    let rewardDistributionTxIndex = 0;
    blockIndex = 0;

    while (rewardDistributionTxIndex < 6) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;

      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];
          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('reward-distribution');
          console.log('Reward distribution result: ', (txMetadata as any)['result']);
          rewardDistributionTxIndex++;
          console.log(
            `Reward Distribution Metadata ${rewardDistributionTxIndex}, block index ${blockIndex}`,
            txMetadata
          );
        }
      }
    }

    // Check Balances

    console.log('deployer:', await getAccount(network, Accounts.DEPLOYER.stxAddress));
    console.log('first user:', await getAccount(network, usersList[0].stxAddress));
    console.log('second user:', await getAccount(network, usersList[1].stxAddress));
    console.log('third user:', await getAccount(network, usersList[2].stxAddress));
    console.log('fourth user:', await getAccount(network, usersList[3].stxAddress));

    await getCheckDelegation(network, usersList[0].stxAddress);
    await getCheckDelegation(network, usersList[1].stxAddress);
    await getCheckDelegation(network, usersList[2].stxAddress);
    await getCheckDelegation(network, usersList[3].stxAddress);

    // Delegate Stack Stx (must happen on the second half of the reward phase)

    poxInfo = await getPoxInfo(network);
    chainUpdate = await waitForRewardCycleId(network, orchestrator, poxInfo.next_cycle.id);

    for (let i = 1; i <= 6; i++) chainUpdate = await orchestrator.waitForNextStacksBlock();

    console.log(
      '** BEFORE DELEGATE STACK ' +
        (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata).bitcoin_anchor_block_identifier.index
    );

    let delegateStack1 = await broadcastDelegateStackStx(
      usersList[0].stxAddress,
      network,
      Accounts.DEPLOYER,
      1000,
      (
        await getAccount(network, Accounts.DEPLOYER.stxAddress)
      ).nonce
    );
    let delegateStack2 = await broadcastDelegateStackStx(
      usersList[1].stxAddress,
      network,
      Accounts.DEPLOYER,
      1000,
      (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce + 1
    );
    let delegateStack3 = await broadcastDelegateStackStx(
      usersList[2].stxAddress,
      network,
      Accounts.DEPLOYER,
      1000,
      (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce + 2
    );

    let delegateStackTxIndex = 0;
    blockIndex = 0;
    while (delegateStackTxIndex < 3) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('delegate-stack-stx');
          console.log(`Delegate Stack STX Metadata ${delegateStackTxIndex}, block index ${blockIndex}`, txMetadata);

          expect((txMetadata as any)['success']).toBe(true);
          delegateStackTxIndex++;
        }
      }
    }

    // Friedger check table entry:

    poxInfo = await getPoxInfo(network);
    console.log('pox info CURRENT CYCLE:', poxInfo.current_cycle);
    console.log('pox info NEXT CYCLE:', poxInfo.next_cycle);

    poxAddrInfo0 = await readRewardCyclePoxAddressForAddress(
      network,
      poxInfo.current_cycle.id,
      Accounts.DEPLOYER.stxAddress
    );

    expect(poxAddrInfo0).toBeNull();

    poxAddrInfo1 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 0);
    poxAddrInfo2 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 1);
    poxAddrInfo;

    if (poxAddrInfo2) {
      poxAddrInfo = poxAddrInfo2;
    } else {
      poxAddrInfo = poxAddrInfo1;
    }

    // Check total stacked ustx

    expect(poxAddrInfo?.['total-ustx']).toEqual(uintCV(50_536_942_145_278));

    // Check balances

    userAccount1 = await getAccount(network, usersList[0].stxAddress);
    console.log('first user:', userAccount1);

    userAccount2 = await getAccount(network, usersList[1].stxAddress);
    console.log('second user:', userAccount2);

    userAccount3 = await getAccount(network, usersList[2].stxAddress);
    console.log('third user:', userAccount3);

    userAccount4 = await getAccount(network, usersList[3].stxAddress);
    console.log('fourth user:', userAccount4);

    await orchestrator.waitForNextStacksBlock();

    // Delegate Stack Stx Many for all stackers in the list
    // can be done after the reward cycle is half through, so wait for the next prepare phase
    // next prepare phase -> 146 > 145

    poxInfo = await getPoxInfo(network);
    chainUpdate = await waitForRewardCycleId(network, orchestrator, poxInfo.next_cycle.id);

    for (let i = 1; i <= 6; i++) chainUpdate = await orchestrator.waitForNextStacksBlock();

    let usersAddressesList = [];
    usersList.forEach((user) => {
      if (user && user.stxAddress) usersAddressesList.push(user.stxAddress);
    });

    console.log(
      '** BEFORE DELEGATE STACK MANY ' +
        (chainUpdate.new_blocks[0].block.metadata as StacksBlockMetadata).bitcoin_anchor_block_identifier.index
    );

    let delegateStackMany1 = await broadcastDelegateStackStxMany({
      stackersLockList: usersAddressesList,
      network: network,
      nonce: (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce,
      user: Accounts.DEPLOYER,
    });

    let delegateStackManyTxIndex = 0;
    blockIndex = 0;
    while (delegateStackManyTxIndex < 1) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          expect(txSC as any).toBe(`${mainContract.address}.${mainContract.name}`);
          expect(txMethod as any).toBe('delegate-stack-stx-many');
          // expect((txMetadata as any)['result']).toBe('(ok ((ok false) (ok true) (ok true) (err u9000)))');
          console.log('Delegate Stack Many Result: ', txMetadata['result']);
          console.log(`Delegate Stack STX Metadata ${delegateStackManyTxIndex}, block index ${blockIndex}`, txMetadata);

          expect((txMetadata as any)['success']).toBe(true);
          delegateStackManyTxIndex++;
        }
      }
    }

    // Friedger check table entry:

    poxInfo = await getPoxInfo(network);
    console.log('pox info CURRENT CYCLE:', poxInfo.current_cycle);
    console.log('pox info NEXT CYCLE:', poxInfo.next_cycle);

    poxAddrInfo0 = await readRewardCyclePoxAddressForAddress(
      network,
      poxInfo.current_cycle.id,
      Accounts.DEPLOYER.stxAddress
    );

    expect(poxAddrInfo0).toBeNull();

    poxAddrInfo1 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 0);
    poxAddrInfo2 = await readRewardCyclePoxAddressListAtIndex(network, await poxInfo.next_cycle.id, 1);
    poxAddrInfo;

    if (poxAddrInfo2) {
      poxAddrInfo = poxAddrInfo2;
    } else {
      poxAddrInfo = poxAddrInfo1;
    }

    // Check the Total Stacked STX

    expect(poxAddrInfo?.['total-ustx']).toEqual(uintCV(50_536_942_145_278));
  });
});
