(define-constant mock-pox-reward-wallet-1 { version: 0x06, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })

;; cycle windows
(define-constant disbursement 0x00)
(define-constant registration 0x01)
(define-constant voting 0x02)
(define-constant transfer 0x03)
(define-constant penalty 0x04)
(define-constant bad-peg-state 0x05)

;; @name can not pre-register without allowing contract to manage stacking
;; @caller wallet_1
(define-public (test-sign-pre-register)
	(begin
		(let
			((registration-result
				(contract-call? .sbtc-stacking-pool signer-pre-register u1000 mock-pox-reward-wallet-1)))
			(asserts! (is-err registration-result) registration-result)
			(asserts! (is-eq (unwrap-err! registration-result registration-result) u1) registration-result))
			(ok true)))

;; @name transfer for unknown pool fails
(define-public (test-balance-was-transferred)
	(begin (let (
		(result (contract-call? .sbtc-peg-transfer relay-handoff-fulfillment u2101 0x 0x u1 u1 (list) 0x 0x 0x (list))))
		(asserts! (is-err result)
			(err "Should return error"))
		(asserts! (is-eq (unwrap-err-panic result) u0)
			(err (concat "Should return err u0, not " (error-to-string (unwrap-err-panic result)))))
		(ok true))))

;; errors from sbtc-peg-transfer
(define-private (error-to-string (error uint))
	(unwrap! (element-at? (list "err-current-pool-not-found") error) "unknown error"))