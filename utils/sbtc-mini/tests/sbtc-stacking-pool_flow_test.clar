(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant public-key 0x0011223344556699001122334455669900112233445566990011223344556699 )
;; @name user can pre-register
;; @caller wallet_1
(define-public (test-sign-pre-register)
	(begin
        ;; @continue
        (unwrap! (contract-call? .pox-3 mock-set-stx-account 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 {locked: u10000000000000, unlock-height: u4200, unlocked: u10000000000000}) (err u111))
        ;; @continue
        (unwrap! (contract-call? .pox-3 allow-contract-caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.sbtc-stacking-pool none) (err u112))
        ;; @continue
        (try! (check-pox-info))
        ;; @mine-blocks-before 5
		(try! (check-sign-pre-register))
        ;; @mine-blocks-before 2100
		(try! (check-sign-register))
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

(define-public (check-sign-register)
    (let
        ((registration-result
				(contract-call? .sbtc-stacking-pool signer-register 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5 u10000000000000 mock-pox-reward-wallet-1 public-key)))
			(asserts! (is-ok registration-result) registration-result)
			(ok true)))