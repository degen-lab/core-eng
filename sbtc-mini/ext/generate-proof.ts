import { concat } from "https://deno.land/std@0.101.0/bytes/mod.ts";
import { randomBytes } from "https://deno.land/std@0.97.0/node/crypto.ts";
import { sha256 } from "https://denopkg.com/chiefbiiko/sha256@v1.0.0/mod.ts";
import { Buffer } from "https://deno.land/std@0.159.0/node/buffer.ts";
import { assert } from "https://deno.land/std@0.97.0/testing/asserts.ts";

export const LEAF_NODE_PREFIX: Uint8Array = new Uint8Array([0]);
export const INNER_NODE_PREFIX: Uint8Array = new Uint8Array([1]);

const numToUint8Array = (num) => {
  const arr = new Uint8Array(8);

  for (let i = 0; i < 8; i++)
    arr.set([num / 0x100 ** i], 7 - i)

  return arr;
}

const generateBitcoinMerkleTree = (data: Uint8Array[], merkleTree: Uint8Array[][]) => {
  merkleTree.push(data);

  if (data.length === 1) return merkleTree;

  const newLevel: Uint8Array[] = [];

  for (let i = 0; i < data.length; i += 2) {
    // Left
    const d1 = data[i]
    // Right, or duplicate left if odd
    const d2 = data[i + 1] || d1
    newLevel.push(
      sha256(sha256(concat(d1, d2))) as Uint8Array)
  }

  generateBitcoinMerkleTree(newLevel, merkleTree);
}

// length max value is 2^64 (8 byte)
const generateBlockIds = (length: number, randomIds = true): Uint8Array[] => {
  const ids = Array.from(new Array(length), (_, idx): Uint8Array => {
    if (randomIds)
      // 32 bytes is enough randomness
      return sha256(randomBytes(32)) as Uint8Array;
    return sha256(numToUint8Array(idx)) as Uint8Array;
  });

  return ids;
}

// txId is reversed (not what you see in explorers)
// output: list of txids is reversed (not what you see in explorers)
const generateSegwitBlockIdsFromTxId = (length: number, txId: Uint8Array, pos: number, randomIds = true) => {
  const ids = generateBlockIds(length, randomIds);
  ids[0] = new Uint8Array(32);
  ids[pos] = txId.slice(0, txId.length);

  return ids;
}

// txId is reversed (not what you see in explorers)
// output: list of txids is reversed (not what you see in explorers)
const generateBlockIdsFromTxId = (length: number, txId: Uint8Array, pos: number, randomIds = true) => {
  assert(length > pos, "list is not long eough for the position chosen");
  assert(txId.byteLength === 32, "txID length has to be 32");

  const ids = generateBlockIds(length, randomIds);
  ids[pos] = txId.slice(0, txId.length);

  return ids;
}

const compare = (left: Uint8Array, right: Uint8Array): boolean => {
  if (left === right) return true;
  if (left.byteLength != right.byteLength) return false;

  for (let i = 0; i < left.byteLength; i++)
    if (left[i] != right[i]) return false;

  return true
}

const getTxIdProofPath = (leaf: Uint8Array, layers: Uint8Array[][]): Uint8Array[] | undefined => {
  const leafs = layers[0];
  const index = leafs.findIndex((val) => compare(leaf, val));

  if (index >= 0)
    return getProofPath(index, layers);

  return undefined;
}

// txid is reversed (not what you see in explorers)
const getProofPath = (index: number, layers: Uint8Array[][]) => {
  const proof = [];
  for (let i = 0; i < layers.length; i++) {
    const layer = layers[i].slice(0, layers[i].length);
    const isRightNode = index % 2;
    const pairIndex = (isRightNode ? index - 1 : true && index === layer.length - 1 && i < layers.length - 1
      // Proof Generation for Bitcoin Trees
      ? index
      // Proof Generation for Non-Bitcoin Trees
      : index + 1)


    if (pairIndex < layer.length) {
      proof.push(layer[pairIndex].slice(0, layer[pairIndex].length))
    }

    // set index to parent index
    index = Math.floor(index / 2)
  }

  return proof;
}

// The merkle root in LE relative to the way txids were hashed
const generateBlockHeader = (merkleRoot: Uint8Array): Uint8Array => {
  const version = new Uint8Array(4);
  const previousBlockHash = new Uint8Array(32);
  const time = new Uint8Array(4);
  const bits = new Uint8Array(4);
  const nonce = new Uint8Array(4);

  const header = new Uint8Array(80);

  header.set(version, 0);
  header.set(previousBlockHash, 4);
  header.set(merkleRoot, 36);
  header.set(time, 68);
  header.set(bits, 72);
  header.set(nonce, 76);

  return header;
}

