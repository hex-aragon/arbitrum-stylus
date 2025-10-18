#!/bin/bash
cargo stylus deploy \
    --no-verify \
    --private-key 0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659 \
    --constructor-args 1


#     |
# 134 |         let _ = self.erc721._mint(minter,token_id);
#     |         +++++++

# deployed code at address: 0xab8e440727a38bbb180f7032ca4a8009e7b52b80
# deployment tx hash: 0xcb1cb81be48c630ab3849acd1a45809aed46ff5ec954ec548173e360f0faccdd

# NOTE: We recommend running cargo stylus cache bid ab8e440727a38bbb180f7032ca4a8009e7b52b80 0 to cache your activated contract in ArbOS.
# Cached contracts benefit from cheaper calls. To read more about the Stylus contract cache, see
# https://docs.arbitrum.io/stylus/how-tos/caching-contracts
# robert@aragon:~/work/arbitrum/stylus/stylus-contract/squiggle$ 
