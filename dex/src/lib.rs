#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;

use alloy_primitives::{aliases::U24, Address, FixedBytes, U256};
use alloy_sol_types::{sol, SolValue};
/// Import items from the SDK. The prelude contains common traits and macros.
use stylus_sdk::{crypto::keccak, prelude::*};

// Define a minimal ERC20 interface, so our contract can transfer ERC-20 tokens when it needs to
sol_interface! {
    interface IERC20 {
        function transferFrom(address from, address to, uint256 value) external returns (bool);
        function transfer(address to, uint256 value) external returns (bool);
    }
}

// Define some persistent storage using the Solidity ABI
// `StylusSwap` will be the entrypoint
sol_storage! {
    #[entrypoint]
    pub struct StylusSwap {
        // Mapping of all pools created within the DEX
        mapping(bytes32 => Pool) pools;
    }

    // A pool is a pair of tokens and a fee which together uniquely identify the pool
    // The struct contains additional data that is used to track the pool's state
    pub struct Pool {
        address token0;
        address token1;
        uint24 fee;
        uint256 liquidity;
        uint256 balance0;
        uint256 balance1;
        mapping(bytes32 => Position) positions;
    }

    // A position is a user's share of the pool's liquidity
    pub struct Position {
        address owner;
        uint256 liquidity;
    }
}

sol! {
    // Thrown when a pool with the same ID already exists
    error PoolAlreadyExists(bytes32 pool_id);
    // Thrown when an action is attempted on a pool that does not exist
    error PoolDoesNotExist(bytes32 pool_id);
    // Thrown when a user attempts to mint liquidity without providing enough tokens
    error InsufficientLiquidityMinted();
    // Thrown when a user attempts to swap with an insufficient amount of tokens
    error InsufficientAmount();
    // Thrown when a user attempts to remove liquidity more than their share of the pool
    error InsufficientLiquidityOwned();
    // Thrown when a token transfer fails
    error FailedOrInsufficientTokenTransfer(address token, address from, address to, uint256 amount);
    // Thrown when the contract fails to refund leftover ETH to the user
    error FailedToReturnExtraEth(address to, uint256 amount);
    // Thrown when the user's swap exceeds their slippage tolerance
    error TooMuchSlippage();

    // Emitted when a pool is created
    event PoolCreated(bytes32 pool_id, address token0, address token1, uint24 fee);
    // Emitted when liquidity is minted
    event LiquidityMinted(bytes32 pool_id, address owner, uint256 liquidity);
    // Emitted when liquidity is burned
    event LiquidityBurned(bytes32 pool_id, address owner, uint256 liquidity);
    // Emitted when a swap is executed
    event Swap(bytes32 pool_id, address user, uint256 input_amount, uint256 output_amount_after_fees, uint256 fees, bool zero_for_one);
}

// Define the Rust-equivalent of the Solidity errors
#[derive(SolidityError)]
pub enum StylusSwapError {
    PoolAlreadyExists(PoolAlreadyExists),
    PoolDoesNotExist(PoolDoesNotExist),
    InsufficientAmount(InsufficientAmount),
    InsufficientLiquidityMinted(InsufficientLiquidityMinted),
    InsufficientLiquidityOwned(InsufficientLiquidityOwned),
    FailedOrInsufficientTokenTransfer(FailedOrInsufficientTokenTransfer),
    FailedToReturnExtraEth(FailedToReturnExtraEth),
    TooMuchSlippage(TooMuchSlippage),
}

