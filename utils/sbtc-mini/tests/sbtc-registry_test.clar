(define-constant mock-wtxid-1 0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f)

(define-constant mock-peg-wallet-1 { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-peg-wallet-2 { version: 0x01, hashbytes: 0xaa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00 })

(define-constant mock-destination { version: 0x01, hashbytes: 0x00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff })
(define-constant mock-unlock-script 0x00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff0011223344556677)

(define-constant burnchain-confirmations-required u4)

(define-constant peg-out-state-requested 0x00)
(define-constant peg-out-state-fulfilled 0x01)
(define-constant peg-out-state-reclaimed 0x02)

(define-constant err-burn-tx-already-processed (err u600))
(define-constant err-peg-wallet-already-set (err u602))
(define-constant err-minimum-burnchain-confirmations-not-reached (err u603))
(define-constant err-not-settled-state (err u604))
(define-constant err-invalid-txid-length (err u605))
(define-constant err-unknown-peg-out-request (err u606))
(define-constant err-peg-out-not-pending (err u607))

(define-private (assert-eq (result (response bool uint)) (compare (response bool uint)) (message (string-ascii 100)))
	(ok (asserts! (is-eq result compare) (err message)))
)

(define-private (assert-all-eq-iter (item (response bool uint)) (state {compare: (response bool uint), result: bool}))
	{
	compare: (get compare state),
	result: (and (get result state) (is-eq item (get compare state)))
	}
)

(define-private (assert-all-eq (results (list 32 (response bool uint))) (compare (response bool uint)) (message (string-ascii 100)))
	(ok (asserts! (get result (fold assert-all-eq-iter results {compare: compare, result: true})) (err message)))
)

(define-public (prepare)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

;; @name Unique wtxid with the minimum amount of confirmations needed is accepted
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-and-height)
	(begin
		(unwrap!
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			(err "Should have succeeded")
			)
		(ok true)
	)
)

;; @name Unique wtxid without the minimum amount of confirmations needed is NOT accepted
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-and-insufficient-confirmations)
	(assert-eq
		(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 (- burn-block-height u1))
		err-minimum-burnchain-confirmations-not-reached
		"Should have failed with err-minimum-burnchain-confirmations-not-reached"
	)
)

;; @name A wtxid can only be accepted once
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-uniqueness)
	(begin
		(try! (assert-eq
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			(ok true)
			"Should have succeeded"
			))
		(assert-eq
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height mock-wtxid-1 u1)
			err-burn-tx-already-processed
			"Should have failed with err-burn-tx-already-processed"
			)
	)
)

;; @name wtxid must be 32 bytes
;; @mine-blocks-before 5
(define-public (test-assert-wtxid-length)
	(assert-all-eq
		(list
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x0000000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x000000000000000000000000000000000000000000000000000000000000 u1)
			(contract-call? .sbtc-registry assert-new-burn-wtxid-and-height 0x00000000000000000000000000000000000000000000000000000000000000 u1)
		)
		err-invalid-txid-length
		"Should have all failed with err-invalid-txid-length"
	)
)

;; @name Cannot set a peg wallet for a cycle that already has one 
;; @mine-blocks-before 5
(define-public (test-insert-duplicate-cycle-peg-wallet)
   (begin
     (unwrap! (contract-call? .sbtc-registry insert-cycle-peg-wallet u1 mock-peg-wallet-1) (err "insert-cycle-peg-wallet should have succeeded"))
     (unwrap! (contract-call? .sbtc-registry insert-cycle-peg-wallet u1 mock-peg-wallet-1) (ok true))
     (err "Should have all failed with err-peg-wallet-already-set")
   ) 
)

;; @name Get peg wallet for cycle with none set returns none
;; @mine-blocks-before 5
(define-public (test-get-none-cycle-peg-wallet)
  (begin
    (unwrap! (contract-call? .sbtc-registry get-cycle-peg-wallet u10) (ok true))
    (err "Peg wallet for cycle should be none")
  )
)

;; @name Get cycle for peg wallet not set returns none
;; @mine-blocks-before 5
(define-public (test-get-none-peg-wallet-cycle)
  (begin
    (unwrap! (contract-call? .sbtc-registry get-peg-wallet-cycle mock-peg-wallet-2) (ok true))
    (err "Peg wallet cycle should be none")
  )
)

;; @name Cannot settle a peg-out request id that is not pending
;; @mine-blocks-before 5
(define-public (test-settle-non-pending-peg-out-request)
  (begin
    (unwrap! (contract-call? .sbtc-registry get-and-settle-pending-peg-out-request u10 peg-out-state-fulfilled) (ok true))
    (err "Should not be settle because req id was not pending")
  )
)

;; @name Fail to get a peg-out request that does not exist
;; @mine-blocks-before 5
(define-public (test-get-unknown-peg-out-request)
  (begin
    (unwrap! (contract-call? .sbtc-registry get-peg-out-request u10) (ok true))
    (err "Peg-out request should be unknown")
  )
)

;; ;; @name Good increments of peg-out nonce
;; ;; @mine-blocks-before 5
(define-public (test-peg-out-request-nonce-increment)
  (let (
    (initial-nonce (contract-call? .sbtc-registry get-peg-out-nonce))
    (nonce1 (unwrap! (contract-call? .sbtc-registry insert-peg-out-request u1 tx-sender u100 mock-destination mock-unlock-script) (err "insert-peg-out-request should have succeeded")))
    (nonce2 (unwrap! (contract-call? .sbtc-registry insert-peg-out-request u1 tx-sender u100 mock-destination mock-unlock-script) (err "insert-peg-out-request should have succeeded")))
    (nonce3 (unwrap! (contract-call? .sbtc-registry insert-peg-out-request u1 tx-sender u100 mock-destination mock-unlock-script) (err "insert-peg-out-request should have succeeded")))
    )
  (asserts! (is-eq nonce1 (+ initial-nonce u0)) (err "Peg-out request nonce for req1 should be u1"))
  (asserts! (is-eq nonce2 (+ initial-nonce u1)) (err "Peg-out request nonce for req2 should be u2"))
  (asserts! (is-eq nonce3 (+ initial-nonce u2)) (err "Peg-out request nonce for req3 should be u3"))
  (ok true)
  )
)

;; @name peg-out-requests-pending is properly incremented
;; @mine-blocks-before 5
(define-public (test-peg-out-requests-pending)
  (begin
    (unwrap! (contract-call? .sbtc-registry insert-peg-out-request u1 tx-sender u100 mock-destination mock-unlock-script) (err "insert-peg-out-request should have succeeded"))
    (unwrap! (contract-call? .sbtc-registry insert-peg-out-request u2 tx-sender u100 mock-destination mock-unlock-script) (err "insert-peg-out-request should have succeeded"))
    (asserts! (is-eq (contract-call? .sbtc-registry get-pending-wallet-peg-outs) u2) (err "Peg-out pending number should be 2"))
    (ok true)
  )
)
