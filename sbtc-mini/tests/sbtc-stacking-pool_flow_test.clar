(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-sbtc-wallet-1 { version: 0x06, hashbytes: 0x1122334455669900112233445566990011223344556699001122334455669900 })
(define-constant public-key 0x0011223344556699001122334455669900112233445566990011223344556699 )
(define-constant public-key-2 0x1122334455669900112233445566990011223344556699001122334455669900 )
(define-constant public-key-3 0x2222334455669900112233445566990011223344556699001122334455669900 )

(define-constant err-error-expected (err u99001))
(define-constant err-voting-period-closed (err u6012))
(define-constant err-already-voted (err u6013))
(define-constant err-pool-cycle (err u6018))
(define-constant err-already-pre-signer-or-signer (err u6003))
(define-constant err-not-pre-signed-or-current-signer (err u6044))

;; @name user can pre-register, register, vote & re-register
;; user stacks 10m STX
;; @caller wallet_1
(define-public (test-signer-pre-register-register-vote-re-register)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_flow_test none) (err u113))
		;; @continue
		(try! (check-pox-info))
		;; @mine-blocks-before 5
		(try! (check-sign-pre-register))
		;; @mine-blocks-before 2100
		(try! (check-sign-register))
		;; @mine-blocks-before 1600
		(try! (check-vote))
		;; @mine-blocks-before 500
		(try! (check-is-active-2))
		;; @mine-blocks-before 1
		(try! (check-is-active-3))
		;; @mine-blocks-before 1
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u6300, unlocked: u10000000000000}) (err u111))
		;; @continue
		(try! (check-sign-register-2))
		(ok true))
)

;; @name user can pre-register, register, & *not* register twice & *not* vote twice
;; user stacks 10m STX
;; @caller wallet_1
(define-public (test-signer-pre-register-register-cant-register-twice)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_flow_test none) (err u113))
		;; @continue
		(try! (check-pox-info))
		;; @mine-blocks-before 5
		(try! (check-sign-pre-register))
		;; @mine-blocks-before 2100
		(try! (check-sign-register))
		;; @mine-blocks-before 1
		(try! (check-sign-register-fail))
		;; @mine-blocks-before 1590
		(try! (check-vote-too-early))
		;; @mine-blocks-before 1
		(try! (check-vote))
		;; @continue
		(try! (check-vote-twice-fails))
		(ok true))
)


;; @name user can't register without pre-registering
;; @caller wallet_1
(define-public (test-signer-register-without-pre-registering)
	(begin
		;; @continue
		(unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
		;; @continue
		(unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
		;; @continue
		(unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_flow_test none) (err u113))
		;; @mine-blocks-before 2100
		(try! (check-sign-register-not-pre-signed))
		(ok true))
)


(define-public (check-pox-info)
	(let ((pox-info (unwrap-panic (contract-call? .pox-3 get-pox-info))))
		(asserts! (is-eq u2100 (get reward-cycle-length pox-info)) (err u221))
	(ok true)))

(define-public (check-sign-pre-register)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u10000000000000 mock-pox-reward-wallet-1)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))


(define-public (check-sign-register-not-pre-signed)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key-3)))
			(asserts! (is-err registration-result) err-error-expected)
			(asserts! (is-eq registration-result err-not-pre-signed-or-current-signer) (err (unwrap-err-panic registration-result)))
			(ok true)))

(define-public (check-sign-register)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))


(define-public (check-sign-register-2)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key-2)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-sign-register-fail)
	(let
		((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key-3)))
			(asserts! (is-err registration-result) err-error-expected)
			(asserts! (is-eq registration-result err-already-pre-signer-or-signer) (err (unwrap-err-panic registration-result)))
			(ok true)))


(define-public (check-vote)
	(let
		((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-ok vote-result) (err (unwrap-err-panic vote-result)))
			(ok true)))

(define-public (check-vote-twice-fails)
	(let
		((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-err vote-result) err-error-expected)
			(asserts! (is-eq vote-result err-already-voted) (err (unwrap-err-panic vote-result)))
			(ok true)))


(define-public (check-vote-too-early)
	(let
		((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-err vote-result) err-error-expected)
			(asserts! (is-eq vote-result err-voting-period-closed) (err (unwrap-err-panic vote-result)))
			(ok true)))

(define-public (check-is-active-2)
	(let ((result (contract-call? .sbtc-stacking-pool is-active-in-cycle u2)))
		(asserts! (is-ok result) result)
		(ok true)))

(define-public (check-is-active-3)
	(let ((result (contract-call? .sbtc-stacking-pool is-active-in-cycle u3)))
		(asserts! (is-err result) err-error-expected)
		(asserts! (is-eq result err-pool-cycle) result)
		(ok true)))