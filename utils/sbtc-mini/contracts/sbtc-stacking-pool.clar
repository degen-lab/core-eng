;; sbtc-mini-stacker-pool


;;;;; Cons, Vars & Maps ;;;;;

;;; constants ;;;
;; cycle windows
(define-constant disbursement 0x00)
(define-constant registration 0x01)
(define-constant voting 0x02)
(define-constant transfer 0x03)
(define-constant penalty 0x04)
(define-constant bad-peg-state 0x05)

;; state as "normal" suggesting that the pool is operating as expected / wasn't in a "bad state"
(define-constant normal-cycle-len u2016)
(define-constant normal-voting-period-len u300)
(define-constant normal-transfer-period-len u100)
(define-constant normal-penalty-period-len u100)

;; Burn POX address for penalizing stackers/signers
(define-constant pox-burn-address { version: 0x00, hashbytes: 0x0000000000000000000000000000000000000000000000000000000000000000})

;; Dust limit placeholder for checking that pox-rewards were disbursed (in sats)
(define-constant dust-limit u100)

;; Signer minimal is 10k STX
(define-constant signer-minimal u10000000000)

(define-constant pox-info (unwrap-panic (contract-call? .pox-3 get-pox-info)))
(define-constant stacking-threshold-25 u20000)

;;; oks ;;;
(define-constant ok-vote-existing-candidate-lost (ok u0))
(define-constant ok-vote-existing-candidate-won (ok u1))
(define-constant ok-voted (ok u2))

;;; errors ;;;
(define-constant const-first-pox-error   u60000)
(define-constant const-second-pox-error u600000)

(define-constant err-not-signer (err u6000))
(define-constant err-allowance-not-set (err u6001))
(define-constant err-allowance-height (err u6002))
(define-constant err-already-pre-signer-or-signer (err u6003)) ;; Attemping to register as a signer when already a signer or pre-sign when already a pre-signer
(define-constant err-not-in-registration-window (err u6004))
(define-constant err-pre-registration-delegate-stx (err u6005))
(define-constant err-pre-registration-delegate-stack-stx (err u6006))
(define-constant err-pre-registration-aggregate-commit (err u6007))
(define-constant err-public-key-already-used (err u6008))
(define-constant err-pox-address-re-use (err u6009))
(define-constant err-not-enough-stacked (err u6010))
(define-constant err-wont-unlock (err u6011))
(define-constant err-voting-period-closed (err u6012))
(define-constant err-already-voted (err u6013))
(define-constant err-decrease-forbidden (err u6014))
(define-constant err-pre-registration-stack-increase (err u6015))
(define-constant err-not-in-good-peg-state (err u6016))
(define-constant err-unwrapping-candidate (err u6017))
(define-constant err-pool-cycle (err u6018))
(define-constant err-too-many-candidates (err u6019))
(define-constant err-not-in-transfer-window (err u6020))
(define-constant err-unhandled-request (err u6021))
(define-constant err-invalid-penalty-type (err u6022))
(define-constant err-already-disbursed (err u6023))
(define-constant err-not-hand-off-contract (err u6024))
(define-constant err-parsing-btc-tx (err u6025))
(define-constant err-threshold-wallet-is-none (err u6026))
(define-constant err-tx-not-mined (err u6027))
(define-constant err-wrong-pubkey (err u6028))
(define-constant err-dust-remains (err u6029))
(define-constant err-balance-not-transferred (err u6030))
(define-constant err-not-in-penalty-window (err u6031))
(define-constant err-rewards-already-disbursed (err u6032))
(define-constant err-not-in-voting-window (err u6033))
(define-constant err-set-peg-state (err u6034))
(define-constant err-not-protocol-caller (err u6035))
(define-constant err-out-of-range (err u6036))
(define-constant err-threshold-to-scriptpubkey (err u6037))
(define-constant err-mass-delegate-stack-extend (err u6038))
(define-constant err-wallet-consensus-reached-execution (err u6039))
(define-constant err-vote-or (err u6040))
(define-constant err-candidates-overflow (err u6041))
(define-constant err-stacking-permission-denied (err u6042))
(define-constant err-already-activated (err u6043))
(define-constant err-not-pre-signed-or-current-signer (err u6044))

;;; variables ;;;

;; Minimum amount of 1m locked STX for the pool to be active
(define-data-var minimal-pool-amount-for-activation uint u1000000000000)

;; Threshold consensus (in 3 digit %)
(define-data-var threshold-consensus uint u700)

