import { allowContractCaller, disallowContractCaller, getStackerInfo } from './client/pox-2-client.ts';
import {
  delegateStackStx,
  delegateStx,
  fpDelegationAllowContractCaller,
  getUserData,
  joinStackingPool,
  quitStackingPool,
} from './client/stacking-pool-client.ts';
import { Clarinet, Chain, Account, Tx, types } from './deps.ts';
import { Errors, PoxErrors, poxAddrFP, poxAddrPool1, poxAddrPool2 } from './constants.ts';

import { expectPartialStackedByCycle, expectTotalStackedByCycle } from './utils.ts';

Clarinet.test({
  name: "Ensure that user can't delegate without allowance",
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const wallet_1 = accounts.get('wallet_1')!;

    // try without any allowance
    let block = chain.mineBlock([delegateStx(20_000_000_000_000, wallet_1)]);

    // check delegation calls
    block.receipts[0].result.expectErr().expectUint(Errors.AllowPoolInPox2);
  },
});

Clarinet.test({
  name: 'Ensure liquidity provider can deposit STX into the SC',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;

    // try without any allowance
    let block = chain.mineBlock([
      Tx.contractCall(
        'stacking-pool-test',
        'deposit-stx-liquidity-provider',
        [types.uint(10_000_000_000)],
        deployer.address
      ),
    ]);

    // check delegation calls
    block.receipts[0].result.expectOk().expectBool(true);

    block = chain.mineBlock([Tx.contractCall('stacking-pool-test', 'get-SC-total-balance', [], deployer.address)]);

    block.receipts[0].result.expectUint(10_000_000_000);
  },
});

Clarinet.test({
  name: "Ensure that user can't join the pool without allowance",
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const wallet_1 = accounts.get('wallet_1')!;

    // try without any allowance
    let block = chain.mineBlock([joinStackingPool(wallet_1)]);

    // check error to be returned
    block.receipts[0].result.expectErr().expectUint(Errors.AllowPoolInPox2);
  },
});

Clarinet.test({
  name: 'Ensure that user can join the pool after allowing pool SC in pox-2 contract',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // try without any allowance
    let block = chain.mineBlock([allowContractCaller(mainContract, undefined, wallet_1), joinStackingPool(wallet_1)]);

    // check that both calls above return true
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
  },
});

Clarinet.test({
  name: 'Ensure that user can quit the pool after disallowing pool SC in pox-2 contract',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    let block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_1),
      joinStackingPool(wallet_1),
      disallowContractCaller(mainContract, wallet_1),
      quitStackingPool(wallet_1),
    ]);

    // check that both calls above return true
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    block.receipts[3].result.expectOk().expectBool(true);
  },
});

Clarinet.test({
  name: 'Ensure that user can only delegate from a contract allowing pox-2 and joining the pool',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const mainDelegateStx = (amountUstx: number, user: Account) => {
      return Tx.contractCall('stacking-pool-test', 'delegate-stx', [types.uint(amountUstx)], user.address);
    };
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;
    const wallet_2 = accounts.get('wallet_2')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // try without any allowance
    let block = chain.mineBlock([mainDelegateStx(20_000_000_000, wallet_1)]);
    block.receipts[0].result.expectErr().expectUint(Errors.AllowPoolInPox2);

    // try with pox allowance only
    block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_1),
      mainDelegateStx(20_000_000_000_000, wallet_1),
    ]);

    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectErr().expectUint(Errors.NotInPool);

    // delegate-stx with pox-2 allowance and joining the pool
    block = chain.mineBlock([joinStackingPool(wallet_1), mainDelegateStx(20_000_000_000_000, wallet_1)]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    expectTotalStackedByCycle(1, 0, 20_000_000_000_000, chain, deployer);
  },
});

Clarinet.test({
  name: 'Ensure that user can delegate how much he wants, but can lock only funds he owns',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    const mainDelegateStx = (amountUstx: number, user: Account) => {
      return Tx.contractCall('stacking-pool-test', 'delegate-stx', [types.uint(amountUstx)], user.address);
    };
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;
    const wallet_2 = accounts.get('wallet_2')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // user owns 100_000_000_000_000, delegates 200_000_000_000_000
    let block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_1),
      joinStackingPool(wallet_1),
      mainDelegateStx(200_000_000_000_000, wallet_1),
      getUserData(wallet_1, wallet_1),
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    // 200_000_000_000_000 delegated, 100_000_000_000_000 locked
    block.receipts[3].result.expectSome().expectTuple()['delegated-balance'].expectUint(200_000_000_000_000);
    block.receipts[3].result.expectSome().expectTuple()['locked-balance'].expectUint(100_000_000_000_000);
    // check total to be 100_000_000_000_000 instead of what user has delegated
    expectTotalStackedByCycle(1, 0, 100_000_000_000_000, chain, deployer);

    block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_2),
      joinStackingPool(wallet_2),
      mainDelegateStx(400_000_000_000_000, wallet_2),
      getUserData(wallet_2, wallet_2),
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    // 400_000_000_000_000 delegated, 200_000_000_000_000 locked
    block.receipts[3].result.expectSome().expectTuple()['delegated-balance'].expectUint(400_000_000_000_000);
    block.receipts[3].result.expectSome().expectTuple()['locked-balance'].expectUint(100_000_000_000_000);
    // check total to be 200_000_000_000_000 (100_000_000_000_000 + 100_000_000_000_000 already stacked) instead of what user has delegated
    expectTotalStackedByCycle(1, 0, 200_000_000_000_000, chain, deployer);
  },
});

