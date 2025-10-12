#!/bin/bash
# cast call \
#     --rpc-url http://localhost:8547 \
#     --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
#     <YOUR_CONTRACT_ADDRESS> "number()(uint256)"

FOUNDRY_DISABLE_NIGHTLY_WARNING=1 cast call \
    --rpc-url http://localhost:8547 \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    0x525c2aba45f66987217323e8a05ea400c65d06dc "number()(uint256)"