;; Highest reward cycle in which all rewards are disbursed (aka the last "good state" peg cycle
(define-data-var last-disbursed-burn-height uint u0)

;; Current cycle threshold wallet
(define-data-var threshold-wallet { version: (buff 1), hashbytes: (buff 32) } { version: 0x00, hashbytes: 0x00 })

;; Same burnchain and PoX constants as mainnet
(define-constant first-burn-block-height u666050)
(define-data-var reward-cycle-len uint u2100)

;; Relative burnchain block heights (between 0 and 2100) as to when the system transitions into different states
(define-data-var registration-window-rel-end uint u1600)
(define-data-var voting-window-rel-end uint u1900)
(define-data-var transfer-window-rel-end uint u2000)
(define-data-var penalty-window-rel-end uint u2100)

;;; maps ;;;

;; Map that tracks all relevant stacker data for a given pool (by cycle index)
(define-map stacking-details-by-cycle uint {
	stackers: (list 100 principal),
	stacked: uint,
	threshold-wallet-candidates: (list 100 { version: (buff 1), hashbytes: (buff 32) }),
	threshold-wallet: (optional { version: (buff 1), hashbytes: (buff 32) }),
	last-aggregation: (optional uint),
	reward-index: (optional uint),
	balance-transferred: bool,
	rewards-disbursed: bool
})

;; Map that tracks all stacker/signer data for a given principal & pool (by cycle index)
(define-map signer-registrations-by-stacker-cycle {stacker: principal, cycle: uint} {
	amount: uint,
	;; pox-addrs must be unique per cycle
	pox-addr: { version: (buff 1), hashbytes: (buff 32) },
	vote: (optional { version: (buff 1), hashbytes: (buff 32) }),
	public-key: (buff 33),
	lock-period: uint,
	btc-earned: (optional uint)
})

;; Map that tracks all votes per cycle
(define-map votes-per-cycle {cycle: uint, wallet-candidate: { version: (buff 1), hashbytes: (buff 32) } } {
	aggregate-commit-index: (optional uint),
	votes-in-ustx: uint,
	num-signer: uint,
})

;; Map that tracks all pre-signer stacker/signer sign-ups for a given principal & pool (by cycle index)
(define-map pre-signer {stacker: principal, cycle: uint} bool)

;; Map of reward cycle to pox reward set index.
(define-map pox-addr-indices uint uint)

;; Map of reward cyle to block height of last commit
(define-map last-aggregation uint uint)

;; Allowed contract-callers handling the sbtc stacking activity.
(define-map allowance-contract-callers
	{ sender: principal, contract-caller: principal}
	{until-burn-ht: (optional uint)})

;; All public keys (buff 33) ever used
(define-map public-keys-used (buff 33) bool)

;; All PoX addresses used per reward cycle (so we don't re-use them)
(define-map payout-address-in-cycle { version: (buff 1), hashbytes: (buff 32) } uint)


;;;;; Read-Only Functions ;;;;;

;; use MOCK function
(define-private (get-stx-account (user principal))
	(contract-call? .pox-3 get-stx-account user))

;; Check if caller is a protocol caller
(define-read-only (is-protocol-caller)
	(contract-call? .sbtc-controller is-protocol-caller contract-caller)
)

;; Get current cycle pool
(define-read-only (get-current-cycle-pool)
	(map-get? stacking-details-by-cycle (current-pox-reward-cycle))
)

;; Get specific cycle pool
(define-read-only (get-specific-cycle-pool (specific-cycle uint))
	(map-get? stacking-details-by-cycle specific-cycle)
)

;; Get signer in cycle
(define-read-only (get-signer-in-cycle (signer-principal principal) (cycle uint))
	(default-to {
		amount: u0,
		pox-addr: { version: 0x00, hashbytes: 0x00 },
		vote: none,
		public-key: 0x00,
		lock-period: u0,
		btc-earned: none
	} (map-get? signer-registrations-by-stacker-cycle {stacker: signer-principal, cycle: cycle}))
)

;; Get current window
(define-read-only (get-current-window)
	(let
		(
			(peg-state (contract-call? .sbtc-registry current-peg-state))
			(current-cycle (current-pox-reward-cycle))
			(current-cycle-burn-height (reward-cycle-to-burn-height current-cycle))
			(next-cycle (+ u1 current-cycle))
			(next-cycle-burn-height (reward-cycle-to-burn-height next-cycle))
			(latest-disbursed-burn-height (var-get last-disbursed-burn-height))
			(start-voting-window (- next-cycle-burn-height (+ normal-voting-period-len normal-transfer-period-len normal-penalty-period-len)))
			(start-transfer-window (- next-cycle-burn-height (+ normal-transfer-period-len normal-penalty-period-len)))
			(start-penalty-window (- next-cycle-burn-height normal-penalty-period-len))
		)

		(asserts! peg-state bad-peg-state)

		(asserts! (>= burn-block-height start-voting-window)
					(if (or (< current-cycle-burn-height latest-disbursed-burn-height) (is-eq latest-disbursed-burn-height u0))
						registration
						disbursement))
		(asserts! (>= burn-block-height start-transfer-window) voting)
		(asserts! (>= burn-block-height start-penalty-window) transfer)
		penalty ;; lasts until the end of the cycle
	)
)


;; Avoid loading pox-3 for conversion only
(define-read-only (reward-cycle-to-burn-height (cycle uint))
	(+ (get first-burnchain-block-height pox-info) (* cycle (get reward-cycle-length pox-info))))

(define-read-only (burn-height-to-reward-cycle (height uint))
	(/ (- height (get first-burnchain-block-height pox-info)) (get reward-cycle-length pox-info)))

(define-read-only (current-pox-reward-cycle)
	(burn-height-to-reward-cycle burn-block-height))

(define-read-only (was-enough-stx-stacked (locked-amount-ustx uint))
	(>= locked-amount-ustx (var-get minimal-pool-amount-for-activation)))

(define-read-only (is-active-in-cycle (cycle uint))
	(let ((pool-details (unwrap! (map-get? stacking-details-by-cycle cycle) err-pool-cycle)))
		(asserts! (was-enough-stx-stacked (get stacked pool-details)) err-not-enough-stacked)
		(ok true)))

;;;;;;; Disbursement Functions ;;;;;;;
;; Function that proves POX rewards have been disbursed from the previous threshold wallet to the previous pool signers. This happens in x steps:
;; 1. Fetch previous pool threshold-wallet / pox-reward-address
;; 2. Parse-tx to get all (8) outputs
;; 3. Check that for each output, the public-key matches the pox-reward-address
;; 4. Check that for each output, the amount is lower than constant dust
;; Note - this may be updated to later check against a specific balance

;; Disburse function for signers in (n - 1) to verify that their pox-rewards have been disbursed
(define-public (prove-rewards-were-disbursed
	(burn-height uint)
	(tx (buff 1024))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
	(witness-merkle-root (buff 32))
	(witness-reserved-data (buff 32))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32))))
	(let
		(
			(current-cycle (burn-height-to-reward-cycle block-height))
			(previous-cycle (- current-cycle u1))
			(previous-pool (unwrap! (map-get? stacking-details-by-cycle previous-cycle) err-pool-cycle))
			(unwrapped-previous-threshold-wallet (unwrap! (get threshold-wallet previous-pool) err-threshold-wallet-is-none))
			(previous-threshold-wallet (get hashbytes unwrapped-previous-threshold-wallet))
			(previous-threshold-wallet-version (get version unwrapped-previous-threshold-wallet))
			(previous-pool-disbursed (get rewards-disbursed previous-pool))
			(parsed-tx (unwrap! (contract-call? .clarity-bitcoin parse-tx tx) err-parsing-btc-tx))
			(tx-outputs (get outs parsed-tx))
			;; Done manually for read/write concerns
			(tx-output-0 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u0)))
			(tx-output-1 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u1)))
			(tx-output-2 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u2)))
			(tx-output-3 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u3)))
			(tx-output-4 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u4)))
			(tx-output-5 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u5)))
			(tx-output-6 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u6)))
			(tx-output-7 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u7)))

			;; versions + hashbytes to scriptPubKey
			(previous-unwrapped-threshold-pubkey (unwrap! (contract-call? .sbtc-btc-tx-helper hashbytes-to-scriptpubkey unwrapped-previous-threshold-wallet) err-threshold-to-scriptpubkey))
		)

			;; Assert we're in the disbursement window
			(asserts! (is-eq (get-current-window) disbursement)  err-not-in-registration-window)

			;; Assert that balance of previous-threshold-wallet was transferred
			(asserts! (get balance-transferred previous-pool) err-balance-not-transferred)

			;; Assert that rewards haven't already been disbursed
			(asserts! (not previous-pool-disbursed) err-already-disbursed)

			;; Assert that transaction was mined...tbd last two params
			(unwrap! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof witness-merkle-root witness-reserved-data ctx cproof) err-tx-not-mined)

			;; Assert that every unwrapped receiver addresss is equal to previous-threshold-wallet
			(asserts! (and
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-0))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-1))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-2))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-3))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-4))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-5))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-6))
				(is-eq previous-unwrapped-threshold-pubkey (get scriptPubKey tx-output-7))
			) err-wrong-pubkey)

			;; All POX rewards have been distributed, update relevant vars/maps
			(var-set last-disbursed-burn-height block-height)
			(ok (map-set stacking-details-by-cycle previous-cycle (merge
				previous-pool
				{rewards-disbursed: true}
			)))
	)
)