impl StylusSwap {
    fn try_transfer_token(
        &mut self,
        token: Address,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<(), StylusSwapError> {
        let address_this = self.vm().contract_address();

        if from != address_this && to != address_this {
            // We are transferring tokens between two addresses where we are neither the sender nor the receiver
            return Err(StylusSwapError::FailedOrInsufficientTokenTransfer(
                FailedOrInsufficientTokenTransfer {
                    token,
                    from,
                    to,
                    amount,
                },
            ));
        }

        // We are transferring ETH
        if token.is_zero() {
            if from == address_this {
                // We are sending ETH out
                let result = self.vm().transfer_eth(to, amount);
                if result.is_err() {
                    return Err(StylusSwapError::FailedOrInsufficientTokenTransfer(
                        FailedOrInsufficientTokenTransfer {
                            token,
                            from,
                            to,
                            amount,
                        },
                    ));
                }
            } else if to == address_this {
                // We are receiving ETH
                if self.vm().msg_value() < amount {
                    return Err(StylusSwapError::FailedOrInsufficientTokenTransfer(
                        FailedOrInsufficientTokenTransfer {
                            token,
                            from,
                            to,
                            amount,
                        },
                    ));
                }

                // Refund any excess ETH back to the sender
                let extra_eth = self.vm().msg_value() - amount;
                if extra_eth > U256::ZERO {
                    self.try_transfer_token(token, address_this, from, extra_eth)?;
                }
            }
        }
        // We are transferring an ERC-20 token
        else {
            let token_contract = IERC20::new(token);
            if from == address_this {
                // We are sending the token out
                let result = token_contract.transfer(&mut *self, to, amount);
                if result.is_err() || result.unwrap() == false {
                    return Err(StylusSwapError::FailedOrInsufficientTokenTransfer(
                        FailedOrInsufficientTokenTransfer {
                            token,
                            from,
                            to,
                            amount,
                        },
                    ));
                }
            } else if to == address_this {
                // We are receiving the token
                let result = token_contract.transfer_from(&mut *self, from, to, amount);
                if result.is_err() || result.unwrap() == false {
                    return Err(StylusSwapError::FailedOrInsufficientTokenTransfer(
                        FailedOrInsufficientTokenTransfer {
                            token,
                            from,
                            to,
                            amount,
                        },
                    ));
                }
            }
        }

        Ok(())
    }

    // Given a U256 value, return the integer square root of the value
    fn integer_sqrt(&self, x: U256) -> U256 {
        let two = U256::from(2);

        let mut z: U256 = (x + U256::from(1)) >> 1;
        let mut y = x;

        while z < y {
            y = z;
            z = (x / z + z) / two;
        }

        y
    }

    // Given two U256 values, return the smaller of the two
    fn min(&self, x: U256, y: U256) -> U256 {
        if x < y {
            return x;
        }

        y
    }
}

#[public]
impl StylusSwap {
    pub fn create_pool(
        &mut self,
        token_a: Address,
        token_b: Address,
        fee: U24,
    ) -> Result<(), StylusSwapError> {
        let (pool_id, token0, token1) = self.get_pool_id(token_a, token_b, fee);
        let existing_pool = self.pools.get(pool_id);

        // If one of the token addresses of this pool in the mapping is non-zero, the pool already exists
        // in our mapping
        if !existing_pool.token0.get().is_zero() || !existing_pool.token1.get().is_zero() {
            return Err(StylusSwapError::PoolAlreadyExists(PoolAlreadyExists {
                pool_id: pool_id,
            }));
        }

        let mut pool_setter = self.pools.setter(pool_id);
        pool_setter.token0.set(token0);
        pool_setter.token1.set(token1);
        pool_setter.fee.set(fee);

        // Initially the pool has no liquidity or token balances
        pool_setter.liquidity.set(U256::from(0));
        pool_setter.balance0.set(U256::from(0));
        pool_setter.balance1.set(U256::from(0));

        // Emit the PoolCreated event
        log(
            self.vm(),
            PoolCreated {
                pool_id,
                token0,
                token1,
                fee,
            },
        );

        Ok(())
    }

