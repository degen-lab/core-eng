(define-constant test-contract-principal (as-contract tx-sender))
(define-constant zero-address 'SP000000000000000000002Q6VF78)

(define-public (add-burnchain-block-header-hash (burn-height uint) (header (buff 80)))
  (contract-call? .clarity-bitcoin mock-add-burnchain-block-header-hash burn-height (contract-call? .clarity-bitcoin reverse-buff32 (sha256 (sha256 header))))
)


(define-public (prepare)
	(begin
    ;; 1
    (unwrap-panic (add-burnchain-block-header-hash u2431087 0x0000a02065bc9201b5b5a1d695a18e4d5efe5d52d8ccc4129a2499141d000000000000009160ba7ae5f29f9632dc0cd89f466ee64e2dddfde737a40808ddc147cd82406f18b8486488a127199842cec7))
    ;; 2
    (unwrap-panic (add-burnchain-block-header-hash u2431459 0x000000207277c3a739ad34a3873a3c9d755f0766e918f4b37adcfe455d079b93000000000bad83579d633d37aa1e2bb725b8a5bad35509374454f1adad8a4de140066d74dbf64e64ffff001d6616bfe2))
    ;; 3
    (unwrap-panic (add-burnchain-block-header-hash u2431567 0x000000208af1a70ab2ed062c7ac53eb56b053498db50f0d9c41f0dc8a5efcb1b000000007b64b9e16eb97b1fb32977aa00e2cb7418856b1e794e232be4f3b4b0512cb31256845064ffff001dc3cdbab0))
    ;; 4
    (unwrap-panic (add-burnchain-block-header-hash u2431619 0x00c0c8306c887f5b2248021d1a6662aa684ca46975a21685800192b3f4e5c8b400000000a140532ff49146607e72df19983ff1e4a56f989cf0671aeb4ee321a74d3670c75c545164695a20192b1ab0c7))
    ;; 5
    (unwrap-panic (add-burnchain-block-header-hash u2430921 0x00004020070e3e8245969a60d47d780670d9e05dbbd860927341dda51d000000000000007ecc2f605412dddfe6e5c7798ec114004e6eda96f7045baf653c26ded334cfe27766466488a127199541c0a6))
    ;; 6
    (unwrap-panic (add-burnchain-block-header-hash u2431087 0x0000a02065bc9201b5b5a1d695a18e4d5efe5d52d8ccc4129a2499141d000000000000009160ba7ae5f29f9632dc0cd89f466ee64e2dddfde737a40808ddc147cd82406f18b8486488a127199842cec7))
		(ok true)
	)
)

;; @name parse raw coinbase transaction
;; @no-prepare
;; arbitrary coinbase transaction
(define-public (test-parse-raw-coinbase-transaction)
  (let (
    ;; block id: 000000000000000606f86a5bc8fb6e38b16050fb4676dea26cba5222583c4d86
      (raw-coinbase-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000)
      (parsed-coinbase-tx (contract-call? .clarity-bitcoin parse-tx raw-coinbase-tx))
      (ins (list { outpoint: { hash: 0x0000000000000000000000000000000000000000000000000000000000000000, index: u4294967295},
        scriptSig: 0x036f18250418b848644d65726d61696465722046545721010000686d20000000000000, sequence: u4294967295}))
      (outs (list {scriptPubKey: 0x0014c035e789d9efffa10aa92e93f48f29b8cfb224c2, value: u2490093}
        {scriptPubKey: 0x6a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c, value: u0}))
      (expected {version: u2, ins: ins, outs: outs, locktime: u0})
    )
    (ok (asserts! (is-eq parsed-coinbase-tx (ok expected)) (err parsed-coinbase-tx)))
  )
)

;; @name verify segwit transaction where OP_RETURN is in output[1]
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-1)
  (let (
    (burnchain-block-height u2431087)
    ;; txid: 3b3a7a31c949048fabf759e670a55ffd5b9472a12e748b684db5d264b6852084
    (raw-tx 0x0200000000010218f905443202116524547142bd55b69335dfc4e4c66ff3afaaaab6267b557c4b030000000000000000e0dbdf1039321ab7a2626ca5458e766c6107690b1a1923e075c4f691cc4928ac0000000000000000000220a10700000000002200208730dbfaa29c49f00312812aa12a62335113909711deb8da5ecedd14688188363c5f26010000000022512036f4ff452cb82e505436e73d0a8b630041b71e037e5997290ba1fe0ae7f4d8d50140a50417be5a056f63e052294cb20643f83038d5cd90e2f90c1ad3f80180026cb99d78cd4480fadbbc5b9cad5fb2248828fb21549e7cb3f7dbd7aefd2d541bd34f0140acde555b7689eae41d5ccf872bb32a270893bdaa1defc828b76c282f6c87fc387d7d4343c5f7288cfd9aa5da0765c7740ca97e44a0205a1abafa279b530d5fe36d182500)
    ;; txid: f1bb118d6663640d6571d08ee36ffff78130c0a6a959e69929f952e745e54050
    (raw-coinbase-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000)
    ;; block id: 000000000000000606f86a5bc8fb6e38b16050fb4676dea26cba5222583c4d86
    (raw-block-header 0x0000a02065bc9201b5b5a1d695a18e4d5efe5d52d8ccc4129a2499141d000000000000009160ba7ae5f29f9632dc0cd89f466ee64e2dddfde737a40808ddc147cd82406f18b8486488a127199842cec7)
    (witness-merkle-root 0x15424423c2614c23aceec8d732b5330c21ff3a306f52243fbeef47a192c65c86)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )

    (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u3
      u2
      (list 0xb2d7ec769ce60ebc0c8fb9cc37f0ad7481690fc176b82c8d17d3c05da80fea6b 0x122f3217765b6e8f3163f6725d4aa3d303e4ffe4b99a5e85fb4ff91a026c17a8)
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0x5f4a8858a112953111d5f94605c4dd8d04690eeeb9ffe2f435475d95943f2f3f 0x3348bf81aae79941662a902206b3ed2d285713668ab134c9e113548daea596fc)
    )
  )
)

