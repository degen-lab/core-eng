(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-pox-reward-wallet-2 { version: 0x06, hashbytes: 0x2200112233445566990011223344556699001122334455669900112233445566 })
(define-constant mock-sbtc-wallet-1 { version: 0x06, hashbytes: 0x1122334455669900112233445566990011223344556699001122334455669900 })
(define-constant public-key-1 0x0011223344556699001122334455669900112233445566990011223344556699 )
(define-constant public-key-2 0x1122334455669900112233445566990011223344556699001122334455669900 )

(define-constant err-error-expected (err u99001))
(define-constant err-pool-cycle (err u6018))

(define-constant ok-vote-existing-candidate-lost (ok u0))
(define-constant ok-vote-existing-candidate-won (ok u1))
(define-constant ok-voted (ok u2))

;; @name user1 and user2 can pre-register, register and vote
;; user1 and user2 stacks 10m STX
(define-public (test-signer-pre-register-register-vote)
	(begin
        ;; @caller wallet_1
        (unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
        ;; @continue
        (unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
        ;; @continue
        (unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_two_signers_flow_test none) (err u113))
        ;; @mine-blocks-before 5
		(try! (check-sign-pre-register-1))
        ;; @caller wallet_2
        (unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u211))
        ;; @continue
        (unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u212))
        ;; @continue
        (unwrap! (contract-call? .sbtc-stacking-pool allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool_two_signers_flow_test none) (err u213))
        ;; @mine-blocks-before 5
        (try! (check-sign-pre-register-2))        
        ;; @mine-blocks-before 2100
        ;; @caller wallet_1
		(try! (check-sign-register-1))
        ;; @mine-blocks-before 1
        ;; @caller wallet_2
        (try! (check-sign-register-2))
        ;; @continue
        (try! (check-is-inactive-1))
        ;; @continue
        (try! (check-is-inactive-2))
        ;; @caller wallet_1
        (unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u6300, unlocked: u10000000000000}) (err u111))
        ;; @caller wallet_2
        (unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG {locked: u10000000000000, unlock-height: u6300, unlocked: u10000000000000}) (err u211))
        ;; @mine-blocks-before 1600
        ;; @caller wallet_1
		(try! (check-vote-1))
        ;; @mine-blocks-before 1
        ;; @caller wallet_2
        (try! (check-vote-2))
        ;; @continue
        (try! (check-is-inactive-1))
        ;; @continue
        (try! (check-is-active-2))
        (ok true))
)

(define-public (check-sign-pre-register-1)
    (let
        ((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u10000000000000 mock-pox-reward-wallet-1)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-sign-pre-register-2)
    (let
        ((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u10000000000000 mock-pox-reward-wallet-2)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-sign-register-1)
    (let
        ((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key-1)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-sign-register-2)
    (let
        ((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG u10000000000000 mock-pox-reward-wallet-1 public-key-2)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))

(define-public (check-vote-1)
    (let
        ((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-ok vote-result) (err (unwrap-err-panic vote-result)))
            (asserts! (is-eq vote-result ok-voted) (err (unwrap-panic vote-result)))
			(ok true)))

(define-public (check-vote-2)
    (let
        ((vote-result
				(contract-call? .sbtc-stacking-pool vote-for-threshold-wallet-candidate mock-sbtc-wallet-1)))
			(asserts! (is-ok vote-result) (err (unwrap-err-panic vote-result)))
            ;;(asserts! (is-eq vote-result ok-vote-existing-candidate-won) (err (unwrap-panic vote-result)))
			(ok true)))

(define-public (check-is-inactive-1)
    (let ((result (contract-call? .sbtc-stacking-pool is-active-in-cycle u1)))
        (asserts! (is-err result) err-error-expected)
		(asserts! (is-eq result err-pool-cycle) result)
		(ok true)))

(define-public (check-is-inactive-2)
    (let ((result (contract-call? .sbtc-stacking-pool is-active-in-cycle u1)))
        (asserts! (is-err result) err-error-expected)
		(asserts! (is-eq result err-pool-cycle) result)
		(ok true)))

(define-public (check-is-active-2)
    (let ((result (contract-call? .sbtc-stacking-pool is-active-in-cycle u2)))
		(asserts! (is-ok result) result)
		(ok true)))
