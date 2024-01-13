(define-constant withdrawal-state-fulfilled 0x01)

(define-constant err-wrong-value (err u5200)) ;; The value transferred to the destination is less than the requested amount.
(define-constant err-invalid-destination (err u5201)) ;; The destination in the withdrawal request is invalid.
(define-constant err-old-burnchain-transaction (err u5202)) ;; The withdrawal fulfilment proof is older than the burn height at which the withdrawal request was made.

(define-read-only (extract-destination-value (outs (list 8 { value: uint, scriptPubKey: (buff 128) })) (destination-scriptpubkey (buff 128)))
	(+
		(match (element-at? outs u0) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u1) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u2) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u3) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u4) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u5) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u6) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
		(match (element-at? outs u7) out (if (is-eq (get scriptPubKey out) destination-scriptpubkey) (get value out) u0) u0)
	)
)

(define-public (relay-withdrawal-fulfilment
	(withdrawal-request-id uint)
	(burn-height uint)
	(tx (buff 4096))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(witness-merkle-root (buff 32))
	(witness-reserved-data (buff 32))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; Check if the tx was mined and get the parsed tx.
		(burn-tx (try! (contract-call? .sbtc-btc-tx-helper was-segwit-tx-mined burn-height tx header tx-index tree-depth wproof witness-merkle-root witness-reserved-data ctx cproof)))
		(burn-wtxid (get txid burn-tx))
		;; Get the pending withdrawal and mark it as settled.
		;; The call will fail if the request is no longer pending.
		(withdrawal-request (try! (contract-call? .sbtc-registry get-and-settle-pending-withdrawal-request withdrawal-request-id withdrawal-state-fulfilled)))
		(destination-scriptpubkey (unwrap! (contract-call? .sbtc-btc-tx-helper hashbytes-to-scriptpubkey (get destination withdrawal-request)) err-invalid-destination))
		(total-value (extract-destination-value (get outs burn-tx) destination-scriptpubkey))
		)
		;; The protocol does not actually care who fulfilled the withdrawal request. Anyone
		;; can pay the BTC, it does not have to come from the peg wallet.
		;; TODO: What if the user sends themselves some BTC for another reason? Then 
		;;       anyone can burn locked sBTC tokens that were not fulfilled.

		;; Check if the tx has not been processed before and if it
		;; reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; Check if the transaction was mined after the withdrawal request was submitted.
		(asserts! (> (get burn-height withdrawal-request) burn-height) err-old-burnchain-transaction)
		;; check if the requested value was paid
		;; possible feature: allow transactions to partially withdrawal a request instead of
		;; all-or-nothing.
		(asserts! (>= total-value (get value withdrawal-request)) err-wrong-value)
		;; Burn the locked user tokens. If the BTC transaction sent
		;; more than the user was pegging out, then they got lucky.
		(contract-call? .sbtc-token protocol-burn-locked (get value withdrawal-request) (get sender withdrawal-request))
	)
)

