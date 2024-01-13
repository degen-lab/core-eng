// import { Clarinet, Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v1.5.4/index.ts';
// import { assertEquals } from 'https://deno.land/std@0.170.0/testing/asserts.ts';
// import sha256 from 'https://deno.land/x/sha256js/sha256.mjs';
// import ripemd160 from 'https://deno.land/x/ripemd160js@v2.0.3/ripemd160.mjs';

// const CONVERT_TO_STX = (amount: number) => {
//   return amount * 1000000;
// };

// const CONTRACT_NAME = 'mining-pool-5-blocks';
// const ASK_TO_JOIN = 'ask-to-join';
// const GET_MINERS_LIST = 'get-miners-list';
// const GET_WAITING_LIST = 'get-waiting-list';
// const GET_PENDING_LIST = `get-pending-accept-list`;
// const ADD_PENDING_MINERS = 'add-pending-miners-to-pool';
// const VOTE_POSITIVE_JOIN = 'vote-positive-join-request';
// const VOTE_NEGATIVE_JOIN = 'vote-negative-join-request';
// const VOTE_POSITIVE_REMOVE = 'vote-positive-remove-request';
// const VOTE_NEGATIVE_REMOVE = 'vote-negative-remove-request';
// const START_VOTE_NOTIFIER = 'start-vote-notifier';
// const END_VOTE_NOTIFIER = 'end-vote-notifier';
// const VOTE_NOTIFIER = 'vote-notifier';
// const DEPOSIT = 'deposit-stx';
// const WITHDRAW = 'withdraw-stx';
// const GET_BALANCE = 'get-balance';
// const GET_REWARD_AT_BLOCK_READ = 'get-reward-at-block-read';
// const REWARD_DISTRIBUTION = 'reward-distribution';
// const GET_DATA_WAITING_MINER = 'get-all-data-waiting-miners';
// const GET_DATA_REMOVAL = 'get-all-data-miners-proposed-for-removal';
// const err_insufficient_balance = '(err u1001)';
// const err_missing_balance = '(err u1002)';
// const LEAVE_POOL = 'leave-pool';
// const TRY_ENTER_POOL = 'try-enter-pool';
// const PROPOSE_REMOVAL = 'propose-removal';
// const PROPOSE_NOTIFIER = 'propose-notifier';
// const GET_PROPOSED_NOTIFIERS_LIST = 'get-proposed-notifiers-list';
// const GET_K = 'get-k';
// const GET_NOTIFIER = 'get-notifier';
// const GET_NOTIFIER_VOTE_STATUS = 'get-notifier-vote-status';
// const GET_NOTIFIER_VOTE_NUMBER = 'get-notifier-vote-number';
// const GET_MAX_VOTES_NOTIFIER = 'get-max-votes-notifier ';
// const GET_MAX_VOTED_NOTIFIER = 'get-max-voted-notifier ';
// const err_no_vote_permission = '(err u105)';
// const err_more_blocks_to_pass = '(err u106)';
// const err_no_pending_miners = '(err u107)';
// const err_already_voted = '(err u108)';
// const err_not_asked_to_join = '(err u109)';
// const err_notifier = '(err u113)';
// const err_not_proposed_removal = '(err u117)';
// const err_cant_vote_himself = '(err u119)';
// const err_already_proposed_for_notifier = '(err u121)';
// const err_not_proposed_notifier = '(err u124)';
// const err_already_notifier = '(err u125)';
// const err_no_voting_period = '(err u129)';

// // Conversions for hashing
// const publicKeyHex = '02e8f7dc91e49a577ce9ea8989c7184aea8886fe5250f02120dc6f98e3619679b0';

// const buffer_from = (num: string) => {
//   const hexString = num;
//   const byteArray = new Uint8Array(hexString.length / 2);

//   for (let i = 0; i < byteArray.length; i++) {
//     byteArray[i] = parseInt(hexString.substr(i * 2, 2), 16);
//   }
//   return byteArray;
// };

// const hash160 = (value: any) => {
//   return sha256(value).then((x: any) => ripemd160(x));
// };

// Clarinet.test({
//   name: 'Get All Data Waiting 300 Miners',
//   async fn(chain: Chain, accounts: Map<string, Account>) {
//     const deployer = accounts.get('deployer')!;
//     let waiting_list = [];
//     let block = chain.mineBlock([]);
//     assertEquals(block.receipts.length, 0);
//     assertEquals(block.height, 2);

//     for (let i = 1; i <= 299; i++) {
//       const miner = accounts.get(`wallet_${i}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(
//           CONTRACT_NAME,
//           ASK_TO_JOIN,
//           [
//             types.tuple({
//               version: types.buff(hash160(buffer_from('00'))),
//               hashbytes: types.buff(hash160(buffer_from(publicKeyHex))),
//             }),
//           ],
//           miner.address
//         ),
//       ]);
//       if (i == 1) waiting_list.push(`${miner.address}`);
//       else waiting_list.push(` ${miner.address}`);
//     }
//     block = chain.mineBlock([Tx.contractCall(CONTRACT_NAME, GET_WAITING_LIST, [], deployer.address)]);
//     assertEquals(block.receipts[0].result, `[${waiting_list}]`);
//     assertEquals(block.receipts.length, 1);
//     assertEquals(block.height, 302);

//     for (let i = 1; i <= 299; i++) {
//       const miner = accounts.get(`wallet_${i}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(CONTRACT_NAME, VOTE_POSITIVE_JOIN, [types.principal(miner.address)], deployer.address),
//         Tx.contractCall(CONTRACT_NAME, TRY_ENTER_POOL, [], miner.address),
//       ]);
//     }

