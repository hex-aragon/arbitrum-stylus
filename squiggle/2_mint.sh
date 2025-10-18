# cast call YOUR_CONTRACT_ADDRESS \
#     --rpc-url http://localhost:8547 "tokenURI(uint256)(string)" 0

# 1번에서 나온 deployed code at address  >> ca 주소를 YOUR_CONTRACT_ADDRESS에 대입 

#!/bin/bash

cast send 0xab8e440727a38bbb180f7032ca4a8009e7b52b80 \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    --value 1 \
    --rpc-url http://localhost:8547 "mint()"