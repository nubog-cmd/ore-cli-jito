#!/bin/bash

echo "Wallet: $1"

while true
do
  echo "Running"
   ./target/release/ore --rpc $2 --jito-client $3 --keypair $1 --priority-fee 1001 --jito-enable --jito-fee 898765 mine
   --threads 8
  echo "Exited"
done