//     block = chain.mineBlock([Tx.contractCall(CONTRACT_NAME, GET_PENDING_LIST, [], deployer.address)]);
//     assertEquals(block.receipts[0].result, `[${waiting_list}]`);
//     assertEquals(block.receipts.length, 1);
//     assertEquals(block.height, 602);

//     for (let i = 1; i <= 50; i++) {
//       block = chain.mineBlock([]);
//     }

//     block = chain.mineBlock([
//       Tx.contractCall(CONTRACT_NAME, ADD_PENDING_MINERS, [], deployer.address),
//       Tx.contractCall(CONTRACT_NAME, GET_MINERS_LIST, [], deployer.address),
//     ]);
//     assertEquals(block.receipts[0].result, `(ok true)`);
//     assertEquals(block.receipts[1].result, `[${deployer.address}, ${waiting_list}]`);
//     assertEquals(block.receipts.length, 2);
//     assertEquals(block.height, 653);

//     block = chain.mineBlock([
//       Tx.contractCall(
//         CONTRACT_NAME,
//         ASK_TO_JOIN,
//         [
//           types.tuple({
//             version: types.buff(hash160(buffer_from('00'))),
//             hashbytes: types.buff(hash160(buffer_from(publicKeyHex))),
//           }),
//         ],
//         accounts.get(`wallet_${300}`)!.address
//       ),
//       Tx.contractCall(
//         CONTRACT_NAME,
//         GET_DATA_WAITING_MINER,
//         [types.list([types.principal(accounts.get(`wallet_${300}`)!.address)])],
//         deployer.address
//       ),
//     ]);

//     for (let i = 91; i <= 299; i++) {
//       const miner = accounts.get(`wallet_${300}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(
//           CONTRACT_NAME,
//           VOTE_POSITIVE_JOIN,
//           [types.principal(miner.address)],
//           accounts.get(`wallet_${i}`)!.address
//         ),
//       ]);
//     }

//     for (let i = 1; i <= 90; i++) {
//       const miner = accounts.get(`wallet_${300}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(
//           CONTRACT_NAME,
//           VOTE_NEGATIVE_JOIN,
//           [types.principal(miner.address)],
//           accounts.get(`wallet_${i}`)!.address
//         ),
//       ]);
//     }

//     block = chain.mineBlock([
//       Tx.contractCall(
//         CONTRACT_NAME,
//         GET_DATA_WAITING_MINER,
//         [types.list([types.principal(accounts.get(`wallet_${300}`)!.address)])],
//         deployer.address
//       ),
//     ]);
//   },
// });

// Clarinet.test({
//   name: 'Get All Data Removals 300 Miners',
//   async fn(chain: Chain, accounts: Map<string, Account>) {
//     const deployer = accounts.get('deployer')!;
//     let waiting_list = [];
//     let block = chain.mineBlock([]);
//     assertEquals(block.receipts.length, 0);
//     assertEquals(block.height, 2);

//     for (let i = 1; i <= 299; i++) {
//       const miner = accounts.get(`wallet_${i}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(
//           CONTRACT_NAME,
//           ASK_TO_JOIN,
//           [
//             types.tuple({
//               version: types.buff(hash160(buffer_from('00'))),
//               hashbytes: types.buff(hash160(buffer_from(publicKeyHex))),
//             }),
//           ],
//           miner.address
//         ),
//       ]);
//       if (i == 1) waiting_list.push(`${miner.address}`);
//       else waiting_list.push(` ${miner.address}`);
//     }
//     block = chain.mineBlock([Tx.contractCall(CONTRACT_NAME, GET_WAITING_LIST, [], deployer.address)]);
//     assertEquals(block.receipts[0].result, `[${waiting_list}]`);
//     assertEquals(block.receipts.length, 1);
//     assertEquals(block.height, 302);

//     for (let i = 1; i <= 299; i++) {
//       const miner = accounts.get(`wallet_${i}`)!;
//       block = chain.mineBlock([
//         Tx.contractCall(CONTRACT_NAME, VOTE_POSITIVE_JOIN, [types.principal(miner.address)], deployer.address),
//         Tx.contractCall(CONTRACT_NAME, TRY_ENTER_POOL, [], miner.address),
//       ]);
//     }

//     block = chain.mineBlock([Tx.contractCall(CONTRACT_NAME, GET_PENDING_LIST, [], deployer.address)]);
//     assertEquals(block.receipts[0].result, `[${waiting_list}]`);
//     assertEquals(block.receipts.length, 1);
//     assertEquals(block.height, 602);

//     for (let i = 1; i <= 50; i++) {
//       block = chain.mineBlock([]);
//     }

//     block = chain.mineBlock([
//       Tx.contractCall(CONTRACT_NAME, ADD_PENDING_MINERS, [], deployer.address),
//       Tx.contractCall(CONTRACT_NAME, GET_MINERS_LIST, [], deployer.address),
//     ]);
//     assertEquals(block.receipts[0].result, `(ok true)`);
//     assertEquals(block.receipts[1].result, `[${deployer.address}, ${waiting_list}]`);
//     assertEquals(block.receipts.length, 2);
//     assertEquals(block.height, 653);

//     block = chain.mineBlock([
//       Tx.contractCall(
//         CONTRACT_NAME,
//         PROPOSE_REMOVAL,
//         [types.principal(accounts.get(`wallet_${299}`)!.address)],
//         deployer.address
//       ),
//       Tx.contractCall(
//         CONTRACT_NAME,
//         GET_DATA_REMOVAL,
//         [types.list([types.principal(accounts.get(`wallet_${299}`)!.address)])],
//         deployer.address
//       ),
//     ]);
//   },
// });
