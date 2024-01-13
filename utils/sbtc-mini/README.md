# sBTC Mini protocol

sBTC Mini is a simplified version of the sBTC protocol. It is a work in
progress.

## Bootstrap

The protocol is made up of multiple contracts, each implementing a part of
sBTC Mini. The bootstrapping procedure happens in two steps:

1. Deploy all the contracts in the correct order. Clarinet will determine it for
   you.
2. Send the bootstrapping transaction from the contract deployer.

### Bootstrapping transaction

The first call to the `upgrade` function of the `sbtc-controller` will bootstrap
the protocol. The function can only be called by the contract deployer, and can
only be called once.

The deployer shall provide a list of all sBTC protocol contracts except the
controller itself to enable them all. In a local Clarinet console session, it
can be done as follows:

```clarity
(contract-call? .sbtc-controller upgrade (list {contract: .sbtc-token, enabled: true} {contract: .sbtc-deposit-verifier, enabled: true} {contract: .sbtc-withdrawal-verifier, enabled: true} {contract: .sbtc-registry, enabled: true} {contract: .sbtc-stacking-pool, enabled: true} {contract: .sbtc-token, enabled: true}))
```

After the bootstrapping transaction is processed, the contract deployer will
have no special access to the protocol and the private key can be discarded or
published.

## Errors

The sBTC protocol contracts each have their own error space. All protocol errors
are in the form `(err uint)` and they are unique across all contracts.

### Error space

| Group           | Error space | Description                                               |
| --------------- | ----------- | --------------------------------------------------------- |
| Controller      | 1XXX        | Errors related to the controller and upgrades.            |
| Registry        | 2XXX        | Errors coming directly from the registry.                 |
| Token           | 3XXX        | Errors coming directly from the token.                    |
| Deposit         | 4XXX        | Errors related to making and processing BTC deposits.     |
| Redemption      | 5XXX        | Errors related to redeeming sBTC for BTC.                 |
| Pool            | 6XXX        | Errors related to the sBTC stacking pool.                 |
| Hand-off        | 7XXX        | Errors related to the peg hand-off process.               |
| Bitcoin library | 8XXX        | Errors coming directly from the Bitcoin library / helper. |
| Debug           | 99XX        | Debugging related errors.                                 |

### Error table