Clarinet.test({
  name: 'Ensure that user can delegate how much he wants, but it will be locked just when the threshold is met',
  // stx_liq_supply / threshold_25 == 1_000_000_000_000_000 / 20_000_000_000 = 50_000

  async fn(chain: Chain, accounts: Map<string, Account>) {
    const mainDelegateStx = (amountUstx: number, user: Account) => {
      return Tx.contractCall('stacking-pool-test', 'delegate-stx', [types.uint(amountUstx)], user.address);
    };
    const deployer = accounts.get('deployer')!;
    const wallet_1 = accounts.get('wallet_1')!;
    const wallet_2 = accounts.get('wallet_2')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // Allow pool SC in pox-2, join stacking pool and delegate with wallet_1
    let block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_1),
      joinStackingPool(wallet_1),
      mainDelegateStx(1_000_000_000, wallet_1), // < 50_000_000_000 uSTX
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(false); // expect false, commit ignored
    // {err-commit-ignored:11}

    // Check local user data
    block = chain.mineBlock([getUserData(wallet_1, deployer), getUserData(wallet_2, deployer)]);

    // verify delegated-balance==locked-balance==1_000_000_000
    block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(1_000_000_000);
    block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(1_000_000_000);

    // verify is-none - not in stacking pool yet
    block.receipts[1].result.expectNone();

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 0, chain, deployer); // does not commit totally

    // Allow pool SC in pox-2, join stacking pool and delegate with wallet_2
    block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_2),
      joinStackingPool(wallet_2),
      mainDelegateStx(10_000_000_000_000, wallet_2), // > 50_000_000_000 uSTX
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 10_001_000_000_000, chain, deployer); // commits totally the amount wallet_1 + wallet_2 delegated

    block = chain.mineBlock([getUserData(wallet_1, deployer), getUserData(wallet_2, deployer)]);

    // verify delegated-balance==locked-balance== 1_000_000_000
    block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(1_000_000_000);
    block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(1_000_000_000);

    // verify delegated-balance==locked-balance==124_000_000_000
    block.receipts[1].result.expectSome().expectTuple()['delegated-balance'].expectUint(10_000_000_000_000);
    block.receipts[1].result.expectSome().expectTuple()['locked-balance'].expectUint(10_000_000_000_000);
  },
});

// Clarinet.test({
//   name: "Ensure stack is extended",

//   async fn(chain: Chain, accounts: Map<string, Account>) {
//     const mainDelegateStx = (amountUstx: number, user: Account) => {
//       return Tx.contractCall(
//         "stacking-pool",
//         "delegate-stx",
//         [types.uint(amountUstx)],
//         user.address
//       );
//     };
//     const deployer = accounts.get("deployer")!;
//     const wallet_1 = accounts.get("wallet_1")!;
//     const wallet_2 = accounts.get("wallet_2")!;
//     const mainContract = deployer.address + ".stacking-pool";

//     // Allow pool SC in pox-2, join stacking pool and delegate with wallet_1
//     let block = chain.mineBlock([
//       allowContractCaller(mainContract, undefined, wallet_1),
//       joinStackingPool(wallet_1),
//       mainDelegateStx(125_000_000_000, wallet_1),
//     ]);
//     block.receipts[0].result.expectOk().expectBool(true);
//     block.receipts[1].result.expectOk().expectBool(true);
//     block.receipts[2].result.expectOk().expectBool(true);

//     // Check local user data
//     block = chain.mineBlock([getUserData(wallet_1, deployer)]);

//     // verify delegated-balance==locked-balance==125_000_000_000
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["delegated-balance"].expectUint(125_000_000_000);
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["locked-balance"].expectUint(125_000_000_000);
//     expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
//     expectTotalStackedByCycle(1, 0, 125_000_000_000, chain, deployer); // commits totally

