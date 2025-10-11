# 특정 값으로 설정
cast send \
    --rpc-url http://localhost:8547 \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    0x525c2aba45f66987217323e8a05ea400c65d06dc "setNumber(uint256)" 100

# 값 더하기
cast send \
    --rpc-url http://localhost:8547 \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    0x525c2aba45f66987217323e8a05ea400c65d06dc "addNumber(uint256)" 50

# 값 곱하기
cast send \
    --rpc-url http://localhost:8547 \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    0x525c2aba45f66987217323e8a05ea400c65d06dc "mulNumber(uint256)" 2