;; @name verify segwit transaction where OP_RETURN is in output[0]
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-2)
  (let (
    (burnchain-block-height u2431459)
    ;; txid: 0edafd2b2e9374ede142d1b60f884b47aa7a4ae0a5d46aa9c1d63d2e8e27087d
    (raw-tx 0x02000000000101cbab643067979ec7a7c74bf49af775b20203df78e31b83b2d25510fe5897872c0000000000feffffff02b0631d0000000000160014f55fa4fafee59cccde7d6204029d388480d354b6ce32599300000000160014cc9957eaabf54d9fa7d1c0ae64bd0f95e0bd0cdf0247304402200f17b4dfafedf598db8c48551efc8cce14de071ef86a0c7494d593e7346856c702207f96c3382ed4ed855bc86ac5a6df2e283c1b06276176cae590abdcd6f7c491bf012102276dc68fca4dae0eedbd5b9d7b36baccaff56b777d103d169ec31ce25f04fd14e2192500)
    ;; block id: 000000004daafb5977a88c7111da740e77cf6dd5a417f343ed01374676266c36
    (raw-block-header 0x000000207277c3a739ad34a3873a3c9d755f0766e918f4b37adcfe455d079b93000000000bad83579d633d37aa1e2bb725b8a5bad35509374454f1adad8a4de140066d74dbf64e64ffff001d6616bfe2)
    (witness-merkle-root 0x9a0f41eac348c25620b76aa15f771a7972638e8423ce590cdbe1432c4f5726c0)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    ;; txid: 28f35a9c67a275e1cf8678c3f030f005b4c85ff8fcbf1742035ec39ebbdb5164
    (raw-coinbase-tx 0x01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff1603e319251300000062696e67626f6e67210001000000ffffffff02bbb329000000000017a9148746e880ec2156d6b527e1962456cff7720695df870000000000000000266a24aa21a9edcbc79776b99d0cc0c2d597ce40c92958ba98f3adb6e59ed27c3a03d1c1703e1c00000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )
    (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u8
      u7
      (list 0xbcbc1fe72ca5d67f74099ac4f851cd6a266d2a42fa6c9a6244b11adf6a2f13fb 0xec7eee1209489f80c4a5df2a90f26ee0ba0838be6f00442e3e0ab9d095865b45 0x0ddfb056b3e566ac3b376a940b9eff66a77296ec200f427b158f1700b2a32398 0x922b21d35cda111feeb04cd69d2fa78312fd34614d34c4fbfd204730b6ef013f 0xcf4c5ecd88374f44ae63ba87f96ef20e0330a247434c219ce8f8fdbc7ace4401 0x04889c3c93e70474fe0724f5270e26d91c974fa7b642703286903350480e2281 0x762dd8d1161d1d32a79c98f6273b6feb50405446e861db0f32c7403a8bb310b2)
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0x012e8988aa58e53ac700c22257237716f642aea5ad45c3f6420eb010bf2b37f9 0x10b71df80cc37bd92c746690a980a7f4187e6dbf8b973ac938f1b1b9d1ef25b6 0x897a09fa66156b753366b38236fd40fe7920db114722b3d7762826bf4930750c 0x2cf51c5e81f70b72c37fd8eccd8d3446c7c69c4d568acd23c9949c25bc1d3fb4 0x7d45568289180def6b91cdf6f89c692f3798872948b582a9f6fd5cc471cb67e0 0xe5fefbaf926e706c4ef331d172dcb589dfd6b997bee4ba6ddee4c7c6e2918549 0x097ae87e24dae74bb00a2bfe0aa3c14537ce04b048285ce69d98301bd9ddee8d)
    )
  )
)


