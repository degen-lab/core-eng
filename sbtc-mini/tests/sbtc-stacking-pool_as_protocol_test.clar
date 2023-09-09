
;; .sbtc-stacking-pool_as_protocol_test is added as protocol contract
;; in prepare function of this unit test.

(define-constant err-error-expected (err u9901))
(define-constant err-out-of-range (err u6036))

(define-public (prepare)
	(begin
		;; Add this contract to the protocol contract set.
		(try! (contract-call? .sbtc-testnet-debug-controller set-protocol-contract .sbtc-stacking-pool_as_protocol_test true))
		(ok true)
	)
)

;; @name Is protocol caller test (is not at first)
(define-public (test-is-protocol-caller)
	(let ((is-protocol-caller
			(contract-call? .sbtc-stacking-pool is-protocol-caller)))
		(asserts! (is-ok is-protocol-caller) is-protocol-caller)
        (asserts! (is-eq is-protocol-caller (ok true)) is-protocol-caller)
		(ok true)
	)
)

;; @name Update threshold percent can be called by protocol contract
(define-public (test-update-threshold-percent)
    (let ((result (contract-call? .sbtc-stacking-pool update-threshold-percent u555)))
        (asserts! (is-ok result) result)
        (ok true)
    )
)

;; @name Update threshold percent can't be called by protocol contract below minimum
(define-public (test-update-threshold-percent-below-minimum)
    (let ((result (contract-call? .sbtc-stacking-pool update-threshold-percent u499)))
		(asserts! (is-err result) err-error-expected)
        (asserts! (is-eq result err-out-of-range) result)
        (ok true)
    )
)

;; @name Update threshold percent can't be called by protocol contract above maximum
(define-public (test-update-threshold-percent-above-maximum)
    (let ((result (contract-call? .sbtc-stacking-pool update-threshold-percent u951)))
		(asserts! (is-err result) err-error-expected)
        (asserts! (is-eq result err-out-of-range) result)
        (ok true)
    )
)

;; @name Update minimum pool amount can be called by protocol contract
;; static minimum for stacking is: u50,000.000000 STX
(define-public (test-update-minimum-pool-amount-protocol)
	(let ((result (contract-call? .sbtc-stacking-pool update-minimum-pool-amount-for-activation u50000000000)))
		(asserts! (is-ok result) result)
		(ok true)
	)
)

;; @name Update minimum pool amount can't be called by protocol contract below minimum
;; static minimum for stacking is: u50,000.000000 STX
(define-public (test-update-minimum-pool-amount-protocol-below-minimum)
	(let ((result (contract-call? .sbtc-stacking-pool update-minimum-pool-amount-for-activation u100)))
		(asserts! (is-err result) err-error-expected)
        (asserts! (is-eq result err-out-of-range) result)
		(ok true)
	)
)