<!--errors-->
| Contract                        | Constant                                        | Value       | Description                                                                                                                 |
|---------------------------------|-------------------------------------------------|-------------|-----------------------------------------------------------------------------------------------------------------------------|
| sbtc-controller                 | err-unauthorised                                | (err u1000) | The `is-protocol-caller` check failed.                                                                                      |
| sbtc-registry                   | err-burn-tx-already-processed                   | (err u2000) | A burnchain TXID was processed (seen) before.                                                                               |
| sbtc-registry                   | err-sbtc-wallet-already-set                     | (err u2001) | A peg wallet address for the specified cycle was already set.                                                               |
| sbtc-registry                   | err-minimum-burnchain-confirmations-not-reached | (err u2002) | The burnchain transaction did not yet reach the minimum amount of confirmation.                                             |
| sbtc-registry                   | err-not-settled-state                           | (err u2003) | The state passed to function `get-and-settle-pending-withdrawal-request` was not a settled state. (Fulfilled or cancelled.) |
| sbtc-registry                   | err-invalid-txid-length                         | (err u2004) | The passed TXID byte length was not equal to 32.                                                                            |
| sbtc-registry                   | err-unknown-withdrawal-request                  | (err u2005) | The withdrawal request ID passed to `get-and-settle-pending-withdrawal-request` does not exist.                             |
| sbtc-registry                   | err-withdrawal-not-pending                      | (err u2006) | The withdrawal request ID passed to `get-and-settle-pending-withdrawal-request` is not in a pending state.                  |
| sbtc-token                      | err-not-token-owner                             | (err u4)    | `tx-sender` or `contract-caller` tried to move a token it does not own.                                                     |
| sbtc-deposit-verifier           | err-not-a-sbtc-wallet                           | (err u4000) | There is no peg wallet address for the specified wallet.                                                                    |
| sbtc-deposit-verifier           | err-invalid-spending-pubkey                     | (err u4001) | The recipient of the BTC is not the same as the pubkey that unlocked the spending script.                                   |
| sbtc-deposit-verifier           | err-peg-value-not-found                         | (err u4002) | There was no output containing the peg wallet scriptPubKey.                                                                 |
| sbtc-deposit-verifier           | err-missing-witness                             | (err u4003) | The Taproot witness was missing.                                                                                            |
| sbtc-deposit-verifier           | err-unlock-script-not-found-or-invalid          | (err u4004) | The unlock script at the specified witness index did not exist or was invalid. (Not according to the sBTC spec.)            |
| sbtc-deposit-verifier           | err-script-invalid-opcode                       | (err u4010) | The opcode in the unlock script was invalid.                                                                                |
| sbtc-deposit-verifier           | err-script-invalid-version                      | (err u4011) | The version in the unlock script was invalid.                                                                               |
| sbtc-deposit-verifier           | err-script-not-op-drop                          | (err u4012) | The script does not contain OP_DROP at the expected offset.                                                                 |
| sbtc-deposit-verifier           | err-script-checksig-missing                     | (err u4013) | The script does not contain OP_CHECKSIG at the expected offset.                                                             |
| sbtc-deposit-verifier           | err-script-missing-pubkey                       | (err u4014) | The script does not contain a Taproot pubkey.                                                                               |
| sbtc-deposit-verifier           | err-script-invalid-principal                    | (err u4015) | The encoded Stacks principal inside the script is invalid.                                                                  |
| sbtc-deposit-verifier           | err-script-invalid-length                       | (err u4016) | The length of the script is different from what is expected.                                                                |
| sbtc-withdrawal-request-btc     | err-token-lock-failed                           | (err u5000) | The amount of tokens specified in the request is larger than the amount the user owns.                                      |
| sbtc-withdrawal-request-btc     | err-unacceptable-expiry-height                  | (err u5001) | The burnchain expiry height specified in the request is too short.                                                          |
| sbtc-withdrawal-request-stx     | err-token-lock-failed                           | (err u5100) | The amount of tokens specified in the request is larger than the amount the user owns.                                      |
| sbtc-withdrawal-request-stx     | err-cannot-set-allowance-for-self               | (err u5101) | Tried to set an allowance for oneself, which is not allowed.                                                                |
| sbtc-withdrawal-request-stx     | err-operator-not-allowed                        | (err u5102) | The operator was not allowed to request the withdrawal. The allowance is zero or insufficient.                              |
| sbtc-withdrawal-request-stx     | err-no-sponsor                                  | (err u5103) | Called a sponsored function but the transaction was not sponsored.                                                          |
| sbtc-withdrawal-verifier        | err-wrong-value                                 | (err u5200) | The value transferred to the destination is less than the requested amount.                                                 |
| sbtc-withdrawal-verifier        | err-invalid-destination                         | (err u5201) | The destination in the withdrawal request is invalid.                                                                       |
| sbtc-withdrawal-verifier        | err-old-burnchain-transaction                   | (err u5202) | The withdrawal fulfilment proof is older than the burn height at which the withdrawal request was made.                     |
| sbtc-withdrawal-request-stx     | err-invalid-destination                         | (err u5203) | The destination is not in a format that the protocol understands. Should be P2WPKH, P2WSH, or P2TR.                         |
| sbtc-withdrawal-request-reclaim | err-withdrawal-not-epxired                      | (err u5300) | The withdrawal request has not yet hit the expiry burn block height.                                                        |
| sbtc-stacking-pool              | err-not-signer                                  | (err u6000) |                                                                                                                             |
| sbtc-stacking-pool              | err-allowance-not-set                           | (err u6001) |                                                                                                                             |
| sbtc-stacking-pool              | err-allowance-height                            | (err u6002) |                                                                                                                             |
| sbtc-stacking-pool              | err-already-pre-signer-or-signer                | (err u6003) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-in-registration-window                  | (err u6004) |                                                                                                                             |
| sbtc-stacking-pool              | err-pre-registration-delegate-stx               | (err u6005) |                                                                                                                             |
| sbtc-stacking-pool              | err-pre-registration-delegate-stack-stx         | (err u6006) |                                                                                                                             |
| sbtc-stacking-pool              | err-pre-registration-aggregate-commit           | (err u6007) |                                                                                                                             |
| sbtc-stacking-pool              | err-public-key-already-used                     | (err u6008) |                                                                                                                             |
| sbtc-stacking-pool              | err-pox-address-re-use                          | (err u6009) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-enough-stacked                          | (err u6010) |                                                                                                                             |
| sbtc-stacking-pool              | err-wont-unlock                                 | (err u6011) |                                                                                                                             |
| sbtc-stacking-pool              | err-voting-period-closed                        | (err u6012) |                                                                                                                             |
| sbtc-stacking-pool              | err-already-voted                               | (err u6013) |                                                                                                                             |
| sbtc-stacking-pool              | err-decrease-forbidden                          | (err u6014) |                                                                                                                             |
| sbtc-stacking-pool              | err-pre-registration-stack-increase             | (err u6015) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-in-good-peg-state                       | (err u6016) |                                                                                                                             |
| sbtc-stacking-pool              | err-unwrapping-candidate                        | (err u6017) |                                                                                                                             |
| sbtc-stacking-pool              | err-pool-cycle                                  | (err u6018) |                                                                                                                             |
| sbtc-stacking-pool              | err-too-many-candidates                         | (err u6019) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-in-transfer-window                      | (err u6020) |                                                                                                                             |
| sbtc-stacking-pool              | err-unhandled-request                           | (err u6021) |                                                                                                                             |
| sbtc-stacking-pool              | err-invalid-penalty-type                        | (err u6022) |                                                                                                                             |
| sbtc-stacking-pool              | err-already-disbursed                           | (err u6023) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-hand-off-contract                       | (err u6024) |                                                                                                                             |
| sbtc-stacking-pool              | err-parsing-btc-tx                              | (err u6025) |                                                                                                                             |
| sbtc-stacking-pool              | err-threshold-wallet-is-none                    | (err u6026) |                                                                                                                             |
| sbtc-stacking-pool              | err-tx-not-mined                                | (err u6027) |                                                                                                                             |
| sbtc-stacking-pool              | err-wrong-pubkey                                | (err u6028) |                                                                                                                             |
| sbtc-stacking-pool              | err-dust-remains                                | (err u6029) |                                                                                                                             |
| sbtc-stacking-pool              | err-balance-not-transferred                     | (err u6030) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-in-penalty-window                       | (err u6031) |                                                                                                                             |
| sbtc-stacking-pool              | err-rewards-already-disbursed                   | (err u6032) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-in-voting-window                        | (err u6033) |                                                                                                                             |
| sbtc-stacking-pool              | err-set-peg-state                               | (err u6034) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-protocol-caller                         | (err u6035) |                                                                                                                             |
| sbtc-stacking-pool              | err-out-of-range                                | (err u6036) |                                                                                                                             |
| sbtc-stacking-pool              | err-threshold-to-scriptpubkey                   | (err u6037) |                                                                                                                             |
| sbtc-stacking-pool              | err-mass-delegate-stack-extend                  | (err u6038) |                                                                                                                             |
| sbtc-stacking-pool              | err-wallet-consensus-reached-execution          | (err u6039) |                                                                                                                             |
| sbtc-stacking-pool              | err-vote-or                                     | (err u6040) |                                                                                                                             |
| sbtc-stacking-pool              | err-candidates-overflow                         | (err u6041) |                                                                                                                             |
| sbtc-stacking-pool              | err-stacking-permission-denied                  | (err u6042) |                                                                                                                             |
| sbtc-stacking-pool              | err-not-enough-locked-stx                       | (err u6043) |                                                                                                                             |
| sbtc-stacking-pool              | err-already-activated                           | (err u6044) |                                                                                                                             |
| sbtc-hand-off                   | err-current-pool-not-found                      | (err u7000) |                                                                                                                             |
| sbtc-hand-off                   | err-current-threshold-wallet                    | (err u7001) |                                                                                                                             |
| sbtc-hand-off                   | err-previous-pool-not-found                     | (err u7002) |                                                                                                                             |
| sbtc-hand-off                   | err-pool-cycle                                  | (err u7003) |                                                                                                                             |
| sbtc-hand-off                   | err-previous-threshold-wallet                   | (err u7004) |                                                                                                                             |
| sbtc-hand-off                   | err-parsing-btc-tx                              | (err u7005) |                                                                                                                             |
| sbtc-hand-off                   | err-tx-not-mined                                | (err u7006) |                                                                                                                             |
| sbtc-hand-off                   | err-not-in-transfer-window                      | (err u7007) |                                                                                                                             |
| sbtc-hand-off                   | err-balance-already-transferred                 | (err u7008) |                                                                                                                             |
| sbtc-hand-off                   | err-wrong-pubkey                                | (err u7009) |                                                                                                                             |
| sbtc-hand-off                   | err-peg-balance-not-sufficient                  | (err u7010) |                                                                                                                             |
| sbtc-hand-off                   | err-threshold-to-scriptpubkey                   | (err u7011) |                                                                                                                             |
| sbtc-testnet-debug-controller   | err-not-debug-controller                        | (err u9900) | The caller is not a debug controller.                                                                                       |
| sbtc-testnet-debug-controller   | err-no-transactions                             | (err u9901) | No transactions to simulate mining a block with.                                                                            |
<!--errors-->

