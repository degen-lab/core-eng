import { Clarinet, Contract, Account } from 'https://deno.land/x/clarinet@v1.7.0/index.ts';
import { getContractName, isTestContract, exitWithError } from './deps.ts';

const readmeFile = './README.md';

const constantErrRegex = /^\s*\(define-constant\s+(err-.+?)\s+(\(.+?\))\s*\)(.*?)$/gm;
const errorCodeRegex = /u([0-9]+)/;
const commentRegex = /;;\s*(.+)/;
const readmeErrorsDelineator = '<!--errors-->';

const tableHeader = ['Contract', 'Constant', 'Value', 'Description'];

function padTableCell(content: string, length: number) {
	const repeat = length - content.length + 1;
	return repeat > 0 ? ' ' + content + ' '.repeat(repeat) : ' ';
}

Clarinet.run({
	async fn(accounts: Map<string, Account>, contracts: Map<string, Contract>) {
		const errorsSeenCount: { [key: string]: { lastConstantName: string, count: number } } = {};
		let readme = await Deno.readTextFile(readmeFile);
		const errorTable: Array<Array<string>> = [];
		const longestColumnCells = tableHeader.map(v => v.length);

		const compareReadme = Deno.env.get("EXTRACT_CHECK") && readme;

		for (const [contractId, contract] of contracts) {
			const contractName = getContractName(contractId);
			if (isTestContract(contractName))
				continue;

			const errorConstants = contract.source.matchAll(constantErrRegex);
			for (const [, errorConstant, errorValue, errorComment] of errorConstants) {
				const errorDescription = errorComment?.match(commentRegex)?.[1] || ''; // || '_None_';
				if (!errorValue.match(errorCodeRegex))
					console.error(`Constant '${errorConstant}' error value is not in form of (err uint)`);
				if (!errorsSeenCount[errorValue])
					errorsSeenCount[errorValue] = { lastConstantName: errorConstant, count: 1 };
				else if (errorsSeenCount[errorValue].lastConstantName !== errorConstant) {
					errorsSeenCount[errorValue].lastConstantName = errorConstant;
					++errorsSeenCount[errorValue].count;
				}
				const row = [getContractName(contractId), errorConstant, errorValue, errorDescription];
				row.map((content, index) => { if (content.length > longestColumnCells[index]) longestColumnCells[index] = content.length });
				errorTable.push(row);
			}
		}

		const nonUniqueErrors = Object.entries(errorsSeenCount).filter(([, value]) => value.count > 1);
		if (nonUniqueErrors.length > 0)
			exitWithError("Found non-unique error codes with different names.", nonUniqueErrors);

		errorTable.sort((a, b) => a[2] > b[2] ? 1 : -1); // string sort

		let errors = '|' + tableHeader.map((content, index) => padTableCell(content, longestColumnCells[index])).join('|') + "|\n";
		errors += '|' + longestColumnCells.map(length => '-'.repeat(length + 2)).join('|') + "|\n";
		errors += errorTable.reduce((accumulator, row) => accumulator + '|' + row.map((content, index) => padTableCell(content, longestColumnCells[index])).join('|') + "|\n", '');

		const split = readme.split(readmeErrorsDelineator);
		readme = `${split[0]}${readmeErrorsDelineator}\n${errors}${readmeErrorsDelineator}${split[2]}`;

		if (compareReadme && compareReadme !== readme) {
			exitWithError("Generated readme is not equal to readme in current commit (error table mismatch)");
		}

		Deno.writeTextFile(readmeFile, readme);
		console.log(`Error table written to ${readmeFile}`);
	}
});
