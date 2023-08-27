import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v1.6.1/index.ts';
import { assertEquals } from 'https://deno.land/std@0.170.0/testing/asserts.ts';

const ammSwapPoolContract = 'amm-swap-pool-v1-1';
const degenBridgeContract = 'degen-bridge-testnet-v3';
const wrappedBitcoinContract = 'Wrapped-Bitcoin';
const alexVaultContract = 'alex-vault-v1-1';
const bridgeContract = 'bridge-contract';
const createPool = 'create-pool';
const initializeWrappedBitcoinSC = 'initialize';
const swapBridgeStxBtc = 'swap-bridge-stx-btc';
const setMaxInRatio = 'set-max-in-ratio';
const setMaxOutRatio = 'set-max-out-ratio';
const setPoolStartBlock = 'set-start-block';
const getPoolDetails = 'get-pool-details';
const addPrincipalToRole = 'add-principal-to-role';
const setApprovedSC = 'set-approved-contract';
const setApprovedToken = ' set-approved-token';
const bridgeSwapFn = 'swap-helper';
const minterRole = 1;
const mintWrappedBitcoin = 'mint-tokens';
const registerSupplier = 'register-supplier';
const wrappedBitcoinTokenName = 'Wrapped Bitcoin';
const wrappedBitcoinSymbol = 'xBTC';
const wrappedBitcoinDecimals = 8;
const xBtcStxTransferResults = {
  0.1: [417032098765, 406860584161, 397056714663, 387602983362, 378482913165, 369680984952, 361182571504, 352973876697],
};
const StxXbtcTransferResults = {
  100: [286470, 286283, 286097, 285911, 285725, 285539, 285354, 285168],
};

const to_one_8 = (amount) => {
  return amount * 100_000_000;
};
const div_one_8 = (amount) => {
  return amount / 100_000_000;
};

