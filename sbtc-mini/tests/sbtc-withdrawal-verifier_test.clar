(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-2 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG)
(define-constant test-mint-amount    u10000000)
(define-constant test-allowance      u50000)
(define-constant test-withdrawal-amount u6000000)
(define-constant test-total-supply (* u3 test-mint-amount))

(define-constant contract-principal (as-contract tx-sender))

(define-constant version-P2TR 0x06)

;;(define-constant burnchain-confirmations-required (unwrap-panic (contract-call? .sbtc-registry get-burnchain-confirmations-required)))

;; bcrt1p38e4lrh823h8w79lrgflz3etk63hcvtyl5a4l4u0n6l0cgcp8pxqdw342z
(define-constant test-recipient-destination { version: version-P2TR, hashbytes: 0x89f35f8ee7546e7778bf1a13f1472bb6a37c3164fd3b5fd78f9ebefc2301384c })
(define-constant test-withdrawal-transaction 0x02000000000101c0ae643bde33b865dec47beeff21ddc92a666d37135c6b6522a4cd9f66ae95780000000000fdffffff02808d5b000000000022512089f35f8ee7546e7778bf1a13f1472bb6a37c3164fd3b5fd78f9ebefc2301384cdb63aa29010000002251200578716337781bf4563d4aacec67ee8d1875e33fced744da937b1bab5e52f8fe0247304402207085dbf16b21bc9eb28aefc52e3a7f0fd3b3782813b33d3d5f8325929037db2a02200f1b2bdc520df530c7484ba8d5d1ee00808d309b54f1cf9ee62d7bc888edc56f012103caf9241a5f9e930cf9711b3fa8fba25d2e77861ff7f15a4074a8aec86787617e00000000)
(define-constant test-withdrawal-txid 0x762e0f47da9603da7b7597a736b2e692e7651406fe04d79d120c50fcbed413f5) ;; le: 0xf513d4befc500c129dd704fe061465e792e6b236a797757bda0396da470f2e76
(define-constant test-withdrawal-txid-le 0xf513d4befc500c129dd704fe061465e792e6b236a797757bda0396da470f2e76)
(define-constant test-withdrawal-wtxid 0x4afc585692d209a3d098fb04565d8b03ccd88e7f448c8091294fea71b989d5ee) ;; le: 0xeed589b971ea4f2991808c447f8ed8cc038b5d5604fb98d0a309d2925658fc4a
(define-constant test-withdrawal-request-id u0)

(define-constant mock-burnchain-height u1)

(define-constant test-header 0x0200000000000000000000000000000000000000000000000000000000000000000000004507b4e69d599ff2d995bbea9367dfb339ecb413bc061d2bd1106212310a9721000000000000000000000000)
(define-constant test-header-hash 0x58ac2cbe02f5070ecea05ba07b78392c639b729d1692ef70dfa4c52f77d1bfa7) ;; le 0xa7bfd1772fc5a4df70ef92169d729b632c39787ba05ba0ce0e07f502be2cac58
(define-constant test-ctx 0x020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff03016500ffffffff0200f2052a010000002251205444612a122cd09b4b3457d46c149a23d8685fb7d3aac61ea7eee8449555293b0000000000000000266a24aa21a9ed56c03b43ba68b8c5d0d99890009071f8b30133b14c167a66335a0beff1783a2f0120000000000000000000000000000000000000000000000000000000000000000000000000)
(define-constant test-coinbase-wtxid 0x0000000000000000000000000000000000000000000000000000000000000000)
(define-constant test-witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
(define-constant test-witness-merkle-root 0x1b0b6ecb3ecef1c3ff149851ade49318fa29a4a4af4e0555a41a88974590da1f) ;; be: 0x1fda904597881aa455054eafa4a429fa1893e4ad519814ffc3f1ce3ecb6e0b1b
(define-constant test-cproof 0x9494986defd6a27e09ab5e3450c42741e203e454d4025580212bfbfcdab94e27)

(define-public (prepare-add-test-to-protocol)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

;; Prepare function called for all tests (unless overridden)
(define-public (prepare)
	(begin
		;; Add the test contract to the protocol contract set.
		(try! (prepare-add-test-to-protocol))
		;; Add the contracts to the protocol contract set.
		(try! (contract-call? .sbtc-testnet-debug-controller set-protocol-contract .sbtc-withdrawal-request-stx true))
		(try! (contract-call? .sbtc-testnet-debug-controller set-protocol-contract .sbtc-withdrawal-verifier true))
		;; Mint some tokens to test principal.
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount contract-principal))
		;; Simulate the Bitcoin block.
		(unwrap! (contract-call? .clarity-bitcoin mock-add-burnchain-block-header-hash mock-burnchain-height test-header-hash) (err u999))
		;; Insert withdrawal request
		(try! (contract-call? .sbtc-withdrawal-request-stx request-withdrawal test-withdrawal-amount contract-principal test-recipient-destination))
		(ok true)
	)
)

;; @name can fulfil a withdrawal
;; @mine-blocks-before 4
(define-public (test-request-withdrawal-fulfilment)
	(let (
		(result (contract-call? .sbtc-withdrawal-verifier relay-withdrawal-fulfilment
			test-withdrawal-request-id
			mock-burnchain-height ;; burn-height
			test-withdrawal-transaction
			test-header ;; header
			u1 ;; tx-index
			u1 ;; tree-depth
			(list test-coinbase-wtxid) ;; wproof
			test-witness-merkle-root ;; witness-merkle-root
			test-witness-reserved-data ;; witness-reserved-data
			test-ctx ;; ctx
			(list test-withdrawal-txid-le) ;; cproof
			))
		(balance-total (unwrap-panic (contract-call? .sbtc-token get-balance contract-principal)))
		(balance-locked (unwrap-panic (contract-call? .sbtc-token get-balance-locked contract-principal)))
		(balance-available (unwrap-panic (contract-call? .sbtc-token get-balance-available contract-principal)))
		)
		(unwrap! result (err {msg: "Relay failed", actual: (some result)}))
		(asserts! (is-eq balance-total (- test-mint-amount test-withdrawal-amount)) (err {msg: "sBTC balance is not equal to test-mint-amount minus test-withdrawal-amount", actual: none}))
		(asserts! (is-eq balance-locked u0) (err {msg: "locked sBTC balance is not zero", actual: none}))
		(asserts! (is-eq balance-available balance-total) (err {msg: "available sBTC balance is not equal to total balance", actual: none}))
		(ok true)
	)
)