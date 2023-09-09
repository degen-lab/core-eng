(define-constant err-not-a-sbtc-wallet (err u4000)) ;; There is no peg wallet address for the specified wallet.
(define-constant err-invalid-spending-pubkey (err u4001)) ;; The recipient of the BTC is not the same as the pubkey that unlocked the spending script.
(define-constant err-peg-value-not-found (err u4002)) ;; There was no output containing the peg wallet scriptPubKey.
(define-constant err-missing-witness (err u4003)) ;; The Taproot witness was missing.
(define-constant err-unlock-script-not-found-or-invalid (err u4004)) ;; The unlock script at the specified witness index did not exist or was invalid. (Not according to the sBTC spec.)

(define-constant err-script-invalid-opcode (err u4010)) ;; The opcode in the unlock script was invalid.
(define-constant err-script-invalid-version (err u4011)) ;; The version in the unlock script was invalid.
(define-constant err-script-not-op-drop (err u4012)) ;; The script does not contain OP_DROP at the expected offset.
(define-constant err-script-checksig-missing (err u4013)) ;; The script does not contain OP_CHECKSIG at the expected offset.
(define-constant err-script-missing-pubkey (err u4014)) ;; The script does not contain a Taproot pubkey.
(define-constant err-script-invalid-principal (err u4015)) ;; The encoded Stacks principal inside the script is invalid.
(define-constant err-script-invalid-length (err u4016)) ;; The length of the script is different from what is expected.

(define-constant type-standard-principal 0x05)
(define-constant type-contract-principal 0x06)

(define-constant version-P2TR 0x06)

(define-constant OP_DROP 0x75)
(define-constant OP_CHECKSIG 0xac)
(define-constant sbtc-opcode 0x3c)
(define-constant sbtc-deposit-payload-version 0x00)

;; --- Public functions

;; Bitcoin transactions must only contain one reveal per transaction.
(define-read-only (extract-sbtc-wallet-value (outs (list 8 { value: uint, scriptPubKey: (buff 128) })) (sbtc-wallet-scriptpubkey (buff 128)))
	(begin
		(match (element-at? outs u0) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u1) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u2) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u3) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u4) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u5) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u6) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		(match (element-at? outs u7) out (asserts! (not (is-eq (get scriptPubKey out) sbtc-wallet-scriptpubkey)) (some (get value out))) false)
		none
	)
)

;; offsets include varint length prefixes
(define-constant offset-opcode u1)
(define-constant offset-version u2)
(define-constant offset-principal u3)
(define-constant offset-principal-end (+ offset-principal u21))
(define-constant offset-op-drop u25)
(define-constant offset-sender-pubkey u26)
(define-constant offset-checksig u59)

(define-read-only (verify-extract-unlock-script (script (buff 128)))
	(begin
		(asserts! (is-eq (element-at? script offset-opcode) (some sbtc-opcode)) err-script-invalid-opcode)
		(asserts! (is-eq (element-at? script offset-version) (some sbtc-deposit-payload-version)) err-script-invalid-version)
		(let ((contract-name-length (match (element-at? script u24) length-byte (buff-to-uint-be length-byte) u0)))
			(asserts! (is-eq (element-at? script (+ offset-op-drop contract-name-length)) (some OP_DROP)) err-script-not-op-drop)
			(asserts! (is-eq (element-at? script (+ offset-checksig contract-name-length)) (some OP_CHECKSIG)) err-script-checksig-missing)
			(asserts! (is-eq (len script) (+ offset-checksig contract-name-length u1)) err-script-invalid-length)
			(ok {
				recipient:
					(unwrap! (from-consensus-buff? principal
						(if (is-eq contract-name-length u0)
							(concat type-standard-principal (unwrap! (slice? script offset-principal offset-principal-end) err-script-invalid-principal))
							(concat type-contract-principal (unwrap! (slice? script offset-principal (+ offset-principal u1 contract-name-length)) err-script-invalid-principal))
						)
					) err-script-invalid-principal),
				input-spending-pubkey: (unwrap! (slice? script (+ offset-sender-pubkey u1 contract-name-length) (+ offset-sender-pubkey u1 contract-name-length u32)) err-script-missing-pubkey)
				}
			)
		)
	)
)

(define-public (complete-deposit
	(cycle uint)
	(burn-height uint)
	(tx (buff 4096))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(witness-merkle-root (buff 32))
	(witness-reserved-data (buff 32))
	(witness-input-index uint)
	(ctx (buff 1024))
	(cproof (list 14 (buff 32)))
	)
	(let (
		;; Check if the tx was mined and get the parsed tx.
		(burn-tx (try! (contract-call? .sbtc-btc-tx-helper was-segwit-tx-mined burn-height tx header tx-index tree-depth wproof witness-merkle-root witness-reserved-data ctx cproof)))
		(burn-wtxid (get txid burn-tx))
		;; Retrieve the scriptpubkey of the current cycle.
		(sbtc-wallet (unwrap! (contract-call? .sbtc-btc-tx-helper get-sbtc-wallet-hashbytes-scriptpubkey (some cycle)) err-not-a-sbtc-wallet))
		;; Extract the value sent to the peg wallet (must be a single output)
		(value (unwrap! (extract-sbtc-wallet-value (get outs burn-tx) (get scriptpubkey sbtc-wallet)) err-peg-value-not-found))
		;; Find the protocol unlock witness script.
		;; It also checks if the protocol opcode and version byte are correct (script must start with 0x3c00).
		;; TODO: There is some duplication between the work the helper does and `verify-extract-unlock-script`. Room for optimisation.
		(unlock-script (unwrap! (contract-call? .sbtc-btc-tx-helper find-protocol-unlock-witness (unwrap! (element-at? (get witnesses burn-tx) witness-input-index) err-missing-witness)) err-unlock-script-not-found-or-invalid))
		;; extract the destination principal and unlocking sbtc-wallet pubkey from the unlock script.
		(extracted-script (try! (verify-extract-unlock-script unlock-script)))
		)
		;; TODO: We have to decide if we want to limit the number of
		;;       cycles one can go back. The question is how to handle
		;;       the situation of an older signer group moving BTC
		;;       from an escrow wallet to somewhere else.

		;; Check if the tx has not been processed before and if the
		;; mined deposit reached the minimum amount of confirmations.
		(try! (contract-call? .sbtc-registry assert-new-burn-wtxid-and-height burn-wtxid burn-height))
		;; Check if the recipient is the same as the unlock script spending pubkey.
		(asserts! (is-eq {version: version-P2TR, hashbytes: (get input-spending-pubkey extracted-script)} (get hashbytes sbtc-wallet)) err-invalid-spending-pubkey)
		;; Print deposit event.
		(print {event: "deposit", wtxid: burn-wtxid, value: value, recipient: (get recipient extracted-script)}) ;; TODO: define protocol events
		;; Mint the tokens.
		(contract-call? .sbtc-token protocol-mint value (get recipient extracted-script))
	)
)
