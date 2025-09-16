//! Types used by the multi-coin pallet.

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

/// Type alias for coin identifiers
pub type CoinId = u32;

/// Information about a coin
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CoinInfo<Symbol, Name, AccountId, Balance, FeeConfig> {
	/// The coin symbol (e.g., "BTC", "ETH")
	pub symbol: Symbol,
	/// The coin name (e.g., "Bitcoin", "Ethereum") 
	pub name: Name,
	/// Number of decimal places
	pub decimals: u8,
	/// The account that owns this coin
	pub owner: AccountId,
	/// Deposit paid for creating this coin
	pub deposit: Balance,
	pub fee_config: FeeConfig, // New: Add fee configuration
}

/// Coin creation parameters
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CreateCoinParams {
	/// The coin symbol
	pub symbol: Vec<u8>,
	/// The coin name
	pub name: Vec<u8>,
	/// Number of decimal places
	pub decimals: u8,
	/// Initial supply to mint to creator
	pub initial_supply: u128,
}

/// Transfer parameters
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct TransferParams<AccountId> {
	/// The coin to transfer
	pub coin_id: CoinId,
	/// The recipient
	pub to: AccountId,
	/// The amount to transfer
	pub amount: u128,
}

/// Mint parameters
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MintParams<AccountId> {
	/// The coin to mint
	pub coin_id: CoinId,
	/// The account to mint to
	pub to: AccountId,
	/// The amount to mint
	pub amount: u128,
}

/// Burn parameters
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct BurnParams {
	/// The coin to burn
	pub coin_id: CoinId,
	/// The amount to burn
	pub amount: u128,
}

/// Balance information for an account
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct AccountBalance {
	/// Free balance
	pub free: u128,
	/// Reserved balance
	pub reserved: u128,
	/// Frozen balance
	pub frozen: u128,
}

impl AccountBalance {
	/// Get the total balance (free + reserved)
	pub fn total(&self) -> u128 {
		self.free.saturating_add(self.reserved)
	}

	/// Get the usable balance (free - frozen)
	pub fn usable(&self) -> u128 {
		self.free.saturating_sub(self.frozen)
	}
}

/// Coin statistics
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct CoinStats {
	/// Total supply
	pub total_supply: u128,
	/// Number of holders
	pub holders: u32,
	/// Number of transfers
	pub transfers: u64,
	/// Total minted
	pub total_minted: u128,
	/// Total burned
	pub total_burned: u128,
}

/// Role permissions for a coin
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct CoinPermissions {
	/// Can mint new coins
	pub can_mint: bool,
	/// Can burn coins
	pub can_burn: bool,
	/// Can modify metadata
	pub can_modify_metadata: bool,
	/// Can transfer ownership
	pub can_transfer_ownership: bool,
}

/// Fee configuration for a coin
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct FeeConfig {
	/// Transfer fee per transaction
	pub transfer_fee: u128,
	/// Minimum balance required
	pub minimum_balance: u128,
	/// Whether this coin can be used to pay transaction fees
	pub can_pay_tx_fees: bool,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct TransferFeeConfig {
    /// Fixed fee per transfer
    pub fixed_fee: u128,
    /// Percentage fee (in basis points, 10000 = 100%)
    pub percentage_fee: u16,
    /// Minimum fee
    pub minimum_fee: u128,
    /// Maximum fee
    pub maximum_fee: u128,
}