//     block = chain.mineBlock([
//       Tx.contractCall(
//         "ST000000000000000000002AMW42H.pox-2",
//         "get-stacker-info",
//         [types.principal(wallet_1.address)],
//         deployer.address
//       ),
//     ]);

//     console.log("STACKER INFO BEFORE:", block.receipts[0].result);

//     for (let i = 1; i <= 4196; i++) {
//       // why only > 4195 works??
//       block = chain.mineBlock([]);
//     }

//     block = chain.mineBlock([
//       Tx.contractCall(
//         "ST000000000000000000002AMW42H.pox-2",
//         "get-stacker-info",
//         [types.principal(wallet_1.address)],
//         deployer.address
//       ),
//     ]);

//     console.log("STACKER INFO AFTER:", block.receipts[0].result);

//     block = chain.mineBlock([mainDelegateStx(125_000_000_000, wallet_1)]);
//     block.receipts[0].result.expectOk().expectBool(true); // expect true, lock funds, extend lock period

//     block = chain.mineBlock([getUserData(wallet_1, deployer)]);

//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["until-burn-ht"].expectSome()
//       .expectUint(6300);
//   },
// });

Clarinet.test({
  name: 'Ensure that commit is ignored when multiple wallets delegate small amounts and total delegated < threshold',
  // stx_liq_supply / threshold_25 == 1_000_000_000_000_000 / 20_000_000_000 = 50_000

  async fn(chain: Chain, accounts: Map<string, Account>) {
    const mainDelegateStx = (amountUstx: number, user: Account) => {
      return Tx.contractCall('stacking-pool-test', 'delegate-stx', [types.uint(amountUstx)], user.address);
    };
    const deployer = accounts.get('deployer')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // Allow pool SC in pox-2, join stacking pool and delegate with first 6 wallets
    let block: any;

    for (let i = 1; i <= 6; i++) {
      let stacker = accounts.get(`wallet_${i}`);
      block = chain.mineBlock([
        allowContractCaller(mainContract, undefined, stacker),
        joinStackingPool(stacker),
        mainDelegateStx(1_000_000_000, stacker), // < 50_000_000_000 uSTX
      ]);
      block.receipts[0].result.expectOk().expectBool(true);
      block.receipts[1].result.expectOk().expectBool(true);
      block.receipts[2].result.expectOk().expectBool(false); // expect false, commit ignored
      // {err-commit-ignored:11}
    }

    for (let i = 1; i <= 6; i++) {
      let stacker = accounts.get(`wallet_${i}`);

      // Check local user data
      block = chain.mineBlock([getUserData(stacker, deployer)]);

      // verify delegated-balance==locked-balance==1_000_000_000
      block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(1_000_000_000);
      block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(1_000_000_000);
    }

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 0, chain, deployer); // does not commit totally
  },
});