;;;;; Registration Functions ;;;;;

;; @desc: pre-registers a stacker for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-pre-register (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
	(let
		(
			(signer-account (get-stx-account tx-sender))
			(new-signer tx-sender)
			(signer-unlocked-balance (get unlocked signer-account))
			(signer-allowance-status (unwrap! (contract-call? .pox-3 get-allowance-contract-callers tx-sender (as-contract tx-sender)) err-allowance-not-set))
			(signer-allowance-end-height (get until-burn-ht signer-allowance-status))
			(current-cycle (current-pox-reward-cycle))
			(next-cycle (+ current-cycle u1))
			(current-pre-signer (map-get? pre-signer {stacker: tx-sender, cycle: current-cycle}))
			(current-signer (map-get? signer-registrations-by-stacker-cycle {stacker: tx-sender, cycle: current-cycle}))
		)

		;; Assert that amount-ustx is greater than signer-minimal
		(asserts! (>= amount-ustx signer-minimal) err-not-enough-stacked)

		;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
		(asserts! (or
			(is-none signer-allowance-end-height)
			(< burn-block-height (default-to burn-block-height signer-allowance-end-height))
		) err-allowance-height)

		;; assert that caller is allowed to handle stacking activity
		(asserts! (check-caller-allowed) err-stacking-permission-denied)

		;; Assert not already pre-signer or signer
		(asserts! (or (is-none current-pre-signer) (is-none current-signer)) err-already-pre-signer-or-signer)

		;; Assert we're in the registration window
		(asserts! (is-eq (get-current-window) registration)  err-not-in-registration-window)

		;; Delegate-stx to their PoX address
		(match (contract-call? .pox-3 delegate-stx amount-ustx (as-contract tx-sender) none none)
			success true
			error (asserts! false (err (+ (to-uint error) const-first-pox-error))))

		;; Delegate-stack-stx for next cycle
		(match (as-contract (contract-call? .pox-3 delegate-stack-stx new-signer amount-ustx pox-addr burn-block-height u1))
			success true
			error (asserts! false (err (+ (to-uint error) const-second-pox-error))))

		;; Stack aggregate-commit
		;; This fails when the user is already stacking.
		;; It does not matter because stx are already locked and ready to be extended in the next cycle
		(match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-addr next-cycle))
			ok-branch
				;; user's stx is ready to earn
				true
			err-branch
				;; user's stx won't earn for next cycle
				true
		)

		;; Record pre-signer
		(ok (map-set pre-signer {stacker: tx-sender, cycle: next-cycle} true))
	)
)

