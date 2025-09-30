//! # Multi-Coin Pallet
//!
//! A FRAME pallet that enables a blockchain to natively support and manage multiple coins
//! on a single runtime. Each coin is treated as a native asset with individual state,
//! supply, and economic logic.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Creating and registering multiple native coins
//! - Managing coin ownership and roles (admin, minter)
//! - Transferring coins between accounts
//! - Minting and burning coins with proper permissions
//! - On-chain metadata storage and retrieval
//! - Multi-currency fee payment support
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_coin` - Create a new coin with metadata and initial supply
//! * `transfer` - Transfer coins between accounts
//! * `mint` - Mint new coins (requires permissions)
//! * `burn` - Burn coins (requires permissions)
//! * `set_metadata` - Update coin metadata (admin only)
//! * `transfer_ownership` - Transfer coin ownership
//! * `set_mint_permission` - Grant/revoke minting permissions
//!
//! ### Public Functions
//!
//! * `balance_of` - Query account balance for a specific coin
//! * `total_supply` - Get total supply of a coin
//! * `get_metadata` - Retrieve coin metadata
//! * `get_coin_id_by_symbol` - Find coin ID by symbol

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_support::BoundedVec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

// Type definitions
pub mod types;
pub use types::*;

pub mod transaction_payment;
pub use transaction_payment::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Get, Currency, ReservableCurrency},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use sp_runtime::traits::Dispatchable;
	use scale_info::prelude::boxed::Box;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// The currency used for reserving funds for coin creation
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Maximum length of coin symbol
		#[pallet::constant]
		type MaxSymbolLength: Get<u32>;

		/// Maximum length of coin name
		#[pallet::constant]
		type MaxNameLength: Get<u32>;

		/// Maximum number of coins that can be created
		#[pallet::constant]
		type MaxCoins: Get<u32>;

		/// Deposit required for creating a coin
		#[pallet::constant]
		type CoinDeposit: Get<<Self::Currency as Currency<Self::AccountId>>::Balance>;

		/// Maximum supply for any coin
		#[pallet::constant]
		type MaxSupply: Get<u128>;
	}

	/// Storage for coin metadata
	#[pallet::storage]
	#[pallet::getter(fn coin_metadata)]
	pub type CoinMetadata<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CoinId,
		CoinInfo<
			BoundedVec<u8, T::MaxSymbolLength>,
			BoundedVec<u8, T::MaxNameLength>,
			T::AccountId,
			<T::Currency as Currency<T::AccountId>>::Balance,
            FeeConfig,
		>,
		OptionQuery,
	>;

	/// Storage for coin balances: CoinId -> AccountId -> Balance
	#[pallet::storage]
	#[pallet::getter(fn balances)]
	pub type Balances<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CoinId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	/// Storage for total supply of each coin
	#[pallet::storage]
	#[pallet::getter(fn total_supply)]
	pub type TotalSupply<T: Config> = StorageMap<_, Blake2_128Concat, CoinId, u128, ValueQuery>;

	/// Storage for coin symbol to ID mapping
	#[pallet::storage]
	#[pallet::getter(fn symbol_to_id)]
	pub type SymbolToId<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BoundedVec<u8, T::MaxSymbolLength>,
		CoinId,
		OptionQuery,
	>;

	/// Next available coin ID
	#[pallet::storage]
	#[pallet::getter(fn next_coin_id)]
	pub type NextCoinId<T: Config> = StorageValue<_, CoinId, ValueQuery>;

	/// Minting permissions: CoinId -> AccountId -> bool
	#[pallet::storage]
	#[pallet::getter(fn mint_permissions)]
	pub type MintPermissions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CoinId,
		Blake2_128Concat,
		T::AccountId,
		bool,
		ValueQuery,
	>;

    #[pallet::storage]
    #[pallet::getter(fn burn_permissions)]
    pub type BurnPermissions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CoinId,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

	/// Storage for user's preferred fee coin
	#[pallet::storage]
	#[pallet::getter(fn preferred_fee_coin)]
	pub type PreferredFeeCoin<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		CoinId,
		OptionQuery,
	>;

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new coin was created
		CoinCreated {
			coin_id: CoinId,
			symbol: Vec<u8>,
			name: Vec<u8>,
			creator: T::AccountId,
			initial_supply: u128,
		},
		/// Coins were transferred
		Transfer {
			coin_id: CoinId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
		/// Coins were minted
		Minted {
			coin_id: CoinId,
			to: T::AccountId,
			amount: u128,
		},
		/// Coins were burned
		Burned {
			coin_id: CoinId,
			from: T::AccountId,
			amount: u128,
		},
		/// Coin ownership was transferred
		OwnershipTransferred {
			coin_id: CoinId,
			old_owner: T::AccountId,
			new_owner: T::AccountId,
		},
		/// Mint permission was granted or revoked
		MintPermissionSet {
			coin_id: CoinId,
			account: T::AccountId,
			can_mint: bool,
		},
		/// Coin metadata was updated
		MetadataUpdated {
			coin_id: CoinId,
		},
        /// Burn permission was granted or revoked
        BurnPermissionSet {
            coin_id: CoinId,
            account: T::AccountId,
            can_burn: bool,
        },
        /// Fee configuration was updated
        FeeConfigUpdated {
            coin_id: CoinId,
            transfer_fee: u128,
            minimum_balance: u128,
			can_pay_tx_fees: bool, // Add this field
        },
		/// Preferred fee coin set for an account
		PreferredFeeCoinSet {
			account: T::AccountId,
			coin_id: Option<CoinId>,
		},
	}

	/// Errors that can occur when using this pallet
	#[pallet::error]
	pub enum Error<T> {
		/// The coin does not exist
		CoinNotFound,
		/// Insufficient balance for the operation
		InsufficientBalance,
		/// Arithmetic overflow occurred
		Overflow,
		/// The symbol is already in use
		SymbolAlreadyExists,
		/// Symbol is too long
		SymbolTooLong,
		/// Name is too long
		NameTooLong,
		/// Maximum number of coins reached
		TooManyCoins,
		/// Not authorized for this operation
		NotAuthorized,
		/// Cannot transfer to the same account
		TransferToSelf,
		/// Initial supply exceeds maximum allowed
		ExceedsMaxSupply,
		/// Amount is zero
		ZeroAmount,
		/// No minting permission
		NoMintPermission,
        /// No burning permission
        NoBurnPermission,
        /// Balance would fall below minimum required
        BelowMinimumBalance,
		/// Coin cannot be used to pay transaction fees
		CannotPayFees,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new coin with the specified metadata and initial supply
		///
		/// The origin must be signed and will become the owner of the coin.
		/// A deposit is required to create a coin.
		///
		/// # Arguments
		/// * `symbol` - The coin symbol (e.g., "BTC", "ETH")
		/// * `name` - The coin name (e.g., "Bitcoin", "Ethereum")
		/// * `decimals` - Number of decimal places
		/// * `initial_supply` - Initial supply of coins to mint to creator
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_coin())]
		pub fn create_coin(
			origin: OriginFor<T>,
			symbol: Vec<u8>,
			name: Vec<u8>,
			decimals: u8,
			initial_supply: u128,
            initial_minters: Option<Vec<T::AccountId>>,  // New: Optional additional minters
            initial_burners: Option<Vec<T::AccountId>>,  // New: Optional additional burners
			can_pay_tx_fees: bool, // New: Optional fee payment eligibility
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Validate inputs
			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			ensure!(initial_supply <= T::MaxSupply::get(), Error::<T>::ExceedsMaxSupply);
			ensure!(initial_supply > 0, Error::<T>::ZeroAmount);

			let bounded_symbol: BoundedVec<u8, T::MaxSymbolLength> = 
				symbol.clone().try_into().map_err(|_| Error::<T>::SymbolTooLong)?;
			let bounded_name: BoundedVec<u8, T::MaxNameLength> = 
				name.clone().try_into().map_err(|_| Error::<T>::NameTooLong)?;

			// Check if symbol already exists
			ensure!(
				!SymbolToId::<T>::contains_key(&bounded_symbol),
				Error::<T>::SymbolAlreadyExists
			);

			// Check maximum coins limit
			let coin_id = NextCoinId::<T>::get();
			ensure!(coin_id < T::MaxCoins::get(), Error::<T>::TooManyCoins);

			// Reserve deposit for coin creation
			let deposit_amount = T::CoinDeposit::get();
			T::Currency::reserve(&who, deposit_amount)
				.map_err(|_| Error::<T>::InsufficientBalance)?;

			// Create coin metadata with default fee config
            let coin_info = CoinInfo {
                symbol: bounded_symbol.clone(),
                name: bounded_name,
                decimals,
                owner: who.clone(),
                deposit: deposit_amount,
                fee_config: FeeConfig {
                    transfer_fee: 0, // Default: no fee
                    minimum_balance: 0, // Default: no minimum
                    can_pay_tx_fees, // Default: cannot pay tx fees (for Task 6)
                },
            };

			// Store coin information
			CoinMetadata::<T>::insert(&coin_id, &coin_info);
			SymbolToId::<T>::insert(&bounded_symbol, &coin_id);
			
			// Set initial supply and balance
			TotalSupply::<T>::insert(&coin_id, initial_supply);
			Balances::<T>::insert(&coin_id, &who, initial_supply);

			// Grant permissions to creator
            MintPermissions::<T>::insert(&coin_id, &who, true);
            BurnPermissions::<T>::insert(&coin_id, &who, true);  // New: Grant burn to creator

            // Grant additional initial minters
            if let Some(minters) = initial_minters {
                for minter in minters {
                    MintPermissions::<T>::insert(&coin_id, &minter, true);
                    Self::deposit_event(Event::MintPermissionSet {  // Reuse event
                        coin_id,
                        account: minter.clone(),
                        can_mint: true,
                    });
                }
            }

            // Grant additional initial burners
            if let Some(burners) = initial_burners {
                for burner in burners {
                    BurnPermissions::<T>::insert(&coin_id, &burner, true);
                    // Optionally add a new Event::BurnPermissionSet if you want separation
                    Self::deposit_event(Event::MintPermissionSet {  // Reuse for now, or add new event
                        coin_id,
                        account: burner.clone(),
                        can_mint: true,  // Adjust if adding separate event
                    });
                }
            }

			// Update next coin ID
			NextCoinId::<T>::put(coin_id + 1);

			// Emit event
			Self::deposit_event(Event::CoinCreated {
				coin_id,
				symbol,
				name,
				creator: who,
				initial_supply,
			});

			Ok(())
		}

		/// Transfer coins from one account to another
		///
		/// # Arguments
		/// * `coin_id` - The ID of the coin to transfer
		/// * `to` - The recipient account
		/// * `amount` - The amount to transfer
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			coin_id: CoinId,
			to: T::AccountId,
			amount: u128,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			// Validate inputs
			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			ensure!(amount > 0, Error::<T>::ZeroAmount);
            ensure!(from != to, Error::<T>::TransferToSelf);
            let coin_info = CoinMetadata::<T>::get(&coin_id)
                .ok_or(Error::<T>::CoinNotFound)?;

            // Calculate total amount to deduct (amount + fee)
            let transfer_fee = coin_info.fee_config.transfer_fee;
            let total_deduct = amount.checked_add(transfer_fee)
                .ok_or(Error::<T>::Overflow)?;

            // Check sender's balance
            let from_balance = Balances::<T>::get(&coin_id, &from);
            ensure!(from_balance >= total_deduct, Error::<T>::InsufficientBalance);

            // Check minimum balance requirement for sender after transfer
            let new_from_balance = from_balance.saturating_sub(total_deduct);
            ensure!(
                new_from_balance >= coin_info.fee_config.minimum_balance,
                Error::<T>::BelowMinimumBalance // New error
            );

            // Update recipient's balance
            let to_balance = Balances::<T>::get(&coin_id, &to);
            let new_to_balance = to_balance.checked_add(amount)
                .ok_or(Error::<T>::Overflow)?;

            // Apply transfer and fee (burn the fee for simplicity)
            Balances::<T>::insert(&coin_id, &from, new_from_balance);
            Balances::<T>::insert(&coin_id, &to, new_to_balance);
            if transfer_fee > 0 {
                let current_supply = TotalSupply::<T>::get(&coin_id);
                let new_supply = current_supply.saturating_sub(transfer_fee);
                TotalSupply::<T>::insert(&coin_id, new_supply);
                // Emit burn event for fee
                Self::deposit_event(Event::Burned {
                    coin_id,
                    from: from.clone(),
                    amount: transfer_fee,
                });
            }

			// Emit event
			Self::deposit_event(Event::Transfer {
				coin_id,
				from,
				to,
				amount,
			});

			Ok(())
		}

		/// Mint new coins to a specified account
		///
		/// Only accounts with minting permission can call this function.
		///
		/// # Arguments
		/// * `coin_id` - The ID of the coin to mint
		/// * `to` - The account to receive the minted coins
		/// * `amount` - The amount to mint
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			coin_id: CoinId,
			to: T::AccountId,
			amount: u128,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Validate inputs
			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			ensure!(amount > 0, Error::<T>::ZeroAmount);
			ensure!(CoinMetadata::<T>::contains_key(&coin_id), Error::<T>::CoinNotFound);

			// Check minting permission
			ensure!(
				MintPermissions::<T>::get(&coin_id, &who),
				Error::<T>::NoMintPermission
			);

			// Check if minting would exceed max supply
			let current_supply = TotalSupply::<T>::get(&coin_id);
			let new_supply = current_supply.checked_add(amount)
				.ok_or(Error::<T>::Overflow)?;
			ensure!(new_supply <= T::MaxSupply::get(), Error::<T>::ExceedsMaxSupply);

			// Update balance and total supply
			let current_balance = Balances::<T>::get(&coin_id, &to);
			let new_balance = current_balance.checked_add(amount)
				.ok_or(Error::<T>::Overflow)?;

			Balances::<T>::insert(&coin_id, &to, new_balance);
			TotalSupply::<T>::insert(&coin_id, new_supply);

			// Emit event
			Self::deposit_event(Event::Minted {
				coin_id,
				to,
				amount,
			});

			Ok(())
		}

		/// Burn coins from the caller's account
		///
		/// # Arguments
		/// * `coin_id` - The ID of the coin to burn
		/// * `amount` - The amount to burn
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::burn())]
		pub fn burn(
			origin: OriginFor<T>,
			coin_id: CoinId,
			amount: u128,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Validate inputs
			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			ensure!(amount > 0, Error::<T>::ZeroAmount);
			ensure!(CoinMetadata::<T>::contains_key(&coin_id), Error::<T>::CoinNotFound);

            // Check burning permission
            ensure!(
                BurnPermissions::<T>::get(&coin_id, &who),
                Error::<T>::NoBurnPermission  // Add this to Error enum
            );

			// Check balance
			let current_balance = Balances::<T>::get(&coin_id, &who);
			ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

			// Update balance and total supply
			let new_balance = current_balance.saturating_sub(amount);
			let current_supply = TotalSupply::<T>::get(&coin_id);
			let new_supply = current_supply.saturating_sub(amount);

			Balances::<T>::insert(&coin_id, &who, new_balance);
			TotalSupply::<T>::insert(&coin_id, new_supply);

			// Emit event
			Self::deposit_event(Event::Burned {
				coin_id,
				from: who,
				amount,
			});

			Ok(())
		}

		/// Transfer ownership of a coin to another account
		///
		/// Only the current owner can call this function.
		///
		/// # Arguments
		/// * `coin_id` - The ID of the coin
		/// * `new_owner` - The new owner account
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::transfer_ownership())]
		pub fn transfer_ownership(
			origin: OriginFor<T>,
			coin_id: CoinId,
			new_owner: T::AccountId,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			// Get coin metadata
			let mut coin_info = CoinMetadata::<T>::get(&coin_id)
				.ok_or(Error::<T>::CoinNotFound)?;

			// Check authorization
			ensure!(coin_info.owner == who, Error::<T>::NotAuthorized);

			let old_owner = coin_info.owner.clone();
			
			// Update owner
			coin_info.owner = new_owner.clone();
			CoinMetadata::<T>::insert(&coin_id, &coin_info);

			// Transfer minting permission from old to new owner
			MintPermissions::<T>::remove(&coin_id, &old_owner);
			MintPermissions::<T>::insert(&coin_id, &new_owner, true);

			// Emit event
			Self::deposit_event(Event::OwnershipTransferred {
				coin_id,
				old_owner,
				new_owner,
			});

			Ok(())
		}

		/// Set minting permission for an account
		///
		/// Only the coin owner can grant or revoke minting permissions.
		///
		/// # Arguments
		/// * `coin_id` - The ID of the coin
		/// * `account` - The account to set permission for
		/// * `can_mint` - Whether the account can mint
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::set_mint_permission())]
		pub fn set_mint_permission(
			origin: OriginFor<T>,
			coin_id: CoinId,
			account: T::AccountId,
			can_mint: bool,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			// Get coin metadata
			let coin_info = CoinMetadata::<T>::get(&coin_id)
				.ok_or(Error::<T>::CoinNotFound)?;

			// Check authorization
			ensure!(coin_info.owner == who, Error::<T>::NotAuthorized);

			// Set permission
			if can_mint {
				MintPermissions::<T>::insert(&coin_id, &account, true);
			} else {
				MintPermissions::<T>::remove(&coin_id, &account);
			}

			// Emit event
			Self::deposit_event(Event::MintPermissionSet {
				coin_id,
				account,
				can_mint,
			});

			Ok(())
		}

        #[pallet::call_index(6)]  // Adjust index as needed
        #[pallet::weight(T::WeightInfo::set_mint_permission())]  // Reuse weight or add new
        pub fn set_burn_permission(
            origin: OriginFor<T>,
            coin_id: CoinId,
            account: T::AccountId,
            can_burn: bool,
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

            // Get coin metadata
            let coin_info = CoinMetadata::<T>::get(&coin_id)
                .ok_or(Error::<T>::CoinNotFound)?;

            // Check authorization
            ensure!(coin_info.owner == who, Error::<T>::NotAuthorized);

            // Set permission
            if can_burn {
                BurnPermissions::<T>::insert(&coin_id, &account, true);
            } else {
                BurnPermissions::<T>::remove(&coin_id, &account);
            }

            // Emit event (add new Event::BurnPermissionSet { coin_id, account, can_burn })
            Self::deposit_event(Event::BurnPermissionSet {  // Add this to Event enum
                coin_id,
                account,
                can_burn,
            });

            Ok(())
        }

        #[pallet::call_index(7)] // Adjust index (e.g., after set_burn_permission)
        #[pallet::weight(T::WeightInfo::set_fee_config())] // Add new weight in weights.rs
        pub fn set_fee_config(
            origin: OriginFor<T>,
            coin_id: CoinId,
            transfer_fee: u128,
            minimum_balance: u128,
			can_pay_tx_fees: bool, // New: Add this parameter
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

            // Get coin metadata
            let mut coin_info = CoinMetadata::<T>::get(&coin_id)
                .ok_or(Error::<T>::CoinNotFound)?;

            // Check authorization
            ensure!(coin_info.owner == who, Error::<T>::NotAuthorized);

            // Update fee config
            coin_info.fee_config.transfer_fee = transfer_fee;
            coin_info.fee_config.minimum_balance = minimum_balance;
			coin_info.fee_config.can_pay_tx_fees = can_pay_tx_fees;
            CoinMetadata::<T>::insert(&coin_id, &coin_info);

            // Emit event
            Self::deposit_event(Event::FeeConfigUpdated {
                coin_id,
                transfer_fee,
                minimum_balance,
				can_pay_tx_fees, // Update event
            });

            Ok(())
        }

		#[pallet::call_index(8)] // Adjust index as needed
		#[pallet::weight(T::WeightInfo::set_preferred_fee_coin())] // Add new weight
		pub fn set_preferred_fee_coin(
			origin: OriginFor<T>,
			coin_id: Option<CoinId>, // None to use Balances pallet
			tx_fee_coin: Option<CoinId>, // Fee coin for THIS transaction
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(coin_id) = coin_id {
				let coin_info = CoinMetadata::<T>::get(&coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			// Set or clear preferred coin
			if let Some(coin_id) = coin_id {
				PreferredFeeCoin::<T>::insert(&who, coin_id);
			} else {
				PreferredFeeCoin::<T>::remove(&who);
			}

			// Emit event
			Self::deposit_event(Event::PreferredFeeCoinSet {
				account: who,
				coin_id,
			});

			Ok(())
		}

		/// Execute any pallet call with a specific fee coin
		/// This is a wrapper that temporarily sets the fee coin for one transaction
		#[pallet::call_index(9)] // Adjust index as needed
		#[pallet::weight(T::WeightInfo::set_preferred_fee_coin())] // Add proper weight calculation
		pub fn call_with_fee_coin(
			origin: OriginFor<T>,
			tx_fee_coin: Option<CoinId>,
			call: Box<T::RuntimeCall>,
		) -> DispatchResult {
			let _who = ensure_signed(origin.clone())?;

			// If coin_id is provided, ensure it exists and can pay fees
			if let Some(tx_coin_id) = tx_fee_coin {
				let coin_info = CoinMetadata::<T>::get(&tx_coin_id)
					.ok_or(Error::<T>::CoinNotFound)?;
				ensure!(coin_info.fee_config.can_pay_tx_fees, Error::<T>::CannotPayFees);
			}

			// Execute the call
			let result = call.dispatch(origin);

			result.map(|_| ()).map_err(|e| e.error)
		}
		
	}
}

// Helper functions implementation
impl<T: Config> Pallet<T> {
	/// Get the balance of an account for a specific coin
	pub fn balance_of(account: &T::AccountId, coin_id: CoinId) -> u128 {
		Balances::<T>::get(coin_id, account)
	}

	/// Get the total supply of a coin
	pub fn total_supply_of(coin_id: CoinId) -> u128 {
		TotalSupply::<T>::get(coin_id)
	}

	/// Get coin metadata
	pub fn get_coin_metadata(
		coin_id: CoinId,
	) -> Option<CoinInfo<
		BoundedVec<u8, T::MaxSymbolLength>, 
		BoundedVec<u8, T::MaxNameLength>, 
		T::AccountId,
		<T::Currency as frame_support::traits::Currency<T::AccountId>>::Balance,
        FeeConfig,
	>> {
		CoinMetadata::<T>::get(coin_id)
	}

	/// Get coin ID by symbol
	pub fn get_coin_id_by_symbol(symbol: &[u8]) -> Option<CoinId> {
		let bounded_symbol: BoundedVec<u8, T::MaxSymbolLength> = symbol.to_vec()
			.try_into()
			.ok()?;
		SymbolToId::<T>::get(bounded_symbol)
	}

	/// Check if an account has minting permission for a coin
	pub fn has_mint_permission(coin_id: CoinId, account: &T::AccountId) -> bool {
		MintPermissions::<T>::get(coin_id, account)
	}

    /// Check if an account has burning permission for a coin
    pub fn has_burn_permission(coin_id: CoinId, account: &T::AccountId) -> bool {
        BurnPermissions::<T>::get(coin_id, account)
    }

}