    // This function is used to add liquidity to a pool. It takes in the pool ID, the desired
    // amounts of each token, and the minimum amounts of each token.
    // It returns an error if the pool does not exist, if the user's desired amounts are
    // insufficient, or if we fail to transfer the tokens to the pool.
    #[payable]
    pub fn add_liquidity(
        &mut self,
        pool_id: FixedBytes<32>,
        amount_0_desired: U256,
        amount_1_desired: U256,
        amount_0_min: U256,
        amount_1_min: U256,
    ) -> Result<(), StylusSwapError> {
        let msg_sender = self.vm().msg_sender();
        let address_this = self.vm().contract_address();

        // Load the pool's current state
        let pool = self.pools.get(pool_id);
        let token0 = pool.token0.get();
        let token1 = pool.token1.get();

        // If both token addresses are zero, this pool is not initialized and does not exist
        if token0.is_zero() && token1.is_zero() {
            return Err(StylusSwapError::PoolDoesNotExist(PoolDoesNotExist {
                pool_id,
            }));
        }

        let balance0 = pool.balance0.get();
        let balance1 = pool.balance1.get();
        let liquidity = pool.liquidity.get();
        let is_initial_liquidity = liquidity.is_zero();

        // Load the user's current position in the pool (default zero if they don't have one)
        let position_id = self.get_position_id(pool_id, msg_sender);
        let user_position = pool.positions.get(position_id);
        let user_liquidity = user_position.liquidity.get();

        let (amount0, amount1) = self.get_liquidity_amounts(
            amount_0_desired,
            amount_1_desired,
            amount_0_min,
            amount_1_min,
            balance0,
            balance1,
        )?;

        // Calculate the new share of the pool's liquidity that the user will own
        let new_user_liquidity = if is_initial_liquidity {
            self.integer_sqrt(amount0 * amount1) - U256::from(1000) // subtract minimum liquidity
        } else {
            let l_0 = (amount0 * liquidity) / balance0;
            let l_1 = (amount1 * liquidity) / balance1;
            self.min(l_0, l_1)
        };

        // Calculate the new liquidity being added to the pool (same as the user's new liquidity if it's not the first time)
        let new_pool_liquidity = if is_initial_liquidity {
            new_user_liquidity + U256::from(1000) // Pool's total liquidity includes the minimum liquidity
        } else {
            new_user_liquidity
        };

        if new_pool_liquidity.is_zero() {
            return Err(StylusSwapError::InsufficientLiquidityMinted(
                InsufficientLiquidityMinted {},
            ));
        }

        // Update the pool's state (total liquidity, token balances, and user's position)
        let mut pool_setter = self.pools.setter(pool_id);
        pool_setter.liquidity.set(liquidity + new_pool_liquidity);
        pool_setter.balance0.set(balance0 + amount0);
        pool_setter.balance1.set(balance1 + amount1);

        let mut user_position_setter = pool_setter.positions.setter(position_id);
        user_position_setter
            .liquidity
            .set(user_liquidity + new_user_liquidity);
        user_position_setter.owner.set(msg_sender);

        // Transfer amount0 of token0 and amount1 of token1 to the pool
        self.try_transfer_token(token0, msg_sender, address_this, amount0)?;
        self.try_transfer_token(token1, msg_sender, address_this, amount1)?;

        // Emit the LiquidityMinted event
        log(
            self.vm(),
            LiquidityMinted {
                pool_id,
                owner: msg_sender,
                liquidity: new_pool_liquidity,
            },
        );

        Ok(())
    }

