import { expect, test } from "bun:test";
import { deployMockErc20 } from "./mockErc20";
import {
  addLiquidity,
  createPool,
  getBalance,
  getPositionLiquidity,
  removeLiquidity,
  stylusSwap,
  swap,
} from "./stylusSwap";
import { zeroAddress } from "viem";

test("Cannot create pool with same token pair and fee value twice", async () => {
  const tokenOne = await deployMockErc20("Test One", "ONE");
  const tokenTwo = await deployMockErc20("Test Two", "TWO");

  await createPool(tokenOne, tokenTwo, 1000);

  expect(createPool(tokenOne, tokenTwo, 1000)).rejects.toThrow(
    "PoolAlreadyExists"
  );
});

test("Cannot add liquidity or swap in a pool that does not exist", async () => {
  const randomPoolId =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

  expect(
    addLiquidity(randomPoolId, 100_000n, 100_000n, 0n, 0n)
  ).rejects.toThrow("PoolDoesNotExist");

  expect(swap(randomPoolId, 10n, 0n, true)).rejects.toThrow("PoolDoesNotExist");
});

test("Cannot remove more liquidity than you have", async () => {
  const tokenOne = await deployMockErc20("Test One", "ONE");
  const tokenTwo = await deployMockErc20("Test Two", "TWO");

  const [poolId, _token0, _token1] = await stylusSwap.read.getPoolId([
    tokenOne,
    tokenTwo,
    1000,
  ]);

  await createPool(tokenOne, tokenTwo, 1000);
  await addLiquidity(poolId, 100_000n, 100_000n, 0n, 0n);

  expect(removeLiquidity(poolId, 500_000n)).rejects.toThrow(
    "InsufficientLiquidityOwned"
  );
});



test("Two ERC-20 Tokens, 10% fee", async () => {
  // Deploy a couple of mock ERC-20 tokens, and create a new pool
  const tokenOne = await deployMockErc20("Test One", "ONE");
  const tokenTwo = await deployMockErc20("Test Two", "TWO");
  const [poolId, token0, token1] = await stylusSwap.read.getPoolId([
    tokenOne,
    tokenTwo,
    1000,
  ]);
  await createPool(tokenOne, tokenTwo, 1000);

  // Load the balances of the tokens in our wallet at this stage as our original balances
  const [originalToken0Balance, originalToken1Balance] = await Promise.all([
    getBalance(token0),
    getBalance(token1),
  ]);

  // Add some liquidity to the pool
  // We are adding equal amounts of both tokens, effectively setting the price of 1 token0 = 1 token1
  await addLiquidity(poolId, 100_000n, 100_000n, 0n, 0n);

  // Load the balances of the tokens in our wallet after adding liquidity
  const [afterAddLiquidityToken0Balance, afterAddLiquidityToken1Balance] =
    await Promise.all([getBalance(token0), getBalance(token1)]);

  // The amount of tokens deducted from our wallet should be equal to the desired amount of tokens
  // we wanted to add as liquidity, since this was first-time liquidity into the pool
  const token0AddedAsLiquidity =
    originalToken0Balance - afterAddLiquidityToken0Balance;
  const token1AddedAsLiquidity =
    originalToken1Balance - afterAddLiquidityToken1Balance;
  expect(token0AddedAsLiquidity).toEqual(100_000n);
  expect(token1AddedAsLiquidity).toEqual(100_000n);

  // But, the liquidity owned by us in our LP Position should be slightly less - as the minimum lockup liquidity is locked up forever
  const userLiquidity = await getPositionLiquidity(poolId);
  expect(userLiquidity).toEqual(100_000n - 1000n);

  // Try swapping 10 tokens of token0 for token1
  await swap(poolId, 10n, 0n, true);

  // Load the balances of the tokens in our wallet after swapping
  const [afterSwapToken0Balance, afterSwapToken1Balance] = await Promise.all([
    getBalance(token0),
    getBalance(token1),
  ]);

  // We should have spent 10 tokens of token0
  // And we should have received 9 tokens of token1 (10 - 1) since the swap fee in the pool is 10%
  const token0Spent = afterAddLiquidityToken0Balance - afterSwapToken0Balance;
  const token1Gained = afterSwapToken1Balance - afterAddLiquidityToken1Balance;
  expect(token0Spent).toEqual(10n);
  expect(token1Gained).toEqual(9n);

  // Remove full liquidity from the pool
  await removeLiquidity(poolId, userLiquidity);
  const [afterRemoveLiquidityToken0Balance, afterRemoveLiquidityToken1Balance] =
    await Promise.all([getBalance(token0), getBalance(token1)]);

  // We should have received the full balance of the pool as we are the only LP, minus the minimum lockup
  const token0Removed =
    afterRemoveLiquidityToken0Balance - afterSwapToken0Balance;
  const token1Removed =
    afterRemoveLiquidityToken1Balance - afterSwapToken1Balance;

  // Originally we added 100k token0 as liquidity, of which 99000 was removable after minimum lockup
  // We supplied 10 more by swapping bringing the total up to 99010
  // Due to math rounding while calculating sqrts in the code, token0 share comes out to be 99009
  expect(token0Removed).toEqual(99_009n);

  // Originally we added 100k token1 as liquidity, of which 99000 was removable after minimum lockup
  // We swapped 10 token0 for 9 token1, bringing redeemable token1 balance in the pool down to 98991
  expect(token1Removed).toEqual(98_991n);
});

