#!/bin/sh
mkdir -p .integration
cp Clarinet.toml .integration/Clarinet.toml
mkdir -p .integration/settings
cp settings/Devnet.toml .integration/settings/Devnet.toml
cp -r ./contracts .integration
cp -r ./tests .integration
cp -r ./deployments .integration
cd .integration
sed -i "s/(contract-call? .pox-3 get-stx-account user)/(stx-account user)/" contracts/sbtc-stacking-pool.clar
find . -name '*.clar' -print0 | xargs -0 sed -i "s/ .pox-3/ 'ST000000000000000000002AMW42H.pox-3/g"
sed -i "s/(define-constant normal-cycle-len u[0-9]*)/(define-constant normal-cycle-len u10)/" contracts/sbtc-stacking-pool.clar
sed -i "s/(define-constant normal-voting-period-len u[0-9]*)/(define-constant normal-voting-period-len u2)/" contracts/sbtc-stacking-pool.clar
sed -i "s/(define-constant normal-transfer-period-len u[0-9]*)/(define-constant normal-transfer-period-len u1)/" contracts/sbtc-stacking-pool.clar
sed -i "s/(define-constant normal-penalty-period-len u[0-9]*)/(define-constant normal-penalty-period-len u1)/" contracts/sbtc-stacking-pool.clar
clarinet integrate