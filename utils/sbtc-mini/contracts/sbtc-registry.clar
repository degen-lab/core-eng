;; The registry tracks state that should survive a protocol upgrade.
;; It is the data storage component of the controller.

(define-constant withdrawal-state-requested 0x00)
(define-constant withdrawal-state-fulfilled 0x01)
(define-constant withdrawal-state-reclaimed 0x02)

;; Types of penalty errors
(define-constant penalty-unhandled-peg-state-change 0x00)
(define-constant penalty-new-wallet-consensus-failed 0x01)
(define-constant penalty-hand-off-failed 0x02)

(define-constant err-burn-tx-already-processed (err u2000)) ;; A burnchain TXID was processed (seen) before.
(define-constant err-sbtc-wallet-already-set (err u2001)) ;; A peg wallet address for the specified cycle was already set.
(define-constant err-minimum-burnchain-confirmations-not-reached (err u2002)) ;; The burnchain transaction did not yet reach the minimum amount of confirmation.
(define-constant err-not-settled-state (err u2003)) ;; The state passed to function `get-and-settle-pending-withdrawal-request` was not a settled state. (Fulfilled or cancelled.)
(define-constant err-invalid-txid-length (err u2004)) ;; The passed TXID byte length was not equal to 32.
(define-constant err-unknown-withdrawal-request (err u2005)) ;; The withdrawal request ID passed to `get-and-settle-pending-withdrawal-request` does not exist.
(define-constant err-withdrawal-not-pending (err u2006)) ;; The withdrawal request ID passed to `get-and-settle-pending-withdrawal-request` is not in a pending state.

(define-data-var burnchain-confirmations-required uint u4)
(define-map processed-burn-wtxids (buff 32) bool)

(define-map sbtc-wallets uint { version: (buff 1), hashbytes: (buff 32) })
(define-map sbtc-wallets-cycle { version: (buff 1), hashbytes: (buff 32) } uint)
(define-data-var withdrawal-request-nonce uint u0)

(define-data-var peg-state bool true)

(define-data-var withdrawal-requests-pending uint u0)
(define-map withdrawal-requests uint
	{
	value: uint,
	sender: principal,
	destination: { version: (buff 1), hashbytes: (buff 32) },
	extra-data: (buff 128),
	burn-height: uint,
	expiry-burn-height: uint
	})

(define-map withdrawal-request-state uint (buff 1))

(define-read-only (current-peg-state)
	(var-get peg-state)
)

(define-read-only (is-protocol-caller)
	(contract-call? .sbtc-controller is-protocol-caller contract-caller)
)

(define-read-only (is-burn-wtx-processed (txid (buff 32)))
	(map-get? processed-burn-wtxids txid)
)

(define-read-only (get-burnchain-confirmations-required)
	(ok (var-get burnchain-confirmations-required))
)

;; #[allow(unchecked_data)]
(define-public (set-burnchain-confirmations-required (minimum-confirmations uint))
	(begin
		(try! (is-protocol-caller))
		(ok (var-set burnchain-confirmations-required (if (> minimum-confirmations u0) minimum-confirmations u1)))
	)
)

(define-public (assert-new-burn-wtxid-and-height (txid (buff 32)) (burn-height uint))
	(begin
		(try! (is-protocol-caller))
		(asserts! (is-eq (len txid) u32) err-invalid-txid-length)
		(asserts! (map-insert processed-burn-wtxids txid true) err-burn-tx-already-processed)
		(ok (asserts! (<= (+ burn-height (var-get burnchain-confirmations-required)) burn-block-height) err-minimum-burnchain-confirmations-not-reached))
	)
)

(define-read-only (get-cycle-sbtc-wallet (cycle uint))
	(map-get? sbtc-wallets cycle)
)

(define-read-only (get-sbtc-wallet-cycle (sbtc-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(map-get? sbtc-wallets-cycle sbtc-wallet)
)

(define-public (insert-cycle-sbtc-wallet (cycle uint) (sbtc-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(begin
		(try! (is-protocol-caller))
		(asserts! (map-insert sbtc-wallets-cycle sbtc-wallet cycle) err-sbtc-wallet-already-set)
		(ok (asserts! (map-insert sbtc-wallets cycle sbtc-wallet) err-sbtc-wallet-already-set))
	)
)

(define-read-only (get-withdrawal-request (id uint))
	(some (merge
		(try! (map-get? withdrawal-requests id))
		{ state: (default-to withdrawal-state-requested (map-get? withdrawal-request-state id)) }
		)
	)
)

(define-read-only (get-withdrawal-request-state (id uint))
	(map-get? withdrawal-request-state id)
)

(define-read-only (get-withdrawal-nonce)
	(var-get withdrawal-request-nonce)
)

(define-read-only (get-pending-wallet-withdrawals)
	(var-get withdrawal-requests-pending)
)

;; to-discuss, placeholder for hand-off contract
(define-read-only (get-peg-balance)
	u1
)

;; Update peg-state
(define-public (set-peg-state (state bool))
	(begin
		(try! (is-protocol-caller))
		(var-set peg-state state)
		(ok state)
	)
)

;; #[allow(unchecked_data)]
(define-public (insert-withdrawal-request
	(value uint)
	(sender principal)
	(expiry-burn-height uint)
	(destination { version: (buff 1), hashbytes: (buff 32) })
	(extra-data (buff 128))
	)
	(let ((nonce (var-get withdrawal-request-nonce)))
		(try! (is-protocol-caller))
		(map-set withdrawal-requests nonce {value: value, sender: sender, destination: destination, extra-data: extra-data, burn-height: burn-block-height, expiry-burn-height: expiry-burn-height })
		(var-set withdrawal-request-nonce (+ nonce u1))
		(var-set withdrawal-requests-pending (+ (var-get withdrawal-requests-pending) u1))
		(ok nonce)
	)
)

;; Call and settle a pending withdrawal request in one go.
;; It will throw if the withdrawal is not pending.
;; It will update the state to a settled state and
;; decrement the pending withdrawal request count. Any
;; protocol function calling into this function needs
;; to make sure the transaction reverts if anything
;; goes wrong.
;; #[allow(unchecked_data)]
(define-public (get-and-settle-pending-withdrawal-request (id uint) (settled-state (buff 1)))
	(let ((request (unwrap! (map-get? withdrawal-requests id) err-unknown-withdrawal-request)))
		(try! (is-protocol-caller))
		(asserts! (is-eq (default-to withdrawal-state-requested (map-get? withdrawal-request-state id)) withdrawal-state-requested) err-withdrawal-not-pending)
		(asserts! (or (is-eq settled-state withdrawal-state-fulfilled) (is-eq settled-state withdrawal-state-reclaimed)) err-not-settled-state)
		(var-set withdrawal-requests-pending (- (var-get withdrawal-requests-pending) u1))
		(map-set withdrawal-request-state id settled-state)
		(ok request)
	)
)