;; @desc: registers a signer for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-register (pre-registered-signer principal) (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}) (public-key (buff 32)))
	(let
		(
			(signer-account (get-stx-account pre-registered-signer))
			(signer-unlocked-balance (get unlocked signer-account))
			(signer-allowance-status (unwrap! (contract-call? .pox-3 get-allowance-contract-callers pre-registered-signer (as-contract tx-sender)) err-allowance-not-set))
			(signer-allowance-end-height (get until-burn-ht signer-allowance-status))
			(current-cycle (current-pox-reward-cycle))
			(previous-cycle (- current-cycle u1))
			(next-cycle (+ current-cycle u1))
			(current-pre-signer (map-get? pre-signer {stacker: pre-registered-signer, cycle: current-cycle}))
			(current-signer (map-get? signer-registrations-by-stacker-cycle {stacker: pre-registered-signer, cycle: current-cycle}))
			(previous-signer (map-get? signer-registrations-by-stacker-cycle {stacker: pre-registered-signer, cycle: previous-cycle}))
			(next-signer (map-get? signer-registrations-by-stacker-cycle {stacker: pre-registered-signer, cycle: next-cycle}))
			(pox-address-cycle-use (map-get? payout-address-in-cycle pox-addr))
		)

		;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
		(asserts! (or (is-none signer-allowance-end-height) (< burn-block-height (default-to burn-block-height signer-allowance-end-height))) err-allowance-height)

		;; Assert we're in a good-peg state & in the registration window
		(asserts! (is-eq (get-current-window) registration)  err-not-in-registration-window)

		;; Assert the public-key hasn't been used before
		(asserts! (is-none (map-get? public-keys-used public-key)) err-public-key-already-used)

		;; Assert that pox-address-cycle-use is either none or the result is not equal to the next cycle
		(asserts! (or
			(is-none pox-address-cycle-use)
			(not (is-eq (default-to u0 pox-address-cycle-use) next-cycle))
		) err-pox-address-re-use)

		;; Assert that user is not registered as a signer for the next-cycle
		(asserts! (is-none next-signer) err-already-pre-signer-or-signer)

		;; Assert that user is either pre-registered for the current-cycle or is a signer for the previous-cycle & voted
		(asserts! (or
			(is-some current-pre-signer)
			(and (is-some current-signer) (is-some (get vote current-signer)))
		) err-not-pre-signed-or-current-signer)

		;; Assert that pre-registered signer has at least the amount of STX to be stacked already locked (entire point of pre-registering)
		(asserts! (>= (get locked signer-account) amount-ustx) err-not-enough-stacked)

		;; Assert that pre-registered signer will unlock in the next cycle
		(asserts! (is-eq next-cycle (burn-height-to-reward-cycle (get unlock-height signer-account))) err-wont-unlock)

		;; update all relevant maps
		;; update signer map
		(map-set signer-registrations-by-stacker-cycle {stacker: tx-sender, cycle: next-cycle} {
			amount: amount-ustx,
			;; pox-addrs must be unique per cycle
			pox-addr: pox-addr,
			vote: none,
			public-key: public-key,
			lock-period: u0,
			btc-earned: none
		})
		;; need to set pool map
		;; check if first time next-cycle pool is set
		(ok (match (map-get? stacking-details-by-cycle next-cycle)
			;; next cycle pool already exists, update/merge
			next-pool
				(map-set stacking-details-by-cycle next-cycle (merge
					next-pool
					{
						stackers: (unwrap! (as-max-len? (append (get stackers next-pool) tx-sender) u100) err-too-many-candidates),
						stacked: (+ (get stacked next-pool) amount-ustx)
					}
				))
			;; next pool initial set
			(map-set stacking-details-by-cycle next-cycle
				{
					stackers: (list tx-sender),
					stacked: amount-ustx,
					threshold-wallet-candidates: (list ),
					threshold-wallet: none,
					last-aggregation: none,
					reward-index: none,
					balance-transferred: false,
					rewards-disbursed: false
				}
			)
		))

	)
)