;; @name verify segwit transaction where OP_RETURN is in output[0]
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-3)
  (let (
    (burnchain-block-height u2431567)
    ;; txid: 2fd0308a0bca2f4ea40fe93a19be976a40b2d5e0df08df1dd991b4df31a563fc
    (raw-tx 0x020000000001017d406bb6466e0da97778f55ece77ab6becc415c930dbe618e81e8dae53ba914400000000171600143e8d4581104393d916566886ae01e26be8f8975afeffffff0276410300000000001600143d5f37543d2916547fe9b1242322b22f6e46d6929c9291d8000000001600145b3298860caeb3302f09e58b7cc3cf58d0a6740402463043021f53943d5951726d73a952473b49aa44dde64446f106749806d6478b0bdc053102200863a4f25914f3032afa8549f329962e8ecc7c575a1aefc51102a566ff5ece70012103742fe8a7244ab8384b3d533eb19138e2758c2b7a2e351c9ec2b8912cf4e046c24e1a2500)
    ;; block id: 00000000096ae97d41a543592c3680477444acdc86c877aeb4832744691cb94b
    (raw-block-header 0x000000208af1a70ab2ed062c7ac53eb56b053498db50f0d9c41f0dc8a5efcb1b000000007b64b9e16eb97b1fb32977aa00e2cb7418856b1e794e232be4f3b4b0512cb31256845064ffff001dc3cdbab0)
    (witness-merkle-root 0xb2ea7fb39beae6b5a225c85cb7087561909e1d17bba74de3592a3c1da3944983)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    ;; txid: 84f8a86015b0a95763bb16128ff301a60f668356548405178953ab1e4cbff36e
    (raw-coinbase-tx 0x01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff32034f1a2500045684506404c862b40c0c11c14a6400000000000000000a636b706f6f6c0e2f6d696e65642062792072736b2fffffffff0317ea2b00000000001976a914ec2f9ffaba0d68ea6bd7c25cedfe2ae710938e6088ac0000000000000000266a24aa21a9ede2ee16ef0a8c0fd6ccb8c78f297199f0f143629121801c46b1e1487c0123cb4b00000000000000002a6a52534b424c4f434b3acb9b4841f625100fb87055003991835f3183bff7668865e93e1f1118003a492d00000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )

    (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u1
      u7
      (list 0x0000000000000000000000000000000000000000000000000000000000000000 0x71f824a016ae0f58a01cb8a4aafb910c6013666640f43b745adc87dcc9118a66 0x3efcbcef748b6b46ce25fc94d455555851322f9bcbf36de02500492dacc8fcfa 0xbca0973353bed064af7388d48969f23b0dc65d8cb1c8eaae95890b7c08a39914 0x84211c0f3a64e0fab33d1ab9d537a60adab81bcb681f1297901aad5566683e56 0x8527cd4ff222ff5010726219744a19522e2d016873d4a8ca3bfa984f32cf7355 0x83fbd3b472c7e7285ad25fd40edaf658acef8756aecf32a115d98fd07d006af9)
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0xfc63a531dfb491d91ddf08dfe0d5b2406a97be193ae90fa44e2fca0b8a30d02f 0x33274cc92f8b980272688e01114cc2944fb661d1aa3a658c7d29675a46a4d5ad 0x1172bf0943aad7bc580aaab5f5d356b1c172f11c297ccf0077515309906352f2 0x2af4fd00ac79b65c0c508fbf44c22d1cf5acb084770079a94bd72ac816cfceb8 0xed4ca325a1800f0dfb2ab9d63761ecb358c014424e684c820d0d87ace45474a1 0xd3292e0e550420e500f29663dfc8ef632dbcb119c8a1ddf49aa3d32ecad83084 0x6369b65eea600edbd69b56386be9269f9662ca3f384a0ca21922ac03d2936102)
    )
  )
)

