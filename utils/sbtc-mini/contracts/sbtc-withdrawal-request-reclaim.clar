(define-constant withdrawal-state-reclaimed 0x02)

(define-constant err-withdrawal-not-epxired (err u5300)) ;; The withdrawal request has not yet hit the expiry burn block height.

;; Unlocks the sBTC tokens after expiry
(define-public (reclaim-locked-tokens (withdrawal-request-id uint))
	(let (
		;; Get the pending withdrawal and mark it as settled.
		;; The call will fail if the request is no longer pending.
		(withdrawal-request (try! (contract-call? .sbtc-registry get-and-settle-pending-withdrawal-request withdrawal-request-id withdrawal-state-reclaimed)))
		)
		;; Check if the withdrawal request has expired (pending check is done above).
		(asserts! (<= (get expiry-burn-height withdrawal-request) burn-block-height) err-withdrawal-not-epxired)
		;; Unlock the locked user tokens.
		(contract-call? .sbtc-token protocol-unlock (get value withdrawal-request) (get sender withdrawal-request))
	)
)