;;;;; Voting Functions ;;;;;;;

;; @desc: Voting function for deciding the threshold-wallet/PoX address for the next pool & cycle, once a single wallet-candidate reaches 70% of the vote, stack-aggregate-index
(define-public (vote-for-threshold-wallet-candidate (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
	(let
		(
			(current-cycle (current-pox-reward-cycle))
			(next-cycle (+ current-cycle u1))
			(next-candidate-status (map-get? votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr}))
			(next-stacking-cycle-details (unwrap! (map-get? stacking-details-by-cycle next-cycle) err-pool-cycle))
			(next-cycle-stackers (get stackers next-stacking-cycle-details))
			(next-cycle-threshold-wallet (get threshold-wallet next-stacking-cycle-details))
			(next-cycle-total-stacked (get stacked next-stacking-cycle-details))
			(next-cycle-signer (unwrap! (map-get? signer-registrations-by-stacker-cycle {stacker: tx-sender, cycle: next-cycle}) err-not-signer))
			(next-cycle-signer-amount (get amount next-cycle-signer))
		)

		;; Assert active state
		(asserts! (was-enough-stx-stacked next-cycle-total-stacked) err-not-enough-stacked)

		;; Assert we're in a good-peg state
		(asserts! (contract-call? .sbtc-registry current-peg-state) err-not-in-good-peg-state)

		;; Assert we're in the voting window
		(asserts! (is-eq (get-current-window) voting) err-voting-period-closed)

		;; Assert signer hasn't voted yet
		(asserts! (is-none (get vote next-cycle-signer)) err-already-voted)

		;; Update signer map with vote
		(map-set signer-registrations-by-stacker-cycle {stacker: tx-sender, cycle: next-cycle} (merge
			next-cycle-signer
			{vote: (some pox-addr)}
		))


		(asserts!
			;; New candidate path
			(and
			   ;; Candidate doesn't exist, update relevant maps: votes-per-cycle & pool
			   (is-none next-candidate-status)
			   ;; Update votes-per-cycle map with first vote for this wallet-candidate
			   (map-set votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr} {
						aggregate-commit-index: none,
						votes-in-ustx: (get amount next-cycle-signer),
						num-signer: u1,
				})
				
				;; Update pool map by appending wallet-candidate to list of candidates
				(map-set stacking-details-by-cycle next-cycle (merge
					next-stacking-cycle-details
					{threshold-wallet-candidates: (unwrap! (as-max-len? (append (get threshold-wallet-candidates next-stacking-cycle-details) pox-addr) u100) err-candidates-overflow)}
				))

			)
			;; Existing candidate path
			(let
				(
					(unwrapped-candidate (unwrap-panic next-candidate-status))
					(unwrapped-candidate-votes (get votes-in-ustx unwrapped-candidate))
					(unwrapped-candidate-num-signer (get num-signer unwrapped-candidate))
					(new-candidate-votes (+ (get amount next-cycle-signer) (get votes-in-ustx unwrapped-candidate)))
				)

					;; Update votes-per-cycle map for existing candidate
					(map-set votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr} (merge
						unwrapped-candidate
						{
							votes-in-ustx: (+ next-cycle-signer-amount unwrapped-candidate-votes),
							num-signer: (+ u1 unwrapped-candidate-num-signer)
						}
					))

					;; Update signer map
					(map-set signer-registrations-by-stacker-cycle {stacker: tx-sender, cycle: next-cycle} (merge next-cycle-signer { vote: (some pox-addr) }))
					;; Asserts! logic to check if 70% wallet consensus has been reached
					(asserts!
						(and
							;; Assert that new-candidate-votes is greater than or equal to 70% of next-cycle-total-stacked
							(>= (/ (* new-candidate-votes u1000) next-cycle-total-stacked) (var-get threshold-consensus))

							;; Assert that extend-and-commit-bool is true (both delegate-stack-extend & aggregate-commit-indexed succeeded)
							(match (as-contract (fold mass-delegate-stack-extend next-cycle-stackers (ok {stacker: tx-sender, unlock-burn-height: u0, pox-addr: pox-addr})))
								success-extend
									(match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-addr next-cycle))
										;; Okay result, update pool map with last-aggregation (block-height) & reward-index
										success-commit
											(map-set stacking-details-by-cycle next-cycle (merge
												next-stacking-cycle-details
												{last-aggregation: (some block-height), reward-index: (some success-commit), threshold-wallet: (some pox-addr)}
											))
										err-commit
											;; Returning false to signify that commit failed
											(begin
   											(print (/ (* new-candidate-votes u1000) next-cycle-total-stacked))
												(print err-commit)
												false)
									)
								err-extend
									;; Returning false to signify that extend failed
									(begin
										(print err-extend)
										false)
							)
						)
						;; result when candidate did not reach 70% consensus or extend failed
						(begin
							(print (/ (* new-candidate-votes u1000) next-cycle-total-stacked))
							ok-vote-existing-candidate-lost)
					)
					;; result when candidate was accepted and stackers extended
					ok-vote-existing-candidate-won
			)
		)
		;; result new candidate path
		ok-voted
	)
)



