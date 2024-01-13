// deposit stx deployer

/// get balance deployer
// everything happens on stacks_2.1
// pox_2
import {
  balanceDepositSTX,
  getBalanceSTX,
  buildDevnetNetworkOrchestrator,
  DEFAULT_EPOCH_TIMELINE,
  getAccount,
  getNetworkIdFromEnv,
  getRewardAtBlock,
  votePositive,
  tryEnterPool,
  addPendingMinersToPool,
  askToJoin,
  distributeRewards,
} from './helpers';
import { StacksTestnet } from '@stacks/network';
import { DevnetNetworkOrchestrator } from '@hirosystems/stacks-devnet-js';
import { FAST_FORWARD_TO_EPOCH_2_4 } from './helpers-stacking';
import { mainContract } from './contracts';
import { crypto } from 'bitcoinjs-lib';

describe('testing depositing balance stx', () => {
  let orchestrator: DevnetNetworkOrchestrator;
  let timeline = FAST_FORWARD_TO_EPOCH_2_4;

  beforeAll(() => {
    orchestrator = buildDevnetNetworkOrchestrator(getNetworkIdFromEnv());
    orchestrator.start(1000);
  });

  afterAll(() => {
    orchestrator.terminate();
  });

  it('test whole flow with initiate, deposit STX and see balance', async () => {
    const Accounts = {
      DEPLOYER: {
        stxAddress: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM',
        btcAddress: 'mqVnk6NPRdhntvfm4hh9vvjiRkFDUuSYsH',
        secretKey: '753b7cc01a1a2e86221266a154af739463fce51219d97e4f856cd7200c3bd2a601',
      },
      WALLET_1: {
        stxAddress: 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5',
        btcAddress: 'mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC',
        secretKey: '7287ba251d44a4d3fd9276c88ce34c5c52a038955511cccaf77e61068649c17801',
      },
      WALLET_2: {
        stxAddress: 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG',
        btcAddress: 'muYdXKmX9bByAueDe6KFfHd5Ff1gdN9ErG',
        secretKey: '530d9f61984c888536871c6573073bdfc0058896dc1adfe9a6a10dfacadc209101',
      },
      WALLET_3: {
        stxAddress: 'ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC',
        btcAddress: 'mvZtbibDAAA3WLpY7zXXFqRa3T4XSknBX7',
        secretKey: 'd655b2523bcd65e34889725c73064feb17ceb796831c0e111ba1a552b0f31b3901',
      },
      WALLET_4: {
        stxAddress: 'ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND',
        btcAddress: 'mg1C76bNTutiCDV3t9nWhZs3Dc8LzUufj8',
        secretKey: 'f9d7206a47f14d2870c163ebab4bf3e70d18f5d14ce1031f3902fbbc894fe4c701',
      },
      WALLET_5: {
        stxAddress: 'ST2REHHS5J3CERCRBEPMGH7921Q6PYKAADT7JP2VB',
        btcAddress: 'mweN5WVqadScHdA81aATSdcVr4B6dNokqx',
        secretKey: '3eccc5dac8056590432db6a35d52b9896876a3d5cbdea53b72400bc9c2099fe801',
      },
      WALLET_6: {
        stxAddress: 'ST3AM1A56AK2C1XAFJ4115ZSV26EB49BVQ10MGCS0',
        btcAddress: 'mzxXgV6e4BZSsz8zVHm3TmqbECt7mbuErt',
        secretKey: '7036b29cb5e235e5fd9b09ae3e8eec4404e44906814d5d01cbca968a60ed4bfb01',
      },
      WALLET_7: {
        stxAddress: 'ST3PF13W7Z0RRM42A8VZRVFQ75SV1K26RXEP8YGKJ',
        btcAddress: 'n37mwmru2oaVosgfuvzBwgV2ysCQRrLko7',
        secretKey: 'b463f0df6c05d2f156393eee73f8016c5372caa0e9e29a901bb7171d90dc4f1401',
      },
      WALLET_8: {
        stxAddress: 'ST3NBRSFKX28FQ2ZJ1MAKX58HKHSDGNV5N7R21XCP',
        btcAddress: 'n2v875jbJ4RjBnTjgbfikDfnwsDV5iUByw',
        secretKey: '6a1a754ba863d7bab14adbbc3f8ebb090af9e871ace621d3e5ab634e1422885e01',
      },
      WALLET_9: {
        stxAddress: 'ST2RF0D3GJR7ZX63PQ9WXVAPNG6EJCCR97NTVFGES',
        btcAddress: 'mweWyYM2SdmaZRMZneysa2XtsP1o6wFWKC',
        secretKey: '2b2c2dd7ae64aa7880042a443f5a1e1a1b575cc0ef16d6c7e3bb8a9ce08bfe1d01',
      },
      WALLET_10: {
        stxAddress: 'ST3CD3T03P3Z8RMAYR7S6BZQ65QNYS9W18QHHJ4KM',
        btcAddress: 'n1HPi2zoJZzwjAFgGrYckLxPnBsR2m7ViD',
        secretKey: '47599fb4a8cfb07e2db61484c81459db81a5480e770b0de8cbe8de960834bf1701',
      },
    };
    const network = new StacksTestnet({ url: orchestrator.getStacksNodeUrl() });
    const fee = 1000;

    const btcAddressVersionUintArray = Uint8Array.from(Buffer.from('00', 'hex'));
    const publicKeyHex = '02e8f7dc91e49a577ce9ea8989c7184aea8886fe5250f02120dc6f98e3619679b0';
    const publicKey = Buffer.from(publicKeyHex, 'hex');
    const pKhash160 = crypto.hash160(publicKey);
    const btcHashBuffer = pKhash160;
    const btcUintArray = Uint8Array.from(btcHashBuffer);

    // Advance to make sure PoX-3 is activated

    await orchestrator.waitForStacksBlockAnchoredOnBitcoinBlockOfHeight(timeline.epoch_2_4 + 1, 5, true);
    // get nonces
    let nonceWallets = {};
    for (let i = 6; i < 10; i++) {
      nonceWallets[i] = (await getAccount(network, Accounts[`WALLET_${i}`].stxAddress)).nonce;
    }

    // ask to join with 4 participants

    for (let i = 6; i < 10; i++) {
      let responseawait = await askToJoin(
        btcAddressVersionUintArray,
        btcUintArray,
        network,
        Accounts[`WALLET_${i}`],
        fee,
        nonceWallets[i]
      );
      nonceWallets[i] += 1;
    }
    let chainUpdate, txs;

    let askToJoinTxIndex = 0;
    let blockIndex = 0;
    while (askToJoinTxIndex < 4) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          if (txSC === `${mainContract.address}.mining-pool-5-blocks` && txMethod === `ask-to-join`) {
            expect((txMetadata as any)['result']).toBe('(ok true)');
            expect((txMetadata as any)['success']).toBe(true);
            askToJoinTxIndex++;
          }
        }
      }
    }

    // deployer vote positive for joining each participant

    let nonceDeployer = (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce;
    for (let i = 6; i < 10; i++) {
      let responseFor = await votePositive(
        Accounts[`WALLET_${i}`].stxAddress,
        network,
        Accounts.DEPLOYER,
        fee,
        nonceDeployer
      );
      nonceDeployer += 1;
      expect(responseFor.error).toBeUndefined();
    }

    let votePositiveJoinIndex = 0;
    blockIndex = 0;

    while (votePositiveJoinIndex < 4) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          if (txSC === `${mainContract.address}.mining-pool-5-blocks` && txMethod === `vote-positive-join-request`) {
            expect((txMetadata as any)['result']).toBe('(ok true)');
            expect((txMetadata as any)['success']).toBe(true);
            votePositiveJoinIndex++;
          }
        }
      }
    }

    // try-enter-pool

    for (let i = 6; i < 10; i++) {
      let responseFor = await tryEnterPool(network, Accounts[`WALLET_${i}`], fee, nonceWallets[i]);
      nonceWallets[i] += 1;
      expect(responseFor.error).toBeUndefined();
    }

    let tryEnterPoolIndex = 0;
    blockIndex = 0;

    while (tryEnterPoolIndex < 4) {
      chainUpdate = await orchestrator.waitForNextStacksBlock();
      txs = chainUpdate.new_blocks[0].block.transactions;
      blockIndex++;
      if (txs.length > 1) {
        for (let i = 1; i <= txs.length - 1; i++) {
          let txMetadata = txs[i].metadata;
          let txData = txMetadata.kind.data;
          let txSC = txData['contract_identifier'];
          let txMethod = txData['method'];

          if (txSC === `${mainContract.address}.mining-pool-5-blocks` && txMethod === `try-enter-pool`) {
            expect((txMetadata as any)['result']).toBe('(ok true)');
            expect((txMetadata as any)['success']).toBe(true);
            tryEnterPoolIndex++;
          }
        }
      }
    }

    let current_block_height = chainUpdate.new_blocks[0].block.block_identifier.index;
    current_block_height += 6;
    let reward_block_height = current_block_height + 1;
    await orchestrator.waitForStacksBlockOfHeight(current_block_height);

    // add pending miners to pool

    let response = await addPendingMinersToPool(network, Accounts.DEPLOYER, fee, nonceDeployer);
    nonceDeployer += 1;
    expect(response.error).toBeUndefined();

    chainUpdate = await orchestrator.waitForNextStacksBlock();
    let metadata = chainUpdate.new_blocks[0].block.transactions[1].metadata;
    expect((metadata as any)['success']).toBe(true);
    expect((metadata as any)['result']).toBe('(ok true)');

    // now 5 participants in mining pool
    // Check balance for particiapnts

    current_block_height = chainUpdate.new_blocks[0].block.block_identifier.index;
    current_block_height += 110;
    await orchestrator.waitForStacksBlockOfHeight(current_block_height);
    let info = await getRewardAtBlock(network, 15);
    console.log('rewards at block 15:');
    console.log(info.value.claimer.value);
    console.log(info.value.reward.value);
    info = await getRewardAtBlock(network, reward_block_height); // 27
    console.log(info.value.reward.value);
    let distributedAmount = 1000301000;

    // 1 000 301 000
    // 3 000 900 000
    // // we are at block 125+

    nonceDeployer = (await getAccount(network, Accounts.DEPLOYER.stxAddress)).nonce;
    distributeRewards(reward_block_height, network, Accounts.DEPLOYER, fee, nonceDeployer);
    nonceDeployer += 1;
    chainUpdate = await orchestrator.waitForNextStacksBlock();
    // 100030100 / 5 = 200060200
    info = await getRewardAtBlock(network, 122); // 27
    console.log('rewards at block 122 principal: ');
    console.log(info.value.claimer.value);
    for (let i = 6; i < 10; i++) {
      info = await getBalanceSTX(network, Accounts[`WALLET_${i}`].stxAddress);
      console.log('Earned values by ', i);
      console.log(info);
      expect(info.value.value).toBe(`200060200`);
    }
    // see at what block_height miners join pool
    // distribute rewards once - working for block 3 at block 120
    // verify with balances of the miners before and after distributing the rewards
    // distribute rewards same - not working for block 3 at block 120
    // distribute rewards same - working for block 4 at block 120
    // check balances for miners
  });
});
