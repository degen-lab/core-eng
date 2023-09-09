(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-2 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG)
(define-constant test-mint-amount    u10000000)
(define-constant test-allowance      u50000)
(define-constant test-withdrawal-amount u6000000)
(define-constant test-total-supply (* u3 test-mint-amount))

(define-constant contract-principal (as-contract tx-sender))

(define-constant withdrawal-state-requested 0x00)
(define-constant withdrawal-state-fulfilled 0x01)
(define-constant withdrawal-state-reclaimed 0x02)

(define-constant sbtc-token-burnchain-lock-time u2100)

(define-constant err-token-lock-failed (err u5100))
(define-constant err-cannot-set-allowance-for-self (err u5101))
(define-constant err-operator-not-allowed (err u5102))
(define-constant err-invalid-destination (err u5203))
(define-constant err-no-sponsor (err u5103))

(define-constant version-P2TR 0x06)

;; bcrt1p38e4lrh823h8w79lrgflz3etk63hcvtyl5a4l4u0n6l0cgcp8pxqdw342z
(define-constant test-recipient-destination { version: version-P2TR, hashbytes: 0x89f35f8ee7546e7778bf1a13f1472bb6a37c3164fd3b5fd78f9ebefc2301384c })

(define-read-only (calculate-expiry-height (burn-height uint))
	(+ burn-height sbtc-token-burnchain-lock-time)
)

(define-public (prepare-add-request-contract-to-protocol)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract .sbtc-withdrawal-request-stx true)
)

(define-public (prepare-add-test-to-protocol)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

;; Prepare function called for all tests (unless overridden)
(define-public (prepare)
	(begin
		;; Add the test contract to the protocol contract set.
		(try! (prepare-add-test-to-protocol))
		;; Add the .sbtc-withdrawal-request-stx contract to the protocol contract set.
		(try! (prepare-add-request-contract-to-protocol))
		;; Mint some tokens to test principals.
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount contract-principal))
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount wallet-1))
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount wallet-2))
		(ok true)
	)
)

;; Prepare function that sets allowance of wallet-1 for contract-caller
(define-public (prepare-and-set-allowance)
	(begin
		(try! (prepare))
		(try! (contract-call? .sbtc-withdrawal-request-stx set-allowance wallet-1 test-allowance))
		(ok true)
	)
)

;; --- Withdrawal request tests, Stacks side

;; @name contract-caller can set allowance
;; @prepare prepare-add-test-to-protocol
(define-public (test-set-allowance)
	(begin
		(unwrap! (contract-call? .sbtc-withdrawal-request-stx set-allowance wallet-2 test-allowance) (err "set-allowance returned false"))
		(asserts! (is-eq (contract-call? .sbtc-withdrawal-request-stx get-allowance contract-principal wallet-2) test-allowance) (err "Allowance not equal to the set allowance"))
		(ok true)
	)
)

;; @name cannot set allowance for self
;; @prepare prepare-add-test-to-protocol
(define-public (test-set-allowance-no-self)
	(begin
		(asserts!
			(is-eq (contract-call? .sbtc-withdrawal-request-stx set-allowance contract-principal test-allowance) err-cannot-set-allowance-for-self)
			(err "Should have failed with err-cannot-set-allowance-for-self")
		)
		(ok true)
	)
)

;; @name request withdrawal
(define-public (test-request-withdrawal-basic)
	(contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount contract-principal test-recipient-destination)
)

;; @name request withdrawal, check if tokens are locked
(define-public (test-request-withdrawal-tokens-locked)
	(let (
		(withdrawal-request-id (unwrap! (contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount contract-principal test-recipient-destination) (err {msg: "Withdrawal request failed", actual: u0, expected: u0})))
		(balance-available (unwrap-panic (contract-call? .sbtc-token get-balance-available contract-principal)))
		(balance-locked (unwrap-panic (contract-call? .sbtc-token get-balance-locked contract-principal)))
		(balance-total (unwrap-panic (contract-call? .sbtc-token get-balance contract-principal)))
		(expected-balance (- test-mint-amount test-withdrawal-amount))
		)
		(asserts! (is-eq balance-locked test-withdrawal-amount) (err {msg: "Invalid amount of tokens locked", expected: test-withdrawal-amount, actual: balance-locked}))
		(asserts! (is-eq balance-available expected-balance) (err {msg: "Invalid amount of remaining balance", expected: expected-balance, actual: balance-available}))
		(asserts! (is-eq balance-total test-mint-amount) (err {msg: "Total balance should not have changed", expected: test-mint-amount, actual: balance-total}))
		(ok true)
	)
)