;;;;;;; Transfer Functions ;;;;;;;

;; Transfer function for proving that current/soon-to-be-old signers have transferred the peg balance to the next threshold-wallet
;; Can only be called by the sbtc-hand-off/hand-off contract. If successful, balance-disbursed is set to true for the previous pool
(define-public (balance-was-transferred (previous-cycle uint))
	(let
		(
			(previous-pool (unwrap! (map-get? stacking-details-by-cycle previous-cycle) err-pool-cycle))
		)

			;; Assert that contract-caller is .sbtc-hand-off contract
			(asserts! (is-eq contract-caller .sbtc-hand-off) err-not-hand-off-contract)

			;; Hand-off success, update relevant vars/maps
			(ok (map-set stacking-details-by-cycle previous-cycle (merge
				previous-pool
				{balance-transferred: true}
			)))

	)
)



;;;;;;; Penalty Functions ;;;;;;;

;; Penalty function for an unexpired, unhandled request-post-vote
(define-public (penalty-unhandled-request)
	(let
		(
			(current-cycle (current-pox-reward-cycle))
			(next-cycle (+ current-cycle u1))
			(current-pool (unwrap! (map-get? stacking-details-by-cycle current-cycle) err-pool-cycle))
			(current-pool-stackers (get stackers current-pool))
			(next-pool (unwrap! (map-get? stacking-details-by-cycle next-cycle) err-pool-cycle))
		)

		;; Assert that we're in the transfer window
		(asserts! (is-eq (get-current-window) transfer) err-not-in-transfer-window)

		;; Assert that pending-wallet-withdrawals is not equal to zero
		(asserts! (> (contract-call? .sbtc-registry get-pending-wallet-withdrawals) u0) err-unhandled-request)

		;; Penalize stackers by re-stacking but with a pox-reward address of burn address
		(try! (penalize-helper current-pool))

		;; Change peg-state to "bad-peg" :(
		(contract-call? .sbtc-registry set-peg-state false)

	)
)

;; Penalty function for when a new wallet vote threshold (70%) is not met in time
(define-public (penalty-vote-threshold)
	(let
		(
			(current-cycle (current-pox-reward-cycle))
			(next-cycle (+ current-cycle u1))
			(current-pool (unwrap! (map-get? stacking-details-by-cycle current-cycle) err-pool-cycle))
			(current-pool-stackers (get stackers current-pool))
			(next-pool (unwrap! (map-get? stacking-details-by-cycle next-cycle) err-pool-cycle))
			(next-pool-threshold-wallet (get threshold-wallet next-pool))
		)

		;; Assert that we're in the transfer window
		(asserts! (is-eq (get-current-window) transfer) err-not-in-transfer-window)

		;; Assert that next-pool-threshold-wallet is-none
		(asserts! (is-some next-pool-threshold-wallet) err-unhandled-request)

		;; Assert that pending-wallet-withdrawals is equal to zero
		(asserts! (is-eq (contract-call? .sbtc-registry get-pending-wallet-withdrawals) u0) err-unhandled-request)

		;; Penalize stackers by re-stacking but with a pox-reward address of burn address
		(try! (penalize-helper current-pool))

		;; Change peg-state to "bad-peg"
		(contract-call? .sbtc-registry set-peg-state false)

	)
)

;; Penalty function for when a stacker fails to transfer the current peg balance to the next threshold wallet
(define-public (penalty-balance-transfer)
	(let
		(
			(current-cycle (current-pox-reward-cycle))
			(current-pool (unwrap! (map-get? stacking-details-by-cycle current-cycle) err-pool-cycle))
			(current-pool-balance-transfer (get balance-transferred current-pool))
			(current-pool-stackers (get stackers current-pool))
			(next-cycle (+ current-cycle u1))
			(next-pool (unwrap! (map-get? stacking-details-by-cycle next-cycle) err-pool-cycle))
			(next-pool-threshold-wallet (get threshold-wallet next-pool))
		)

		;; Assert that we're in the penalty window
		(asserts! (is-eq (get-current-window) penalty) err-not-in-penalty-window)

		;; Assert that next-pool-threshold-wallet is-some / was voted in correctly
		(asserts! (is-some next-pool-threshold-wallet) err-unhandled-request)

		;; Assert that balance-transfer is false (wasn't already transferred)
		(asserts! (not current-pool-balance-transfer) err-unhandled-request)

		;; Penalize stackers by re-stacking but with a pox-reward address of burn address
		(try! (penalize-helper current-pool))

		;; Change peg-state to "bad-peg"
		(contract-call? .sbtc-registry set-peg-state false)

	)
)

;; Penalty function for when the pox-rewards aren't disbursed in time / registration is missing
(define-public (penalty-pox-reward-disbursement)
	(let
		(
			(current-cycle (current-pox-reward-cycle))
			(current-pool (unwrap! (map-get? stacking-details-by-cycle current-cycle) err-pool-cycle))
			(previous-cycle (- current-cycle u1))
			(previous-pool (unwrap! (map-get? stacking-details-by-cycle previous-cycle) err-pool-cycle))
			(previous-pool-rewards-disbursed (get rewards-disbursed previous-pool))
			(next-cycle (+ current-cycle u1))
			(next-pool (unwrap! (map-get? stacking-details-by-cycle next-cycle) err-pool-cycle))
		)

		;; Assert that we're in the voting window (aka registration window was missed)
		(asserts! (is-eq (get-current-window) voting) err-not-in-voting-window)

		;; Assert that last pool disbursed is false (wasn't already disbursed)
		(asserts! (not previous-pool-rewards-disbursed) err-rewards-already-disbursed)

		;; To review/ask about
		;; Penalize stackers by re-stacking but with a pox-reward address of burn address
		;; Distributing pox-rewards punishes current signers even if they did nothing wrong?
		;; Meanwhile previous signers aren't punished since the pox-rewards are already at the previous threshold wallet
		;; (match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-burn-address next-cycle))
		;;     ok-result
		;;         (map-set stacking-details-by-cycle next-cycle (merge
		;;             next-pool
		;;             {last-aggregation: (some block-height), reward-index: (some ok-result)}
		;;         ))
		;;     err-result
		;;         false
		;; )
		(try! (penalize-helper current-pool))

		;; Change peg-state to "bad-peg"
		(contract-call? .sbtc-registry set-peg-state false)

	)
)

;; Penalize helper
(define-private (penalize-helper (penalized-pool
		{
			stackers: (list 100 principal),
			stacked: uint,
			threshold-wallet-candidates: (list 100 { version: (buff 1), hashbytes: (buff 32) }),
			threshold-wallet: (optional { version: (buff 1), hashbytes: (buff 32) }),
			last-aggregation: (optional uint),
			reward-index: (optional uint),
			balance-transferred: bool,
			rewards-disbursed: bool
		}
	))
	(as-contract (fold mass-delegate-stack-extend (get stackers penalized-pool) (ok {stacker: tx-sender, unlock-burn-height: u0, pox-addr: pox-burn-address})))
)

;; mass delegate stack extender helper for either penalizing or concluded vote
(define-private (mass-delegate-stack-extend (stacker principal) (pox-return (response {stacker: principal, unlock-burn-height: uint, pox-addr: { version: (buff 1), hashbytes: (buff 32) }} uint)))
	(let
		(
			(unwrap-response (try! pox-return))
			(param-pox-addr (get pox-addr unwrap-response))
			(extend-delegate-response (unwrap! (contract-call? .pox-3 delegate-stack-extend stacker param-pox-addr u1) err-mass-delegate-stack-extend))
			(extend-delegate-stacker (get stacker extend-delegate-response))
			(extend-delegate-unlock-burn-height (get unlock-burn-height extend-delegate-response))
		)

		;; Assert previous pox-return was (ok ...)
		(unwrap! pox-return err-mass-delegate-stack-extend)

		(ok {stacker: extend-delegate-stacker, unlock-burn-height: extend-delegate-unlock-burn-height, pox-addr: param-pox-addr})
	)
)


;;; Protocol Functions ;;;;
;; Protocol function for updating threshold-percent
(define-public (update-threshold-percent (new-threshold-percent uint))
	(begin

		;; Assert that caller is protocol caller
		(unwrap! (is-protocol-caller) err-not-protocol-caller)

		;; Assert that new-threshold-percent is greater u500 or less than u950
		(asserts! (and (>= new-threshold-percent u500) (<= new-threshold-percent u950)) err-out-of-range)

		;; Update threshold-percent
		(ok (var-set threshold-consensus new-threshold-percent))
	)
)

(define-public (update-minimum-pool-amount-for-activation (new-minimum uint))
	(begin

		;; Assert that caller is protocol caller
		(unwrap! (is-protocol-caller) err-not-protocol-caller)

		;; Assert that new-minimum is greater than the static minimum for stacking
		(asserts! (and (>= new-minimum (/ (get total-liquid-supply-ustx pox-info) stacking-threshold-25))) err-out-of-range)

		;; Update minimum amount
		(ok (var-set minimal-pool-amount-for-activation new-minimum))
	)
)


;;
;; Functions about allowance of delegation/stacking contract calls
;;

;; Give a contract-caller authorization to call stacking methods
;;  normally, stacking methods may only be invoked by _direct_ transactions
;;   (i.e., the tx-sender issues a direct contract-call to the stacking methods)
;;  by issuing an allowance, the tx-sender may call through the allowed contract
(define-public (allow-contract-caller (caller principal) (until-burn-ht (optional uint)))
	(begin
		(asserts! (is-eq tx-sender contract-caller) err-stacking-permission-denied)
		(ok (map-set allowance-contract-callers
			{ sender: tx-sender, contract-caller: caller}
			{ until-burn-ht: until-burn-ht}))))

;; Revokes contract-caller authorization to call stacking methods
(define-public (disallow-contract-caller (caller principal))
	(begin
		(asserts! (is-eq tx-sender contract-caller) err-stacking-permission-denied)
		(ok (map-delete allowance-contract-callers { sender: tx-sender, contract-caller: caller}))))

;; Verifies that the contract caller has allowance to handle the tx-sender's stacking
(define-read-only (check-caller-allowed)
	(or (is-eq tx-sender contract-caller)
		(let ((caller-allowed
			;; if not in the caller map, return false
			(unwrap! (map-get? allowance-contract-callers
					{ sender: tx-sender, contract-caller: contract-caller})
			  	false))
		  (expires-at
			;; if until-burn-ht not set, then return true (because no expiry)
			(unwrap! (get until-burn-ht caller-allowed) true)))
		  ;; is the caller allowance still valid
	  (< burn-block-height expires-at))))

;; Returns the burn height at which a particular contract is allowed to stack for a particular principal.
;; The result is (some (some X)) if X is the burn height at which the allowance terminates.
;; The result is (some none) if the caller is allowed indefinitely.
;; The result is none if there is no allowance record.
(define-read-only (get-allowance-contract-callers (sender principal) (calling-contract principal))
	(map-get? allowance-contract-callers { sender: sender, contract-caller: calling-contract}))
