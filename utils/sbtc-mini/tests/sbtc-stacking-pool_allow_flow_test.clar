(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant public-key 0x0011223344556699001122334455669900112233445566990011223344556699 )

;;; errors ;;;
(define-constant err-error-expected (err u99001))

(define-constant err-not-signer (err u6000))
(define-constant err-allowance-not-set (err u6001))
(define-constant err-allowance-height (err u6002))
(define-constant err-already-pre-signer-or-signer (err u6003))
(define-constant err-not-in-registration-window (err u6004))
(define-constant err-pre-registration-delegate-stx (err u6005))
(define-constant err-pre-registration-delegate-stack-stx (err u6006))
(define-constant err-pre-registration-aggregate-commit (err u6007))
(define-constant err-public-key-already-used (err u6008))
(define-constant err-pox-address-re-use (err u6009))
(define-constant err-not-enough-stacked (err u6010))
(define-constant err-wont-unlock (err u6011))
(define-constant err-voting-period-closed (err u6012))
(define-constant err-already-voted (err u6013))
(define-constant err-decrease-forbidden (err u6014))
(define-constant err-pre-registration-stack-increase (err u6015))
(define-constant err-not-in-good-peg-state (err u6016))
(define-constant err-unwrapping-candidate (err u6017))
(define-constant err-pool-cycle (err u6018))
(define-constant err-too-many-candidates (err u6019))
(define-constant err-not-in-transfer-window (err u6020))
(define-constant err-unhandled-request (err u6021))
(define-constant err-invalid-penalty-type (err u6022))
(define-constant err-already-disbursed (err u6023))
(define-constant err-not-hand-off-contract (err u6024))
(define-constant err-parsing-btc-tx (err u6025))
(define-constant err-threshold-wallet-is-none (err u6026))
(define-constant err-tx-not-mined (err u6027))
(define-constant err-wrong-pubkey (err u6028))
(define-constant err-dust-remains (err u6029))
(define-constant err-balance-not-transferred (err u6030))
(define-constant err-not-in-penalty-window (err u6031))
(define-constant err-rewards-already-disbursed (err u6032))
(define-constant err-not-in-voting-window (err u6033))
(define-constant err-set-peg-state (err u6034))
(define-constant err-not-protocol-caller (err u6035))
(define-constant err-threshold-percent-out-of-range (err u6036))
(define-constant err-threshold-to-scriptpubkey (err u6037))
(define-constant err-mass-delegate-stack-extend (err u6038))
(define-constant err-wallet-consensus-reached-execution (err u6039))
(define-constant err-vote-or (err u6040))
(define-constant err-candidates-overflow (err u6041))
(define-constant err-stacking-permission-denied (err u6042))

;; @name user can't sign pre-register when allowance removed
;; @caller wallet_1
(define-public (test-sign-pre-register-with-disallow)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
		;; @caller wallet_1
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test none) (err u113))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool disallow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test) (err u114))
		;; @mine-blocks-before 5
		(try! (check-sign-pre-register-disallowed))
		(ok true)
	)
)

;; @name user can pre-register only when allowance does expire in the future
;; @caller wallet_1
;; TODO: failing for me but out of scope for this PR.
(define-public (test-sign-pre-register-with-expired-allowance-in-future)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test (some u7)) (err u113))
		;; @mine-blocks-before 5
		(try! (check-sign-pre-register-disallowed-pox-3))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool (some u7)) (err u112))
		;; @mine-blocks-before 1
		(try! (check-sign-pre-register-disallowed-pox-3-height))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool (some u12)) (err u112))
		;; @mine-blocks-before 1
		(try! (check-sign-pre-register-disallowed))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test (some u12)) (err u114))
		;; @mine-blocks-before 1
		(try! (check-sign-pre-register-allowed))
		(ok true)
	)
)

(define-public (check-sign-pre-register-allowed)
	(begin
		(let ((actual (check-sign-pre-register)))
			(asserts! (is-ok actual) actual)
			(ok true))
	)
)



(define-public (check-sign-pre-register-disallowed-pox-3)
	(begin
		(let ((actual (check-sign-pre-register))
				(allowance (contract-call? .sbtc-stacking-pool get-allowance-contract-callers 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test)))
			(asserts! (is-err actual) actual)
			(asserts! (is-eq actual err-allowance-not-set) actual)
			(ok true))
	)
)

(define-public (check-sign-pre-register-disallowed-pox-3-height)
	(begin
		(let ((actual (check-sign-pre-register))
				(allowance (contract-call? .sbtc-stacking-pool get-allowance-contract-callers 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test)))
			(asserts! (is-err actual) actual)
			(asserts! (is-eq actual err-allowance-height) actual)
			(ok true))
	)
)

(define-public (check-sign-pre-register-disallowed)
	(begin
		(let ((actual (check-sign-pre-register))
				(allowance (contract-call? .sbtc-stacking-pool get-allowance-contract-callers 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_allow_flow_test)))
			(asserts! (is-err actual) actual)
			(asserts! (is-eq actual err-stacking-permission-denied) actual)
			(ok true))
	)
)

(define-public (check-sign-pre-register)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u10000000000000 mock-pox-reward-wallet-1)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))
