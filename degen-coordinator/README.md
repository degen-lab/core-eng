what to be run from coordinator


##### shell commands
```shell
relay-server ❯ RUST_LOG=info cargo run --bin relay-server
degen-signer ❯ RUST_LOG=info cargo run run --id 1 --config conf/signer.toml
```

##### coordinator - commands for ide
```shell
Command
run -- --config ./conf/coordinator.toml --signer-config ./conf/signer.toml dkg-sign

Environment variables
RUST_LOG=info

Working directory
add current subfolder /degen-coordinator
```