;; @name request withdrawal, check if it exists in the registry
(define-public (test-request-withdrawal-exists)
	(let (
		(withdrawal-request-id (unwrap! (contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount contract-principal test-recipient-destination) (err {msg: "Withdrawal request failed", expected: none, actual: none})))
		(withdrawal-request (unwrap! (contract-call? .sbtc-registry get-withdrawal-request withdrawal-request-id) (err {msg: "The returned withdrawal-request-id does not exist", expected: none, actual: none})))
		(expected
			{
				value: test-withdrawal-amount,
				sender: contract-principal,
				destination: test-recipient-destination,
				extra-data: 0x,
				burn-height: burn-block-height,
				expiry-burn-height: (calculate-expiry-height burn-block-height),
				state: withdrawal-state-requested
			}
			)
		)
		(asserts! (is-eq withdrawal-request expected) (err {msg: "Withdrawal request does not match", expected: (some expected), actual: (some withdrawal-request)}))
		(ok true)
	)
)

;; @name request withdrawal as operator of another sender, check if tokens are locked
;; @prepare prepare-and-set-allowance
;; FIXME: we cannot set contract-caller so we cannot test allowance right now.
;; (define-public (test-request-withdrawal-tokens-locked-allowance)
;; 	(let (
;; 		(withdrawal-request-id (unwrap! (contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount wallet-1 test-recipient-destination) (err {msg: "Withdrawal request failed", actual: u0, expected: u0})))
;; 		(balance-available (unwrap-panic (contract-call? .sbtc-token get-balance-available wallet-1)))
;; 		(balance-locked (unwrap-panic (contract-call? .sbtc-token get-balance-locked wallet-1)))
;; 		(balance-total (unwrap-panic (contract-call? .sbtc-token get-balance wallet-1)))
;; 		(expected-balance (- test-mint-amount test-withdrawal-amount))
;; 		)
;; 		(asserts! (is-eq balance-locked test-withdrawal-amount) (err {msg: "Invalid amount of tokens locked", expected: test-withdrawal-amount, actual: balance-locked}))
;; 		(asserts! (is-eq balance-available expected-balance) (err {msg: "Invalid amount of remaining balance", expected: expected-balance, actual: balance-available}))
;; 		(asserts! (is-eq balance-total test-mint-amount) (err {msg: "Total balance should not have changed", expected: test-mint-amount, actual: balance-total}))
;; 		(ok true)
;; 	)
;; )

;; @name request withdrawal as operator of another sender, check if it exists in the registry
;; FIXME: we cannot set contract-caller so we cannot test allowance right now.
;; (define-public (test-request-withdrawal-exists-allowance)
;; 	(let (
;; 		(withdrawal-request-id (unwrap! (contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount wallet-1 test-recipient-destination) (err {msg: "Withdrawal request failed", expected: none, actual: none})))
;; 		(withdrawal-request (unwrap! (contract-call? .sbtc-registry get-withdrawal-request withdrawal-request-id) (err {msg: "The returned withdrawal-request-id does not exist", expected: none, actual: none})))
;; 		(expected
;; 			{
;; 				value: test-withdrawal-amount,
;; 				sender: wallet-1,
;; 				destination: test-recipient-destination,
;; 				extra-data: 0x,
;; 				burn-height: burn-block-height,
;; 				expiry-burn-height: (calculate-expiry-height burn-block-height),
;; 				state: withdrawal-state-requested
;; 			}
;; 			)
;; 		)
;; 		(asserts! (is-eq withdrawal-request expected) (err {msg: "Withdrawal request does not match", expected: (some expected), actual: (some withdrawal-request)}))
;; 		(ok true)
;; 	)
;; )