Clarinet.test({
  name: 'Exchanging both STX to xBTC and xBTC to STX with 8 users',
  async fn(chain: Chain, accounts: Map<string, Account>) {
    let deployer = accounts.get('deployer')!;
    let wallet_1 = accounts.get('wallet_1')!;
    let wallet_2 = accounts.get('wallet_2')!;
    let wallet_3 = accounts.get('wallet_3')!;
    let wallet_4 = accounts.get('wallet_4')!;
    let wallet_5 = accounts.get('wallet_5')!;
    let wallet_6 = accounts.get('wallet_6')!;

    // Prerequirements

    // 1. Wrapped-Bitcoin

    // a. Initialize the Wrapped-Bitcoin contract (token-name, symbol, decimals, owner)
    // b. Add minter role for deployer in order to mint
    let block = chain.mineBlock([
      Tx.contractCall(
        wrappedBitcoinContract,
        initializeWrappedBitcoinSC,
        [
          types.ascii(wrappedBitcoinTokenName),
          types.ascii(wrappedBitcoinSymbol),
          types.uint(wrappedBitcoinDecimals),
          types.principal(deployer.address),
        ],
        deployer.address
      ),
      Tx.contractCall(
        wrappedBitcoinContract,
        addPrincipalToRole,
        [types.uint(minterRole), types.principal(deployer.address)],
        deployer.address
      ),
    ]);

    assertEquals(block.receipts.length, 2);
    assertEquals(block.height, 2);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);

    // c. Mint resources in order to create the trading pool
    block = chain.mineBlock([
      Tx.contractCall(
        wrappedBitcoinContract,
        mintWrappedBitcoin,
        [
          types.uint(to_one_8(12)), // 12 BTC
          types.principal(deployer.address),
        ],
        deployer.address
      ),
    ]);

    assertEquals(block.receipts.length, 1);
    assertEquals(block.height, 3);
    block.receipts[0].result.expectOk().expectBool(true);

    for (let i = 1; i <= 4; i++) {
      block = chain.mineBlock([
        Tx.contractCall(
          wrappedBitcoinContract,
          mintWrappedBitcoin,
          [
            types.uint(to_one_8(12)), // 12 BTC
            types.principal(accounts.get(`wallet_${2 * i - 1}`)!.address),
          ],
          deployer.address
        ),
        Tx.contractCall(
          wrappedBitcoinContract,
          mintWrappedBitcoin,
          [
            types.uint(to_one_8(12)), // 12 BTC
            types.principal(accounts.get(`wallet_${2 * i}`)!.address),
          ],
          deployer.address
        ),
      ]);

      assertEquals(block.receipts.length, 2);
      assertEquals(block.height, i + 3);
      block.receipts[0].result.expectOk().expectBool(true);
    }

    // 2. alex-vault-v1-1

    // a. Set Approved Contract: amm-swap-pool-v1-1
    // b. Approve tokens: .wbtc, .wstx
    block = chain.mineBlock([
      Tx.contractCall(
        alexVaultContract,
        setApprovedSC,
        [types.principal(`${deployer.address}.amm-swap-pool-v1-1`), types.bool(true)],
        deployer.address
      ),
      Tx.contractCall(
        alexVaultContract,
        setApprovedToken,
        [types.principal(`${deployer.address}.token-wbtc`), types.bool(true)],
        deployer.address
      ),
      Tx.contractCall(
        alexVaultContract,
        setApprovedToken,
        [types.principal(`${deployer.address}.token-wstx`), types.bool(true)],
        deployer.address
      ),
    ]);

    assertEquals(block.receipts.length, 3);
    assertEquals(block.height, 8);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);

    // 3. amm-swap-pool-v1-1

    // a. Create the xBTC - STX trading pool
    // b. Set Max In Ratio
    // c. Set Max Out Ratio
    // d. Set Start Block
    // e. check pool details to be correct
    block = chain.mineBlock([
      Tx.contractCall(
        ammSwapPoolContract,
        createPool,
        [
          types.principal(`${deployer.address}.token-wbtc`),
          types.principal(`${deployer.address}.token-wstx`),
          types.uint(100_000_000),
          types.principal(deployer.address),
          types.uint(to_one_8(8)), // 8 BTC
          types.uint(to_one_8(337_796)), // 337_796 STX
        ],
        deployer.address
      ),
      Tx.contractCall(
        ammSwapPoolContract,
        setMaxInRatio,
        [
          types.principal(`${deployer.address}.token-wbtc`),
          types.principal(`${deployer.address}.token-wstx`),
          types.uint(100_000_000),
          types.uint(to_one_8(0.5)), // in ratio == 0.5 == 50% of balance
        ],
        deployer.address
      ),
      Tx.contractCall(
        ammSwapPoolContract,
        setMaxOutRatio,
        [
          types.principal(`${deployer.address}.token-wbtc`),
          types.principal(`${deployer.address}.token-wstx`),
          types.uint(100_000_000),
          types.uint(to_one_8(0.5)), // out ratio == 0.5 == 50% of balance
        ],
        deployer.address
      ),
      Tx.contractCall(
        ammSwapPoolContract,
        setPoolStartBlock,
        [
          types.principal(`${deployer.address}.token-wbtc`),
          types.principal(`${deployer.address}.token-wstx`),
          types.uint(100_000_000),
          types.uint(1), //start block
        ],
        deployer.address
      ),
      Tx.contractCall(
        ammSwapPoolContract,
        getPoolDetails,
        [
          types.principal(`${deployer.address}.token-wbtc`),
          types.principal(`${deployer.address}.token-wstx`),
          types.uint(100_000_000),
        ],
        deployer.address
      ),
    ]);

    assertEquals(block.receipts.length, 5);
    assertEquals(block.height, 9);
    block.receipts[0].result.expectOk().expectBool(true);
    block.receipts[1].result.expectOk().expectBool(true);
    block.receipts[2].result.expectOk().expectBool(true);
    block.receipts[3].result.expectOk().expectBool(true);
    console.log('Trading Pool details: ', block.receipts[4].result);

    // 4. bridge-contract

    // a. swap 10_000 sats using the exchange function
    console.log('8 SWAPS 0.1 xBTC to STX:');
    for (let i = 1; i <= 8; i++) {
      block = chain.mineBlock([
        Tx.contractCall(
          bridgeContract,
          bridgeSwapFn,
          [
            types.principal(`${deployer.address}.token-wbtc`),
            types.principal(`${deployer.address}.token-wstx`),
            types.uint(to_one_8(0.1)), //0.1 xBTC, we have to parse amount in xBTC * 10^8 (sats)
            types.uint(5),
          ],
          accounts.get(`wallet_${i}`).address
        ),
      ]);

      assertEquals(block.receipts.length, 1);
      assertEquals(block.height, i + 9);

      block.receipts[0].result
        .expectOk()
        .expectOk()
        .expectUint(xBtcStxTransferResults[0.1][i - 1]);
      let StxExchangeResult = block.receipts[0].result
        .expectOk()
        .expectOk()
        .expectUint(xBtcStxTransferResults[0.1][i - 1]);
      console.log(
        '0.1 xBTC == ',

        div_one_8(parseFloat(StxExchangeResult)),
        'STX'
      );
    }
    for (let i = 1; i <= 8; i++) {
      block = chain.mineBlock([
        Tx.contractCall(
          bridgeContract,
          bridgeSwapFn,
          [
            types.principal(`${deployer.address}.token-wstx`),
            types.principal(`${deployer.address}.token-wbtc`),
            types.uint(to_one_8(100)), //100 STX, we have to parse amount in STX * 10^8
            types.uint(5),
          ],
          accounts.get(`wallet_${i}`).address
        ),
      ]);

      assertEquals(block.receipts.length, 1);
      console.log(i + 20);
      assertEquals(block.height, i + 17);
      let StxExchangeResult = block.receipts[0].result
        .expectOk()
        .expectOk()
        .expectUint(StxXbtcTransferResults[100][i - 1]);
      console.log('100 STX == ', div_one_8(parseFloat(StxExchangeResult)), 'xBTC');
    }
    // register supplier Bridge

    block = chain.mineBlock([
      Tx.contractCall(
        degenBridgeContract,
        registerSupplier,
        [
          types.buff(
            Uint8Array.from([
              2, 186, 94, 65, 230, 231, 129, 119, 148, 195, 178, 221, 245, 108, 78, 140, 61, 60, 139, 195, 90, 1, 107,
              255, 151, 68, 49, 178, 71, 209, 100, 179, 235,
            ]) // the Uint8Array corresponding to publicKey: 02ba5e41e6e7817794c3b2ddf56c4e8c3d3c8bc35a016bff974431b247d164b3eb
          ),
          types.none(),
          types.none(),
          types.int(0),
          types.int(to_one_8(0.1)), // supply with 0.1 xBTC
          types.uint(1000),
        ],
        deployer.address
      ),
    ]);

    // assert: review returned data, contract state, and other requirements
    assertEquals(block.receipts.length, 1);
    assertEquals(block.height, 26);
    block.receipts[0].result.expectOk().expectUint(0); // supplier Id

    // register supplier Bridge

    block = chain.mineBlock([
      Tx.contractCall(
        bridgeContract,
        swapBridgeStxBtc,
        [
          types.principal(`${deployer.address}.token-wstx`),
          types.principal(`${deployer.address}.token-wbtc`),
          types.uint(to_one_8(100)), //100 STX
          types.uint(5),
          types.buff(
            Uint8Array.from([0]) // the Uint8Array corresponding to btc address version (00 or 0x00)
          ),
          types.buff(
            Uint8Array.from([
              217, 70, 197, 171, 28, 179, 27, 246, 208, 166, 125, 130, 78, 84, 131, 147, 220, 163, 185, 248,
            ])
          ), //02e8f7dc91e49a577ce9ea8989c7184aea8886fe5250f02120dc6f98e3619679b0
          types.uint(0),
        ],
        wallet_1.address
      ),
    ]);

    assertEquals(block.receipts.length, 1);
    assertEquals(block.height, 27);
    block.receipts[0].result.expectOk().expectUint(284983);
  },
});
