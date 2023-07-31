# fund.sh
bitcoin-cli -regtest -rpcpassword=devnet -rpcuser=devnet generatetoaddress 1 <btc-addr-1>
bitcoin-cli -regtest -rpcpassword=devnet -rpcuser=devnet generatetoaddress 1 <btc-addr-2>
bitcoin-cli -regtest -rpcpassword=devnet -rpcuser=devnet generatetoaddress 101 <btc-addr-3>