## Unit testing

### Running tests

All unit tests for sBTC Mini are written in the Clarity language. (As opposed
to TypeScript like is usual for Clarinet projects.) These tests can be found in
the `./tests` folder.

To run all unit tests, invoke the testing script:
```
./scripts/test.sh
```

The test script uses a Clarinet run script to generate unit test stubs for all
test functions in the Clarity unit test contract and will then run those tests.

The purpose of this setup provides the following benefits:

1. The unit tests are written in the same language as the protocol (Clarity.)
2. Test stubs are generated and not checked in, meaning there is one source of
   truth.
3. Using Clarinet allows to make use of its test runner and code coverage report
   generation.

### Adding tests

To write unit tests, follow these steps:

1. Create a new Clarity contract in the `./tests` folder. It can have any name
   but it should end in `_test.clar`. Files that do not follow this convention
   are ignored. (For example: `my-contract_test.clar` will be included and
   `my-contract.clar` will not.)
2. Add the new Clarity contract to `Clarinet.toml`.
3. Write unit tests as public functions, the function name must start with `test-`.
4. Run `./scripts/test.sh` to run the new tests.

### Writing tests

Unit test functions should be public without parameters. If they return an `ok`
response of any kind, the test is considered to have passed whereas an `err`
indicates a failure. The failure value is printed so it can be used to provide a
helpful message. The body of the unit test is written like one would usually
write Clarity, using `try!` and `unwrap!` and so on as needed.

