(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-sbtc-wallet-1 { version: 0x06, hashbytes: 0x1122334455669900112233445566990011223344556699001122334455669900 })
(define-constant public-key 0x0011223344556699001122334455669900112233445566990011223344556699 )
(define-constant public-key-2 0x1122334455669900112233445566990011223344556699001122334455669900 )

(define-constant err-ok-expected (err u99001))
(define-constant err-error-expected (err u99002))

(define-constant err-not-enough-stacked (err u6010))

;; @name user can pre-register, register, but vote fails due to not enough locked STX
;; user stacks 10k STX only
;; @caller wallet_1
(define-public (test-inactive-state)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_inactive_flow_test none) (err u113))
		;; @mine-blocks-before 5
		(try! (check-sign-pre-register))
		;; @mine-blocks-before 2100
		(try! (check-sign-register))
		;; @mine-blocks-before 1600
		(try! (check-vote-inactive))
		;; @mine-blocks-before 1
		(try! (check-active-in-cycle-inactive))
		(ok true))
)

(define-public (check-sign-pre-register)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u10000000000 mock-pox-reward-wallet-1)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-sign-register)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000 mock-pox-reward-wallet-1 public-key)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))


(define-public (check-vote-inactive)
	(let
		((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-err vote-result) err-ok-expected)
			(asserts! (is-eq vote-result err-not-enough-stacked) (err (unwrap-err-panic vote-result)))
			(ok true)))

(define-public (check-active-in-cycle-inactive)
	(let ((active-result (contract-call? .sbtc-stacking-pool is-active-in-cycle u2)))
		(asserts! (is-err active-result) err-error-expected)
		(asserts! (is-eq active-result err-not-enough-stacked) active-result)
		(ok true)))