;; @name verify segwit transaction where the incorrect proof is provided and only the first transaction is given. should fail
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-4)
  (let (
    (burnchain-block-height u2431619)
    ;; txid: c770364da721e34eeb1a67f09c986fa5e4f13f9819df727e604691f42f5340a1
    (raw-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff2503831a250000000000000000000000000000000002000000000000073c7500000000000000ffffffff02be402500000000001976a91455a2e914aeb9729b4cd265248cb67a865eae95fd88ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf900000000)
    ;; block id: 
    (raw-block-header 0x00c0c8306c887f5b2248021d1a6662aa684ca46975a21685800192b3f4e5c8b400000000a140532ff49146607e72df19983ff1e4a56f989cf0671aeb4ee321a74d3670c75c545164695a20192b1ab0c7)
    (witness-merkle-root 0x0000000000000000000000000000000000000000000000000000000000000000)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    ;; txid: c770364da721e34eeb1a67f09c986fa5e4f13f9819df727e604691f42f5340a1
    (raw-coinbase-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff2503831a250000000000000000000000000000000002000000000000073c7500000000000000ffffffff02be402500000000001976a91455a2e914aeb9729b4cd265248cb67a865eae95fd88ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf900000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )

    (match (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u0
      u1
      (list )
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0x12b32c51b0b4f3e42b234e791e6b851874cbe200aa7729b31f7bb96ee1b9647b)
    )
      ok-res (err u1)
      err-res (if (is-eq err-res ERR-PROOF-TOO-SHORT) (ok err-res) (err err-res))
    )
  )
)

