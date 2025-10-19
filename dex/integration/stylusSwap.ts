import { getContract, zeroAddress, type Address } from "viem";
import { MockERC20ABI, StylusSwapABI } from "./abis";
import { walletClient } from "./chain";

export const StylusSwapAddress = "0x525c2aba45f66987217323e8a05ea400c65d06dc";

export const stylusSwap = getContract({
  abi: StylusSwapABI,
  address: StylusSwapAddress,
  client: walletClient,
});

// Create a new pool with the given tokens and fee
// Returns the txn receipt
export async function createPool(
  tokenOne: Address,
  tokenTwo: Address,
  fee: number
) {
  const createPoolHash = await stylusSwap.write.createPool([
    tokenOne,
    tokenTwo,
    fee,
  ]);

  const createPoolReceipt = await walletClient.waitForTransactionReceipt({
    hash: createPoolHash,
  });

  return createPoolReceipt;
}

// Add liquidity to a pool
// Returns the txn receipt
export async function addLiquidity(
  poolId: `0x${string}`,
  amount0Desired: bigint,
  amount1Desired: bigint,
  amount0Min: bigint,
  amount1Min: bigint,
  isToken0Native?: boolean
) {
  const addLiquidityHash = await stylusSwap.write.addLiquidity(
    [poolId, amount0Desired, amount1Desired, amount0Min, amount1Min],
    {
      value: isToken0Native ? amount0Desired : 0n,
    }
  );

  const addLiquidityReceipt = await walletClient.waitForTransactionReceipt({
    hash: addLiquidityHash,
  });

  return addLiquidityReceipt;
}

// Swap tokens in a pool
// Returns the txn receipt
export async function swap(
  poolId: `0x${string}`,
  inputAmount: bigint,
  minOutputAmount: bigint,
  zeroForOne: boolean,
  isToken0Native?: boolean
) {
  const addValue = isToken0Native && zeroForOne;

  const swapHash = await stylusSwap.write.swap(
    [poolId, inputAmount, minOutputAmount, zeroForOne],
    {
      value: addValue ? inputAmount : 0n,
    }
  );

  const swapReceipt = await walletClient.waitForTransactionReceipt({
    hash: swapHash,
  });

  return swapReceipt;
}

// Remove liquidity from a pool
// Returns the txn receipt
export async function removeLiquidity(
  poolId: `0x${string}`,
  liquidityToRemove: bigint
) {
  const removeLiquidityHash = await stylusSwap.write.removeLiquidity([
    poolId,
    liquidityToRemove,
  ]);

  const removeLiquidityReceipt = await walletClient.waitForTransactionReceipt({
    hash: removeLiquidityHash,
  });

  return removeLiquidityReceipt;
}

// Get the liquidity in a user's position
// Returns the liquidity
export async function getPositionLiquidity(poolId: `0x${string}`) {
  const positionLiquidity = await stylusSwap.read.getPositionLiquidity([
    poolId,
    walletClient.account.address,
  ]);
  return positionLiquidity;
}

// Get the balance of a token in the user's wallet
// Returns ETH balance if the token is the zero address, otherwise uses ERC20 `.balanceOf(...)`
// Returns the balance
export async function getBalance(token: Address) {
  if (token === zeroAddress) {
    return walletClient.getBalance({ address: walletClient.account.address });
  }

  const tokenContract = getContract({
    abi: MockERC20ABI,
    address: token,
    client: walletClient,
  });

  const balance = await tokenContract.read.balanceOf([
    walletClient.account.address,
  ]);
  return balance;
}