Example:

```clarity
(define-public (test-my-feature)
	(begin
		(unwrap! (contract-call? .some-project-contract my-feature) (err "Calling my-feature failed"))
		(ok true)
	)
)
```

### Prepare function

Sometimes you need to run some preparation logic that is common to all or
multiple unit tests. If the script detects a function called `prepare`, it will
be invoked before calling the unit test function itself. The `prepare` function
should return an `ok`, otherwise the test fails.

```clarity
(define-public (prepare)
	(begin
		(unwrap! (contract-call? .some-project-contract prepare-something) (err "Preparation failed"))
		(ok true)
	)
)

(define-public (test-something)
	;; prepare will be executed before running the test.
)
```

### Annotations

You can add certain comment annotations before unit test functions to add
information or modify behaviour. Annotations are optional.

| Annotation            | Description                                                                                                                                  |
|-----------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| `@name`               | Give the unit test a name, this text shows up when running unit tests.                                                                       |
| `@no-prepare`         | Do not call the `prepare` function before running this unit test.                                                                            |
| `@prepare`            | Override the default `prepare` function with another. The function name should follow the tag.                                               |
| `@caller`             | Override the default caller when running this unit test. Either specify an account name or standard principal prefixed by a single tick `'`. |
| `@mine-blocks-before` | Mine a number of blocks before running the test. The number of blocks should follow the tag.                                                 |

Examples:

```clarity
(define-public (prepare) (ok "Default prepare function"))

(define-public (custom-prepare) (ok "A custom prepare function"))

;; A test without any annotations
(define-public (test-zero) (ok true))

;; @name A normal test with a name, the prepare function will run before.
(define-public (test-one) (ok true))

;; @name This test will be executed without running the default prepare function.
;; @no-prepare
(define-public (test-two) (ok true))

;; @name Override the default prepare function, it will run custom-prepare instead.
;; @prepare custom-prepare
(define-public (test-three) (ok true))

;; @name This test will be called with tx-sender set to wallet_1 (from the settings toml file).
;; @caller wallet_1
(define-public (test-four) (ok true))

;; @name This test will be called with tx-sender set to the specified principal.
;; @caller 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG
(define-public (test-five) (ok true))

;; @name Five blocks are mined before this test is executed.
;; @mine-blocks-before 5
(define-public (test-six) (ok true))
```
