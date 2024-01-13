;; exhchange utilities
(use-trait ft-trait .trait-sip-010.sip-010-trait)

(define-map approved-tokens {token: principal} {approved: bool})

(define-constant err-forbidden (err u100))
(define-constant err-token-not-approved (err u999))

(define-constant contract-admin tx-sender)
(define-constant ONE_8 u100000000)

(map-set approved-tokens {token: .token-wbtc} {approved: true})
(map-set approved-tokens {token: .token-wstx} {approved: true})

(define-private (to-one-8 (a uint))
  (* a ONE_8))

(define-private (mul-down (a uint) (b uint))
  (/ (* a b) ONE_8))

(define-private (div-down (a uint) (b uint))
  (if (is-eq a u0)
    u0
    (/ (* a ONE_8) b)))

(define-private (minus-percent (a uint) (percent uint))
  (if (is-eq a u0)
    u0
    (/ (- (* a u100) (* a percent)) u100)))

;; public to check return btc-to-stx amount
;; input: 'SP3K8BC0PPEVCV7NZ6QSRWPQ2JE9E5B6N3PA0KBR9.token-wbtc 'SP3K8BC0PPEVCV7NZ6QSRWPQ2JE9E5B6N3PA0KBR9.token-wstx uN (xBTC or STX) * 10^8  u5 (5%)
(define-public (swap-helper (token-x-trait <ft-trait>) (token-y-trait <ft-trait>) (multiplied-amount uint) (slippeage uint)) 
  (let (
    (token-x (contract-of token-x-trait))
    (token-y (contract-of token-y-trait))
    (fee-amount 
      (contract-call? .amm-swap-pool-v1-1 fee-helper token-x token-y ONE_8))
    (get-helper-result (try! (contract-call? .amm-swap-pool-v1-1 get-helper token-x token-y ONE_8 multiplied-amount)))
    (converted-amount 
      (mul-down 
        get-helper-result 
        (- ONE_8 (unwrap-panic fee-amount))))
    (converted-amount-slippeage (minus-percent converted-amount slippeage)))
    (asserts! 
    (and 
      (get-approved-token token-x)
      (get-approved-token token-y))
    err-token-not-approved)
    (ok (contract-call? 
        .amm-swap-pool-v1-1 swap-helper
          token-x-trait
          token-y-trait
          ONE_8
          multiplied-amount
        (some converted-amount-slippeage)))))

(define-public (swap-bridge-stx-btc 
                  (token-x-trait <ft-trait>) 
                  (token-y-trait <ft-trait>) 
                  (multiplied-amount uint) 
                  (slippeage uint) 
                  (btc-version (buff 1)) 
                  (btc-hash (buff 20)) 
                  (supplier-id uint)) 
  (let (
    (token-x (contract-of token-x-trait))
    (token-y (contract-of token-y-trait))
    (fee-amount 
      (contract-call? .amm-swap-pool-v1-1 fee-helper token-x token-y ONE_8))
    (get-helper-result (try! (contract-call? .amm-swap-pool-v1-1 get-helper token-x token-y ONE_8 multiplied-amount)))
    (xbtc-amount 
      (mul-down 
        get-helper-result 
        (- ONE_8 (unwrap-panic fee-amount))))
    (xbtc-amount-slippeage (minus-percent xbtc-amount slippeage))
    (swap-result 
      (try! (contract-call? 
        .amm-swap-pool-v1-1 swap-helper
          token-x-trait
          token-y-trait
          ONE_8
          multiplied-amount
        (some xbtc-amount-slippeage)))))
    (try! 
      (contract-call? .degen-bridge-testnet-v3 initiate-outbound-swap 
                          swap-result 
                          btc-version 
                          btc-hash 
                          supplier-id))
    (ok swap-result)))

(define-public (swap-preview (token-x-trait <ft-trait>) (token-y-trait <ft-trait>) (multiplied-amount uint) (slippeage uint)) 
  (let (
    (token-x (contract-of token-x-trait))
    (token-y (contract-of token-y-trait))
    (fee-amount 
      (contract-call? .amm-swap-pool-v1-1 fee-helper token-x token-y ONE_8))
    (get-helper-result (try! (contract-call? .amm-swap-pool-v1-1 get-helper token-x token-y ONE_8 multiplied-amount)))
    (converted-amount 
      (mul-down 
        get-helper-result 
        (- ONE_8 (unwrap-panic fee-amount))))
    (converted-amount-slippeage (minus-percent converted-amount slippeage)))
      (ok converted-amount)))

(define-public (set-approved-token (token principal))
(begin 
  (asserts! (is-eq contract-caller contract-admin) err-forbidden)
  (ok (map-set approved-tokens {token: token} {approved: true}))))

(define-read-only (get-approved-token (token principal))
(begin
  (default-to false 
    (get approved 
      (map-get? approved-tokens {token: token})))))