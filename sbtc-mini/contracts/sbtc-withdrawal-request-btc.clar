(define-constant withdrawal-state-requested 0x00)
(define-constant withdrawal-state-fulfilled 0x01)
(define-constant withdrawal-state-reclaimed 0x02)

(define-constant sbtc-token-burnchain-lock-time u2100)

(define-constant err-token-lock-failed (err u5000)) ;; The amount of tokens specified in the request is larger than the amount the user owns.
(define-constant err-unacceptable-expiry-height (err u5001)) ;; The burnchain expiry height specified in the request is too short.

;; 0      2  3          11                76
;; |------|--|----------|-----------------|
;;  magic  op   amount       signature

(define-read-only (verify-extract-unlock-script (script (buff 128)))
	(if true (ok {
		sender: tx-sender,
		value: u1,
		destination: { version: 0x00, hashbytes: 0x },
		expiry-burn-height: burn-block-height
	}) (err u1))
)

(define-public (register-withdrawal-request 
	(burn-height uint)
	(tx (buff 4096))
	(p2tr-unlock-script (buff 128))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; check if the tx was mined (todo: segwit wtxid)
		;; #[filter(tx)]
		(burn-wtxid (try! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof 0x 0x ctx cproof)))
		;; get the withdrawal data
		;; #[filter(ts)]
		(withdrawal-data (try! (verify-extract-unlock-script p2tr-unlock-script)))
		)
		;; There are still open questions about this part of the API.
		;; We can submit the P2TR funding transaction along with unlock script
		;; and store it, but it seems quite hard to verify that the unlock
		;; script can actually spend the P2TR output in Clarity. We have to
		;; derive the witness program and compare it with the one in the 
		;; transaction.

		;; check if the tx has not been processed before and if it
		;; reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; check that the expiry height is acceptable
		(asserts! (>= (get expiry-burn-height withdrawal-data) (+ burn-block-height sbtc-token-burnchain-lock-time)) err-unacceptable-expiry-height)
		;; lock sender's the tokens
		(unwrap! (contract-call? .sbtc-token protocol-lock (get value withdrawal-data) (get sender withdrawal-data)) err-token-lock-failed)
		;; insert the request, returns the withdrawal request-id
		(contract-call? .sbtc-registry insert-withdrawal-request (get value withdrawal-data) (get sender withdrawal-data) (get expiry-burn-height withdrawal-data) (get destination withdrawal-data) p2tr-unlock-script)
	)
)