;; @name OP_RETURN is too large. Fails to parse segwit transaction
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-5)
  (let (
    (burnchain-block-height u2430921)
    ;; txid: f95ece8dde2672891df03a79ed6099c0de4ebfdaee3c31145fe497946368cbb0
    (raw-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff2503831a250000000000000000000000000000000002000000000000073c7500000000000000ffffffff02be402500000000001976a91455a2e914aeb9729b4cd265248cb67a865eae95fd88ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf900000000)
    ;; block id: 000000000000000011958fb8368d946487bde267f7cb78256ede44ec14e38d87
    (raw-block-header 0x00004020070e3e8245969a60d47d780670d9e05dbbd860927341dda51d000000000000007ecc2f605412dddfe6e5c7798ec114004e6eda96f7045baf653c26ded334cfe27766466488a127199541c0a6)
    (witness-merkle-root 0x366c4677e2f40e524b773344401f1de980ec9a9cb3453620cd1ff652b9c1d53e)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    ;; txid: e2798f96e3584be98f0b6eb6833dbf8284f95c4bb183f425bf409eb3ecc4bf6b
    (raw-coinbase-tx 0x01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff1b03c917250477664664004000009be40b0000084d617261636f726500000000030000000000000000266a24aa21a9edb675067096401108d0bd081d693fb4adae204ea41a2c444ad79826a2d9f0bb250000000000000000fd80017b226964223a6e756c6c2c22726573756c74223a7b2268617368223a2239346562366639633664633130396239646630396333306237616561613065383538656165626536343831346138363036383962383065633931313634383864222c22636861696e6964223a312c2270726576696f7573626c6f636b68617368223a2232663566306161663139633139623138313433333839663864643330353234323539323937393137613562396330613864353432366362323233373731336364222c22636f696e6261736576616c7565223a3632353030303030302c2262697473223a223230376666666666222c22686569676874223a3437382c225f746172676574223a2230303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030666666663766222c226d65726b6c655f73697a65223a312c226d65726b6c655f6e6f6e6365223a323539363939363136327d2c226572726f72223a6e756c6c7db87a2500000000001976a914bb94b2b8ce1719201bf0346d79ee0f60c9d2700088ac00000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )

    (match (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u2
      u2
      (list 0x060f653adf158c9765994dcc38c2d29c4722b4415e56468aae2908cc26d5b7fc 0x438e9befc51be8d8570386ce9b5050e75ddcd410c92d0e7693b11c82b4c73f2f)
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0x3d52480061d7634fa8060430cf86d8de3f577499a2056f5ff80cc36918a78dcc 0x41dd33b4cffe074cc8263f98da7c81006521eb2cc8712028fa491efed619cffb)
    )
      ok-res (err u1)
      err-res (if (is-eq err-res ERR-VARSLICE-TOO-LONG) (ok err-res) (err err-res))
    )
  )
)

;; @name verify segwit transaction where OP_RETURN is in output[1]
;; arbitrary segwit transaction
(define-public (test-was-wtx-mined-internal-6)
  (let (
    (burnchain-block-height u2431087)
    ;; txid: 3b3a7a31c949048fabf759e670a55ffd5b9472a12e748b684db5d264b6852084
    (raw-tx 0x0200000000010218f905443202116524547142bd55b69335dfc4e4c66ff3afaaaab6267b557c4b030000000000000000e0dbdf1039321ab7a2626ca5458e766c6107690b1a1923e075c4f691cc4928ac0000000000000000000220a10700000000002200208730dbfaa29c49f00312812aa12a62335113909711deb8da5ecedd14688188363c5f26010000000022512036f4ff452cb82e505436e73d0a8b630041b71e037e5997290ba1fe0ae7f4d8d50140a50417be5a056f63e052294cb20643f83038d5cd90e2f90c1ad3f80180026cb99d78cd4480fadbbc5b9cad5fb2248828fb21549e7cb3f7dbd7aefd2d541bd34f0140acde555b7689eae41d5ccf872bb32a270893bdaa1defc828b76c282f6c87fc387d7d4343c5f7288cfd9aa5da0765c7740ca97e44a0205a1abafa279b530d5fe36d182500)
    ;; txid: f1bb118d6663640d6571d08ee36ffff78130c0a6a959e69929f952e745e54050
    (raw-coinbase-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000)
    ;; block id: 000000000000000606f86a5bc8fb6e38b16050fb4676dea26cba5222583c4d86
    (raw-block-header 0x0000a02065bc9201b5b5a1d695a18e4d5efe5d52d8ccc4129a2499141d000000000000009160ba7ae5f29f9632dc0cd89f466ee64e2dddfde737a40808ddc147cd82406f18b8486488a127199842cec7)
    (witness-merkle-root 0x15424423c2614c23aceec8d732b5330c21ff3a306f52243fbeef47a192c65c86)
    (witness-reserved-data 0x0000000000000000000000000000000000000000000000000000000000000000)
    (parsed-block-header (contract-call? .clarity-bitcoin parse-block-header raw-block-header))
    (parsed-tx (contract-call? .clarity-bitcoin parse-wtx raw-tx))
  )
    (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
      burnchain-block-height
      raw-tx
      raw-block-header
      u3
      u2
      (list 0xb2d7ec769ce60ebc0c8fb9cc37f0ad7481690fc176b82c8d17d3c05da80fea6b 0x122f3217765b6e8f3163f6725d4aa3d303e4ffe4b99a5e85fb4ff91a026c17a8)
      witness-merkle-root
      witness-reserved-data
      raw-coinbase-tx
      (list 0x5f4a8858a112953111d5f94605c4dd8d04690eeeb9ffe2f435475d95943f2f3f 0x3348bf81aae79941662a902206b3ed2d285713668ab134c9e113548daea596fc)
    )
  )
)