    // This function is used to remove liquidity from a pool. It takes in the pool ID and the
    // amount of liquidity to remove.
    // It returns an error if the pool does not exist, if the user's liquidity is insufficient,
    // or if we fail to transfer the tokens to the user.
    pub fn remove_liquidity(
        &mut self,
        pool_id: FixedBytes<32>,
        liquidity_to_remove: U256,
    ) -> Result<(), StylusSwapError> {
        let msg_sender = self.vm().msg_sender();
        let address_this = self.vm().contract_address();

        // Load the pool's current state
        let pool = self.pools.get(pool_id);
        let token0 = pool.token0.get();
        let token1 = pool.token1.get();

        // If both token addresses are zero, this pool is not initialized and does not exist
        if token0.is_zero() && token1.is_zero() {
            return Err(StylusSwapError::PoolDoesNotExist(PoolDoesNotExist {
                pool_id,
            }));
        }

        let balance0 = pool.balance0.get();
        let balance1 = pool.balance1.get();
        let liquidity = pool.liquidity.get();

        // Load the user's current position in the pool (default zero if they don't have one)
        let position_id = self.get_position_id(pool_id, msg_sender);
        let user_position = pool.positions.get(position_id);
        let user_liquidity = user_position.liquidity.get();

        if liquidity_to_remove > user_liquidity {
            return Err(StylusSwapError::InsufficientLiquidityOwned(
                InsufficientLiquidityOwned {},
            ));
        }

        // The amount of tokens to be removed is the % share of the pool's balance of each token
        // based on the user's share of the pool's liquidity
        // e.g. If user owns 10% of the pool's total liquidity, they will receive 10% of the pool's
        // token0 balance, and 10% of the pool's token1 balance
        let amount_0 = (balance0 * liquidity_to_remove) / liquidity;
        let amount_1 = (balance1 * liquidity_to_remove) / liquidity;

        if amount_0.is_zero() || amount_1.is_zero() {
            return Err(StylusSwapError::InsufficientLiquidityOwned(
                InsufficientLiquidityOwned {},
            ));
        }

        let mut pool_setter = self.pools.setter(pool_id);
        pool_setter.liquidity.set(liquidity - liquidity_to_remove);
        pool_setter.balance0.set(balance0 - amount_0);
        pool_setter.balance1.set(balance1 - amount_1);
        let mut position_setter = pool_setter.positions.setter(position_id);
        position_setter
            .liquidity
            .set(user_liquidity - liquidity_to_remove);

        // Transfer amount0 of token0 and amount1 of token1 to the user
        self.try_transfer_token(token0, address_this, msg_sender, amount_0)?;
        self.try_transfer_token(token1, address_this, msg_sender, amount_1)?;

        // Emit the LiquidityBurned event
        log(
            self.vm(),
            LiquidityBurned {
                pool_id,
                owner: msg_sender,
                liquidity: liquidity_to_remove,
            },
        );

        Ok(())
    }

    // This function is used to swap tokens in a pool. It takes in the pool ID, the amount of
    // input tokens to swap, the minimum amount of output tokens to receive, and a boolean
    // indicating whether to swap is to sell token0 or token1.
    // It returns an error if the pool does not exist, if the user's input amount is insufficient,
    // or if we fail to transfer the tokens to the pool.
    #[payable]
    pub fn swap(
        &mut self,
        pool_id: FixedBytes<32>,
        input_amount: U256,
        min_output_amount: U256,
        zero_for_one: bool,
    ) -> Result<(), StylusSwapError> {
        if input_amount.is_zero() {
            return Err(StylusSwapError::InsufficientAmount(InsufficientAmount {}));
        }

        let msg_sender = self.vm().msg_sender();
        let address_this = self.vm().contract_address();

        // Load the pool's current state
        let pool = self.pools.get(pool_id);
        let token0 = pool.token0.get();
        let token1 = pool.token1.get();

        // If both token addresses are zero, this pool is not initialized and does not exist
        if token0.is_zero() && token1.is_zero() {
            return Err(StylusSwapError::PoolDoesNotExist(PoolDoesNotExist {
                pool_id,
            }));
        }

        let balance0 = pool.balance0.get();
        let balance1 = pool.balance1.get();
        let fee = pool.fee.get();

        let original_k = balance0 * balance1;

        let input_token = if zero_for_one { token0 } else { token1 };
        let output_token = if zero_for_one { token1 } else { token0 };
        let input_balance = if zero_for_one { balance0 } else { balance1 };
        let output_balance = if zero_for_one { balance1 } else { balance0 };

        // Here we solve for xy = k to keep k constant
        // i.e. (input_balance * output_balance) = original_k
        // ((input_balance + input_amount) * (output_balance - output_amount)) = original_k
        // Therefore, (input_balance * output_balance) = (input_balance + input_amount) * (output_balance - output_amount)
        // Solving for output_amount:
        // output_amount = output_balance - ((input_balance * output_balance) / (input_balance + input_amount))
        // i.e. output_amount = output_balance - (original_k / (input_balance + input_amount))
        let output_amount = output_balance - (original_k / (input_balance + input_amount));

        // Now we apply swap fees on the output amount so LPs earn some yield for providing liquidity
        // First, we calculate the amount of fees to deduct
        let fees = (output_amount * U256::from(fee)) / U256::from(10_000);
        // Then, we calculate how much output amount the user will get after fees
        let output_amount_after_fees = output_amount - fees;

        // If the user's output amount is less than the minimum output amount, we return an error
        if output_amount_after_fees < min_output_amount {
            return Err(StylusSwapError::TooMuchSlippage(TooMuchSlippage {}));
        }

        // Now we update the pool state (token balances)
        let mut pool_setter = self.pools.setter(pool_id);
        if zero_for_one {
            pool_setter.balance0.set(balance0 + input_amount);
            pool_setter
                .balance1
                .set(balance1 - output_amount_after_fees);
        } else {
            pool_setter
                .balance0
                .set(balance0 - output_amount_after_fees);
            pool_setter.balance1.set(balance1 + input_amount);
        }

        // Transfer the input token from user to pool
        self.try_transfer_token(input_token, msg_sender, address_this, input_amount)?;
        // Transfer the output token from pool to user
        self.try_transfer_token(
            output_token,
            address_this,
            msg_sender,
            output_amount_after_fees,
        )?;

        // Emit the Swap event
        log(
            self.vm(),
            Swap {
                pool_id,
                user: msg_sender,
                input_amount,
                output_amount_after_fees,
                fees,
                zero_for_one,
            },
        );
        Ok(())
    }