test("ETH and ERC-20 Token, 10% fee", async () => {
  // Deploy a mock ERC-20 token to create the pool with
  const token = await deployMockErc20("Test", "TST");
  const [poolId, token0, token1] = await stylusSwap.read.getPoolId([
    zeroAddress,
    token,
    1000,
  ]);
  await createPool(zeroAddress, token, 1000);

  // Load the balances of the tokens in our wallet at this stage as our original balances
  const [originalToken0Balance, originalToken1Balance] = await Promise.all([
    getBalance(zeroAddress),
    getBalance(token),
  ]);

  // Add some liquidity to the pool
  // We are adding equal amounts of both tokens, effectively setting the price of 1 ETH = 1 TOKEN
  const addLiquidityReceipt = await addLiquidity(
    poolId,
    100_000n,
    100_000n,
    0n,
    0n,
    true
  );

  // Calculate the amount of ETH spent on gas for the add liquidity transaction
  const ethSpentOnGas =
    addLiquidityReceipt.cumulativeGasUsed *
    addLiquidityReceipt.effectiveGasPrice;

  // Load the balances of the tokens in our wallet after adding liquidity
  const [afterAddLiquidityToken0Balance, afterAddLiquidityToken1Balance] =
    await Promise.all([getBalance(zeroAddress), getBalance(token)]);

  // ETH withdrawn from our wallet should be equal to amount that got added as liquidity PLUS amount that we spent as gas fee on this transaction
  const token0AddedAsLiquidity =
    originalToken0Balance - ethSpentOnGas - afterAddLiquidityToken0Balance;
  // TEST token withdrawn from our wallet should be equal to amount that got added as liquidity
  const token1AddedAsLiquidity =
    originalToken1Balance - afterAddLiquidityToken1Balance;
  expect(token0AddedAsLiquidity).toEqual(100_000n);
  expect(token1AddedAsLiquidity).toEqual(100_000n);

  // Swap 10 ETH for TEST token
  const swapReceipt = await swap(poolId, 10n, 0n, true, true);

  // Calculate the amount of ETH spent on gas for the swap transaction
  const ethSpentOnGasSwap =
    swapReceipt.cumulativeGasUsed * swapReceipt.effectiveGasPrice;

  // Load the balances of the tokens in our wallet after swapping
  const [afterSwapToken0Balance, afterSwapToken1Balance] = await Promise.all([
    getBalance(zeroAddress),
    getBalance(token),
  ]);

  // Amount of ETH withdrawn from our wallet should be equal to our swap amount PLUS amount we spent on gas
  const token0Spent =
    afterAddLiquidityToken0Balance - ethSpentOnGasSwap - afterSwapToken0Balance;
  // Amount of TEST token received in our wallet should be 9 (10 - 1) due to 10% swap fee in the pool
  const token1Gained = afterSwapToken1Balance - afterAddLiquidityToken1Balance;
  expect(token0Spent).toEqual(10n);
  expect(token1Gained).toEqual(9n);
});