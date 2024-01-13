(define-constant sbtc-token-burnchain-lock-time u2100)
(define-constant max-uint128 u340282366920938463463374607431768211455)

(define-constant err-token-lock-failed (err u5100)) ;; The amount of tokens specified in the request is larger than the amount the user owns.
(define-constant err-cannot-set-allowance-for-self (err u5101)) ;; Tried to set an allowance for oneself, which is not allowed.
(define-constant err-operator-not-allowed (err u5102)) ;; The operator was not allowed to request the withdrawal. The allowance is zero or insufficient.
(define-constant err-invalid-destination (err u5203)) ;; The destination is not in a format that the protocol understands. Should be P2WPKH, P2WSH, or P2TR.

(define-constant err-no-sponsor (err u5103)) ;; Called a sponsored function but the transaction was not sponsored.

(define-read-only (get-expiry-burn-height)
	(+ burn-block-height sbtc-token-burnchain-lock-time)
)

(define-map allowances {sender: principal, operator: principal} uint)

(define-read-only (get-allowance (sender principal) (operator principal))
	(default-to u0 (map-get? allowances {sender: sender, operator: operator}))
)

;; #[allow(unchecked_data)]
(define-public (set-allowance (operator principal) (allowance uint))
	(begin
		;; TODO- burn and mint allowance token to protect this function with a post condition?
		(asserts! (not (is-eq contract-caller operator)) err-cannot-set-allowance-for-self)
		(ok (map-set allowances {sender: contract-caller, operator: operator} allowance))
	)
)

(define-private (is-allowed-operator-and-deduct (sender principal) (operator principal) (amount uint))
	(begin
		(asserts! (not (is-eq sender operator)) true)
		(let ((allowance (get-allowance sender operator)))
			(asserts! (>= allowance amount) false)
			(and
				(< allowance max-uint128)
				(map-set allowances {sender: sender, operator: operator} (- allowance amount))
			)
			true
		)
	)
)

(define-public (request-withdrawal (amount uint) (sender principal) (destination { version: (buff 1), hashbytes: (buff 32) }))
	(begin
		;; Check if the operator is allowed to request a withdrawal for sender for the specified amount.
		(asserts! (is-allowed-operator-and-deduct sender contract-caller amount) err-operator-not-allowed)
		;; Check if the protocol understands the destination by parsing it to a scriptpubkey.
		(unwrap! (contract-call? .sbtc-btc-tx-helper hashbytes-to-scriptpubkey destination) err-invalid-destination)
		;; Lock the tokens.
		(unwrap! (contract-call? .sbtc-token protocol-lock amount sender) err-token-lock-failed)
		;; Insert the request, returns the withdrawal request-id.
		(contract-call? .sbtc-registry insert-withdrawal-request amount sender (get-expiry-burn-height) destination 0x)
	)
)

(define-public (request-withdrawal-sponsored (amount uint) (sender principal) (destination { version: (buff 1), hashbytes: (buff 32) }) (fee uint))
	(begin
		;; Pay the fee.
		(try! (contract-call? .sbtc-token protocol-transfer fee sender (unwrap! tx-sponsor? err-no-sponsor)))
		;; Request the withdrawal.
		(request-withdrawal amount sender destination)
	)
)