    // Given two arbitrary token addresses and a fee value, compute a determinsitic Pool ID
    // irrespective of the order of the tokens in the supplied arguments
    // Returns the Pool ID, the token0 address, and the token1 address
    pub fn get_pool_id(
        &self,
        token_a: Address,
        token_b: Address,
        fee: U24,
    ) -> (FixedBytes<32>, Address, Address) {
        let token0: Address;
        let token1: Address;

        // Sort the tokens to ensure determinism
        if token_a <= token_b {
            token0 = token_a;
            token1 = token_b;
        } else {
            token0 = token_b;
            token1 = token_a;
        }

        let hash_data = (token0, token1, fee);
        let pool_id = keccak(hash_data.abi_encode_sequence());

        (pool_id, token0, token1)
    }

    // Given a pool ID and an owner address, compute a determinsitic Position ID
    // Returns the Position ID
    pub fn get_position_id(&self, pool_id: FixedBytes<32>, owner: Address) -> FixedBytes<32> {
        let hash_data = (pool_id, owner);
        let position_id = keccak(hash_data.abi_encode_sequence());
        position_id
    }

    // Given a pool ID and an owner address, return the user's position liquidity
    pub fn get_position_liquidity(&self, pool_id: FixedBytes<32>, owner: Address) -> U256 {
        let position_id = self.get_position_id(pool_id, owner);
        let pool = self.pools.get(pool_id);
        let position = pool.positions.get(position_id);
        position.liquidity.get()
    }

    // This function is used to calculate the amounts of tokens to transfer to the pool
    // when adding liquidity. It takes in the desired amounts of each token, the minimum
    // amounts of each token, and the current balances of the pool.
    pub fn get_liquidity_amounts(
        &self,
        amount_0_desired: U256,
        amount_1_desired: U256,
        amount_0_min: U256,
        amount_1_min: U256,
        balance0: U256,
        balance1: U256,
    ) -> Result<(U256, U256), StylusSwapError> {
        // If the pool has no balance of either token already, this is initial liquidity
        // so we can just return the desired amounts
        if balance0.eq(&U256::ZERO) && balance1.eq(&U256::ZERO) {
            return Ok((amount_0_desired, amount_1_desired));
        }

        // Otherwise, we need to check if their desired amounts are within the bounds of the pool
        let amount_1_optimal = (amount_0_desired * balance1) / balance0;
        if amount_1_optimal <= amount_1_desired {
            if amount_1_optimal < amount_1_min {
                return Err(StylusSwapError::InsufficientAmount(InsufficientAmount {}));
            }

            return Ok((amount_0_desired, amount_1_optimal));
        }

        let amount_0_optimal = (amount_1_desired * balance0) / balance1;
        if amount_0_optimal < amount_0_min {
            return Err(StylusSwapError::InsufficientAmount(InsufficientAmount {}));
        }

        return Ok((amount_0_optimal, amount_1_desired));
    }
}