Clarinet.test({
  name: 'Ensure that user can delegate how much he wants, but it will be locked just when the threshold is met',

  async fn(chain: Chain, accounts: Map<string, Account>) {
    const mainDelegateStx = (amountUstx: number, user: Account) => {
      return Tx.contractCall('stacking-pool-test', 'delegate-stx', [types.uint(amountUstx)], user.address);
    };
    const deployer = accounts.get('deployer')!;
    const wallet_4 = accounts.get('wallet_4')!;
    const wallet_8 = accounts.get('wallet_8')!;
    const mainContract = deployer.address + '.stacking-pool-test';

    // Allow pool SC in pox-2, join stacking pool and delegate with first 3 wallets
    let block: any;

    for (let i = 1; i <= 3; i++) {
      let stacker = accounts.get(`wallet_${i}`)!;
      block = chain.mineBlock([
        allowContractCaller(mainContract, undefined, stacker),
        joinStackingPool(stacker),
        mainDelegateStx(1_000_000_000, stacker), // < 125_000_000_000 uSTX
      ]);
      block.receipts[0].result.expectOk().expectBool(true);
      block.receipts[1].result.expectOk().expectBool(true);
      block.receipts[2].result.expectOk().expectBool(false); // expect false, commit ignored
      // {err-commit-ignored:11}
    }

    // Check local user data
    for (let i = 1; i <= 3; i++) {
      let stacker = accounts.get(`wallet_${i}`)!;

      // Check local user data
      block = chain.mineBlock([getUserData(stacker, deployer)]);

      // verify delegated-balance==locked-balance==1_000_000_000
      block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(1_000_000_000);
      block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(1_000_000_000);
    }

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 0, chain, deployer); // does not commit totally

    // Allow pool SC in pox-2, join stacking pool and delegate with wallet_4
    block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_4),
      joinStackingPool(wallet_4),
      mainDelegateStx(3_780_000_000_000, wallet_4), // approximate threshold
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    // no print, returns ok true => amount commited

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 3783000000000, chain, deployer); // commits totally the amount wallet_1 + wallet_2 delegated

    for (let i = 5; i <= 7; i++) {
      let stacker = accounts.get(`wallet_${i}`)!;
      block = chain.mineBlock([
        allowContractCaller(mainContract, undefined, stacker),
        joinStackingPool(stacker),
        mainDelegateStx(1_000_000_000, stacker), // < 125_000_000_000 uSTX
      ]);
      block.receipts[0].result.expectOk().expectBool(true);
      block.receipts[1].result.expectOk().expectBool(true);
      block.receipts[2].result.expectOk().expectBool(true); // after threshold is met, everything is commited (increase)
      // threshold was previously met, ok true
    }

    for (let i = 5; i <= 7; i++) {
      let stacker = accounts.get(`wallet_${i}`)!;

      // Check local user data
      block = chain.mineBlock([getUserData(stacker, deployer)]);

      // verify delegated-balance==locked-balance==1_000_000_000
      block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(1_000_000_000);
      block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(1_000_000_000);
    }

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 3786000000000, chain, deployer); // does not commit totally

    // Allow pool SC in pox-2, join stacking pool and delegate with wallet_8
    block = chain.mineBlock([
      allowContractCaller(mainContract, undefined, wallet_8),
      joinStackingPool(wallet_8),
      mainDelegateStx(122_000_000_000, wallet_8), // < 125_000_000_000 uSTX
    ]);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    // no print, returns ok true => amount commited

    expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
    expectTotalStackedByCycle(1, 0, 3908000000000, chain, deployer); // commits totally the amount wallet_1, wallet_2, ... wallet_8 delegated

    block = chain.mineBlock([getUserData(wallet_8, deployer)]);

    // verify delegated-balance==locked-balance==122_000_000_000
    block.receipts[0].result.expectSome().expectTuple()['delegated-balance'].expectUint(122_000_000_000);
    block.receipts[0].result.expectSome().expectTuple()['locked-balance'].expectUint(122_000_000_000);
  },
});

// Clarinet.test({
//   name: "Ensure stack is increased",
//   // stx_liq_supply / threshold_25 == 1_000_000_000_000_000 / 20_000_000_000 = 50_000

//   async fn(chain: Chain, accounts: Map<string, Account>) {
//     const mainDelegateStx = (amountUstx: number, user: Account) => {
//       return Tx.contractCall(
//         "stacking-pool",
//         "delegate-stx",
//         [types.uint(amountUstx)],
//         user.address
//       );
//     };
//     const deployer = accounts.get("deployer")!;
//     const wallet_1 = accounts.get("wallet_1")!;
//     const wallet_2 = accounts.get("wallet_2")!;
//     const mainContract = deployer.address + ".stacking-pool";

//     // Allow pool SC in pox-2, join stacking pool and delegate with wallet_1
//     let block = chain.mineBlock([
//       allowContractCaller(mainContract, undefined, wallet_1),
//       joinStackingPool(wallet_1),
//       mainDelegateStx(125_000_000_000, wallet_1), // min increment stx = 125717355363
//     ]);
//     block.receipts[0].result.expectOk().expectBool(true);
//     block.receipts[1].result.expectOk().expectBool(true);
//     block.receipts[2].result.expectOk().expectBool(true);
//     console.log(block.receipts[2]);

//     // Check local user data
//     block = chain.mineBlock([getUserData(wallet_1, deployer)]);

//     // verify delegated-balance==locked-balance==125_000_000_000
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["delegated-balance"].expectUint(125_000_000_000);
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["locked-balance"].expectUint(125_000_000_000);

//     expectPartialStackedByCycle(poxAddrFP, 1, 0, chain, deployer); // does not commit partially
//     expectTotalStackedByCycle(1, 0, 125_000_000_000, chain, deployer); // commits totally

//     // one cycle (== 2100) blocks pass
//     for (let i = 1; i <= 4200; i++) {
//       block = chain.mineBlock([]); // why 4198??
//     }

//     // increase stacked amount

//     block = chain.mineBlock([delegateStx(126_000_000_000, wallet_1)]);
//     block.receipts[0].result.expectOk().expectBool(true);
//     console.log(block.receipts[0]);

//     // Check local user data
//     block = chain.mineBlock([getUserData(wallet_1, deployer)]);

//     // verify delegated-balance==locked-balance==126_000_000_000 until 2nd reward cycle's end (6300 block height)
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["delegated-balance"].expectUint(126_000_000_000);
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["locked-balance"].expectUint(126_000_000_000);
//     block.receipts[0].result
//       .expectSome()
//       .expectTuple()
//       ["until-burn-ht"].expectSome()
//       .expectUint(6300);
//   },
// });