;; @name parse-wtx is unable to parse a non-segwit transaction (returns unpredictable values)
(define-public (test-parse-wtx-on-non-segwit-1)
  (let (
    (segwit-tx 0x020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff03016500ffffffff0200f2052a010000002251205444612a122cd09b4b3457d46c149a23d8685fb7d3aac61ea7eee8449555293b0000000000000000266a24aa21a9edab124e70e4a18e72f2ac6f635aebb08862d5b5b1c0519cd9f3222a24a48482560120000000000000000000000000000000000000000000000000000000000000000000000000)
    (non-segwit-tx 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff2503831a250000000000000000000000000000000002000000000000073c7500000000000000ffffffff02be402500000000001976a91455a2e914aeb9729b4cd265248cb67a865eae95fd88ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf900000000)
    (parsed-tx-with-segwit-function (try! (contract-call? .clarity-bitcoin parse-wtx non-segwit-tx)))
    (parsed-tx-with-non-segwit-function (try! (contract-call? .clarity-bitcoin parse-tx non-segwit-tx)))
    (correct-parsed-tx { ins: (list { outpoint: { hash: 0x0000000000000000000000000000000000000000000000000000000000000000, index: u4294967295 }, scriptSig: 0x03831a250000000000000000000000000000000002000000000000073c7500000000000000, sequence: u4294967295 }), locktime: u0, outs: (list { scriptPubKey: 0x76a91455a2e914aeb9729b4cd265248cb67a865eae95fd88ac, value: u2441406 }  { scriptPubKey: 0x6a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9, value: u0 }), version: u2 })
    (wrong-parsed-tx { ins: (list), locktime: u0, outs: (list), segwit-marker: u1, segwit-version: u0, version: u2, witnesses: (list) })
  )
    (ok
      (asserts!
        (and 
          (is-eq parsed-tx-with-segwit-function wrong-parsed-tx)
          (is-eq parsed-tx-with-non-segwit-function correct-parsed-tx)
        )
      (err u1)))
  )
)

(define-constant ERR-OUT-OF-BOUNDS u1)
(define-constant ERR-TOO-MANY-TXINS u2)
(define-constant ERR-TOO-MANY-TXOUTS u3)
(define-constant ERR-VARSLICE-TOO-LONG u4)
(define-constant ERR-BAD-HEADER u5)
(define-constant ERR-PROOF-TOO-SHORT u6)
(define-constant ERR-TOO-MANY-WITNESSES u7)
(define-constant ERR-INVALID-COMMITMENT u8)
(define-constant ERR-WITNESS-TX-NOT-IN-COMMITMENT u9)