const getReversedTxId = (rawTx: Uint8Array): Uint8Array => {
  return sha256(sha256(rawTx)) as Uint8Array;
}

const generateProofs = (segwitRawTx: Uint8Array, txAmount: number, txIndex: number, randomTxs = false) => {
  // static coinbase address
  const segwitBlockIds = generateSegwitBlockIdsFromTxId(txAmount, getReversedTxId(segwitRawTx), txIndex, randomTxs);
  const reversedTxid = getReversedTxId(segwitRawTx);

  const segwitLayers: Uint8Array[][] = [];

  generateBitcoinMerkleTree(segwitBlockIds, segwitLayers);

  const segwitProof = getTxIdProofPath(reversedTxid, segwitLayers)!;
  const segwitMerkleRoot = segwitLayers[segwitLayers.length - 1][0];
  const witnessReservedData = new Uint8Array(32);

  const output = `6a24aa21a9ed${Buffer.from(sha256(sha256(concat(segwitMerkleRoot, witnessReservedData)))).toString("hex")}`;
  const coinbaseRawTX = Buffer.from(`01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff1e0383a02519444d47426c6f636b636861696e309e3c092400000000000000ffffffff029e49250000000000160014b23716e183ba0949c55d6cac21a3e94176eed112000000000000000026${output}0120000000000000000000000000000000000000000000000000000000000000000000000000`, "hex");

  const coinbaseReversedTxid = getReversedTxId(coinbaseRawTX);

  const blockIds = generateBlockIdsFromTxId(txAmount, coinbaseReversedTxid, 0, randomTxs);
  const layers: Uint8Array[][] = [];

  generateBitcoinMerkleTree(blockIds, layers);

  const proof = getTxIdProofPath(coinbaseReversedTxid, layers)!;
  const merkleRoot = layers[layers.length - 1][0];
  const treeDepth = layers.length - 1;

  const blockHeader = generateBlockHeader(merkleRoot.slice(0, merkleRoot.byteLength));

  return {
    coinbaseRawTX,
    blockHeader,
    txIndex,
    treeDepth,
    coinbaseProof: proof,
    segwitRawTx,
    segwitProof: segwitProof,
    segwitMerkleRoot,
    witnessReservedData,
  };
}

const mockTxHex = "02000000000101c0ae643bde33b865dec47beeff21ddc92a666d37135c6b6522a4cd9f66ae95780000000000fdffffff02808d5b000000000022512089f35f8ee7546e7778bf1a13f1472bb6a37c3164fd3b5fd78f9ebefc2301384cdb63aa29010000002251200578716337781bf4563d4aacec67ee8d1875e33fced744da937b1bab5e52f8fe0247304402207085dbf16b21bc9eb28aefc52e3a7f0fd3b3782813b33d3d5f8325929037db2a02200f1b2bdc520df530c7484ba8d5d1ee00808d309b54f1cf9ee62d7bc888edc56f012103caf9241a5f9e930cf9711b3fa8fba25d2e77861ff7f15a4074a8aec86787617e00000000";
const proof = generateProofs(Buffer.from(mockTxHex, "hex"), 2, 1);

// console.log(a);


console.log("tx: ", Buffer.from(proof.segwitRawTx).toString("hex"));
console.log("header hash: ", Buffer.from((sha256(sha256(proof.blockHeader)) as Uint8Array).reverse()).toString("hex"));
console.log("tx-index: ", proof.txIndex);
console.log("tree-depth: ", proof.treeDepth);
console.log("block header: ", Buffer.from(proof.blockHeader).toString("hex"));

console.log("\nwproof");
console.log(proof.segwitProof.map((tx) => Buffer.from(tx).toString("hex")));
console.log("witness-merkle-root: ", Buffer.from(proof.segwitMerkleRoot).toString("hex"));
console.log("Witness Reversed Data: ", Buffer.from(proof.witnessReservedData).toString("hex"));

console.log("ctx: ", Buffer.from(proof.coinbaseRawTX).toString("hex"));

console.log("\ncproof");
console.log(proof.coinbaseProof.map((tx) => Buffer.from(tx).toString("hex")));

