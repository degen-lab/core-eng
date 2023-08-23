import { Tx, Chain, Account, types } from 'https://deno.land/x/clarinet@v1.7.0/index.ts';

export const bootstrapContracts = [
	'.sbtc-token',
	'.sbtc-peg-in-processor',
	'.sbtc-peg-out-processor',
	'.sbtc-registry',
	'.sbtc-stacking-pool',
	'.sbtc-testnet-debug-controller',
	'.sbtc-token'
];

export function bootstrap(chain: Chain, deployer: Account) {
	const { receipts } = chain.mineBlock([
		Tx.contractCall(
			`${deployer.address}.sbtc-controller`,
			'upgrade',
			[types.list(bootstrapContracts.map(contract => types.tuple({ contract, enabled: true })))],
			deployer.address
		)
	]);
	receipts[0].result.expectOk().expectList().map(result => result.expectBool(true));
}
