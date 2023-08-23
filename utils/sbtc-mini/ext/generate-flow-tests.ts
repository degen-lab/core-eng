import {
  Clarinet,
  Contract,
  Account,
} from "https://deno.land/x/clarinet@v1.7.0/index.ts";
import { CallInfo, FunctionAnnotations, FunctionBody, extractTestAnnotationsAndCalls, getContractName } from "./utils/clarity-parser.ts";
import { defaultDeps, generateBootstrapFile, warningText } from "./utils/generate.ts";

const sourcebootstrapFile = './tests/bootstrap.ts';
const targetFolder = ".test";

function isTestContract(contractName: string) {
  return (
    contractName.substring(contractName.length - 10) === "_flow_test");
}

Clarinet.run({
  async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>) {
		Deno.writeTextFile(`${targetFolder}/deps.ts`, defaultDeps);
		Deno.writeTextFile(`${targetFolder}/bootstrap.ts`, await generateBootstrapFile(sourcebootstrapFile));

    for (const [contractId, contract] of contracts) {
      const contractName = getContractName(contractId);
      if (!isTestContract(contractName)) continue;

      const hasDefaultPrepareFunction =
        contract.contract_interface.functions.reduce(
          (a, v) =>
            a ||
            (v.name === "prepare" &&
              v.access === "public" &&
              v.args.length === 0),
          false
        );
      const [annotations, functionBodies] = extractTestAnnotationsAndCalls(
        contract.source
      );

      const code: string[][] = [];
      code.push([
        warningText,
        ``,
        `import { Clarinet, Tx, Chain, Account, types, assertEquals, printEvents } from './deps.ts';`,
        `import { bootstrap } from './bootstrap.ts';`,
        ``,
      ]);

      for (const {
        name,
        access,
        args,
      } of contract.contract_interface.functions.reverse()) {
        // is test function
        if (access !== "public" || name.substring(0, 5) !== "test-") continue;
        if (args.length > 0)
          throw new Error(
            `Test functions cannot take arguments. (Offending function: ${name})`
          );
        const functionAnnotations = annotations[name] || {};
        // update prepare annotation
        if (hasDefaultPrepareFunction && !functionAnnotations.prepare)
          functionAnnotations.prepare = "prepare";
        if (functionAnnotations["no-prepare"])
          delete functionAnnotations.prepare;

        const functionBody = functionBodies[name] || [];
        code.push([
          generateTest(contractId, name, functionAnnotations, functionBody),
        ]);
      }

      Deno.writeTextFile(
        `${targetFolder}/${contractName}.ts`,
        code.flat().join("\n")
      );
    }
  },
});

/**
 * Generates ts code for a contract call with arguments.
 * The function is called by `contractCaller`.
 * To be used in mineBlock.
 *
 * Only arguments that are supported by the clarity-parser are
 * generated correctly in this generator method.
 *
 * @param callInfo
 * @param contractPrincipal
 * @returns
 */
function generateTxContractCallWithArguments(
  callInfo: CallInfo,
  contractPrincipal: string
): string {
  const argStrings = callInfo.args
    .map((arg) => arg.value)
    .join(", ");

  return `Tx.contractCall("${callInfo.contractName || contractPrincipal}", "${
    callInfo.functionName
  }", [${argStrings}], callerAddress)
  `;
}

/**
 * Generates ts code for mineBlock calls
 * with grouped function calls as defined in the function body.
 * @param contractPrincipal
 * @param calls
 * @returns
 */
function generateBlocks(contractPrincipal: string, calls: FunctionBody) {
  let code = "";
  let blockStarted = false;
  for (const { callAnnotations, callInfo } of calls) {
    // mine empty blocks
    const mineBlocksBefore =
      parseInt(callAnnotations["mine-blocks-before"] as string) || 0;
    if (mineBlocksBefore > 1) {
      if (blockStarted) {
        code += `
			  ]);
			  block.receipts.map(({result}) => result.expectOk());
			  `;
        blockStarted = false;
      }
      code += `
			  chain.mineEmptyBlock(${mineBlocksBefore - 1});`;
    }
    // start a new block if necessary
    if (!blockStarted) {
      code += `
			  block = chain.mineBlock([`;
      blockStarted = true;
    }
    // add tx to current block
    code += generateTxContractCallWithArguments(callInfo, contractPrincipal);
    code += `,
	`;
  }
  // close final block
  if (blockStarted) {
    code += `
	  ]);
	  block.receipts.map(({result}) => result.expectOk());
	  `;
    blockStarted = false;
  }
  return code;
}


/**
 * Generates the ts code for a unit test
 * @param contractPrincipal
 * @param testFunction
 * @param annotations
 * @returns
 */
function generateTest(
  contractPrincipal: string,
  testFunction: string,
  annotations: FunctionAnnotations,
  body: FunctionBody
) {
  return `Clarinet.test({
	name: "${
    annotations.name
      ? testFunction + ": " + (annotations.name as string).replace(/"/g, '\\"')
      : testFunction
  }",
	async fn(chain: Chain, accounts: Map<string, Account>) {
		const deployer = accounts.get("deployer")!;
		bootstrap(chain, deployer);
		let callerAddress = ${
      annotations.caller
        ? annotations.caller[0] === "'"
          ? `"${(annotations.caller as string).substring(1)}"`
          : `accounts.get('${annotations.caller}')!.address`
        : `accounts.get('deployer')!.address`
    };
		let block;
		${generateBlocks(contractPrincipal, body)}
	}
});
`;
}