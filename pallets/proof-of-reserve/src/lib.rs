#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::traits::{Currency, Get, tokens::fungible};
    use sp_runtime::traits::Dispatchable;
    use sp_std::vec::Vec;
    use sp_runtime::traits::{Zero, Saturating, Hash, AccountIdConversion};
    use frame_support::PalletId;
    use scale_info::prelude::boxed::Box;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;

        /// Pallet ID for the custody account
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type MaxTxIdLength: Get<u32>;

        #[pallet::constant]
        type MaxWalletLength: Get<u32>;

        #[pallet::constant]
        type MaxCoinNameLength: Get<u32>;

        type WeightInfo: WeightInfo;
    }

    /// Status of a deposit request
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum DepositStatus {
        Pending,
        Approved,
        Rejected,
    }

    /// Status of a withdrawal request
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum WithdrawalStatus {
        Pending,
        Completed,
        Rejected,
    }

    /// Deposit request submitted by user
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct DepositRequest<T: Config> {
        /// Account that submitted the request
        pub submitter: T::AccountId,
        /// Onchain account that will receive the native tokens
        pub recipient: T::AccountId,
        /// External transaction ID (BTC tx hash)
        pub external_tx_id: BoundedVec<u8, T::MaxTxIdLength>,
        /// User's external wallet (BTC address they sent from)
        pub external_wallet: BoundedVec<u8, T::MaxWalletLength>,
        /// Coin type (e.g., "BTC")
        pub coin_name: BoundedVec<u8, T::MaxCoinNameLength>,
        /// Amount deposited in external coin (in smallest unit)
        pub external_amount: u128,
        /// Exchange ratio (external coin value / native coin value)
        pub ratio: u128,
        /// Calculated native coins to mint
        pub native_amount: BalanceOf<T>,
        /// Current status
        pub status: DepositStatus,
        /// Block number when submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Validator who approved (if any)
        pub approved_by: Option<T::AccountId>,
    }

    /// Withdrawal request submitted by user
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct WithdrawalRequest<T: Config> {
        /// User requesting withdrawal
        pub user: T::AccountId,
        /// User's external wallet to receive funds
        pub external_wallet: BoundedVec<u8, T::MaxWalletLength>,
        /// Coin type
        pub coin_name: BoundedVec<u8, T::MaxCoinNameLength>,
        /// Amount of native coins to burn
        pub native_amount: BalanceOf<T>,
        /// Calculated external coin amount to release
        pub external_amount: u128,
        /// Exchange ratio at time of request
        pub ratio: u128,
        /// Current status
        pub status: WithdrawalStatus,
        /// Block number when submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Processor who completed withdrawal
        pub processed_by: Option<T::AccountId>,
    }

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// User submitted deposit request
        DepositRequested {
            request_id: T::Hash,
            submitter: T::AccountId,
            recipient: T::AccountId,
            external_tx_id: BoundedVec<u8, T::MaxTxIdLength>,
            external_wallet: BoundedVec<u8, T::MaxWalletLength>,
            coin_name: BoundedVec<u8, T::MaxCoinNameLength>,
            external_amount: u128,
            native_amount: BalanceOf<T>,
        },
        /// Validator approved deposit and minted coins
        DepositApproved {
            request_id: T::Hash,
            user: T::AccountId,
            validator: T::AccountId,
            native_amount: BalanceOf<T>,
        },
        /// Deposit request rejected
        DepositRejected {
            request_id: T::Hash,
            validator: T::AccountId,
        },
        /// User requested withdrawal
        WithdrawalRequested {
            request_id: T::Hash,
            user: T::AccountId,
            external_wallet: BoundedVec<u8, T::MaxWalletLength>,
            native_amount: BalanceOf<T>,
            external_amount: u128,
        },
        /// Withdrawal processed and coins burned
        WithdrawalCompleted {
            request_id: T::Hash,
            user: T::AccountId,
            processor: T::AccountId,
            native_amount: BalanceOf<T>,
        },
        /// Withdrawal rejected
        WithdrawalRejected {
            request_id: T::Hash,
            processor: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Request not found
        RequestNotFound,
        /// Only validators can approve/reject
        NotAuthorized,
        /// Request already processed
        AlreadyProcessed,
        /// Invalid amount
        InvalidAmount,
        /// Insufficient balance for withdrawal
        InsufficientBalance,
        /// Invalid ratio
        InvalidRatio,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Data too long
        DataTooLong,
        /// User cannot approve own request
        CannotApproveOwnRequest,
    }

    /// Pending deposit requests
    #[pallet::storage]
    pub type DepositRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        DepositRequest<T>
    >;

    /// Pending withdrawal requests
    #[pallet::storage]
    pub type WithdrawalRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        WithdrawalRequest<T>
    >;

    /// User's deposit request IDs
    #[pallet::storage]
    pub type UserDeposits<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<T::Hash, ConstU32<100>>,
        ValueQuery
    >;

    /// User's withdrawal request IDs
    #[pallet::storage]
    pub type UserWithdrawals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<T::Hash, ConstU32<100>>,
        ValueQuery
    >;

    /// Total minted native coins from all deposits
    #[pallet::storage]
    #[pallet::getter(fn total_minted)]
    pub type TotalMinted<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Total burned native coins from all withdrawals
    #[pallet::storage]
    #[pallet::getter(fn total_burned)]
    pub type TotalBurned<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Request counter for unique IDs
    #[pallet::storage]
    pub type RequestCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// User submits deposit request after sending coins to custody wallet
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::request_deposit())]
        pub fn request_deposit(
            origin: OriginFor<T>,
            onchain_account: T::AccountId,
            external_tx_id: Vec<u8>,
            external_wallet: Vec<u8>,
            coin_name: Vec<u8>,
            external_amount: u128,
            ratio: u128,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            // Validate inputs
            ensure!(external_amount > 0, Error::<T>::InvalidAmount);
            ensure!(ratio > 0, Error::<T>::InvalidRatio);

            // Create bounded vectors
            let bounded_tx_id: BoundedVec<u8, T::MaxTxIdLength> = external_tx_id.try_into()
                .map_err(|_| Error::<T>::DataTooLong)?;
            let bounded_wallet: BoundedVec<u8, T::MaxWalletLength> = external_wallet.try_into()
                .map_err(|_| Error::<T>::DataTooLong)?;
            let bounded_coin_name: BoundedVec<u8, T::MaxCoinNameLength> = coin_name.try_into()
                .map_err(|_| Error::<T>::DataTooLong)?;

            // Calculate native amount: external_amount * ratio
            let native_amount_u128 = external_amount.checked_mul(ratio)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let native_amount: BalanceOf<T> = native_amount_u128.try_into()
                .map_err(|_| Error::<T>::ArithmeticOverflow)?;

            // Generate request ID
            let counter = RequestCounter::<T>::get();
            RequestCounter::<T>::put(counter.saturating_add(1));
            let current_block = <frame_system::Pallet<T>>::block_number();
            let request_id = <T::Hashing as Hash>::hash_of(&(&submitter, counter, current_block));

            // Create deposit request
            let request = DepositRequest {
                submitter: submitter.clone(),
                recipient: onchain_account.clone(),
                external_tx_id: bounded_tx_id.clone(),
                external_wallet: bounded_wallet.clone(),
                coin_name: bounded_coin_name.clone(),
                external_amount,
                ratio,
                native_amount,
                status: DepositStatus::Pending,
                submitted_at: current_block,
                approved_by: None,
            };

            // Store request
            DepositRequests::<T>::insert(&request_id, &request);

            // Track user's deposits (track by recipient account)
            UserDeposits::<T>::try_mutate(&onchain_account, |deposits| {
                deposits.try_push(request_id)
                    .map_err(|_| Error::<T>::ArithmeticOverflow)
            })?;

            Self::deposit_event(Event::DepositRequested {
                request_id,
                submitter,
                recipient: onchain_account,
                external_tx_id: bounded_tx_id,
                external_wallet: bounded_wallet,
                coin_name: bounded_coin_name,
                external_amount,
                native_amount,
            });

            Ok(())
        }

        /// Validator approves deposit and transfers tokens from custody
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::approve_deposit())]
        pub fn approve_deposit(
            origin: OriginFor<T>,
            request_id: T::Hash,
        ) -> DispatchResult {
            let validator = ensure_signed(origin)?;

            let mut request = DepositRequests::<T>::get(&request_id)
                .ok_or(Error::<T>::RequestNotFound)?;

            ensure!(request.status == DepositStatus::Pending, Error::<T>::AlreadyProcessed);
            ensure!(request.submitter != validator, Error::<T>::CannotApproveOwnRequest);

            // Get custody account
            let custody_account = Self::account_id();

            // Transfer tokens from custody to recipient
            T::Currency::transfer(
                &custody_account,
                &request.recipient,
                request.native_amount,
                frame_support::traits::ExistenceRequirement::AllowDeath,
            )?;

            // Update request status
            request.status = DepositStatus::Approved;
            request.approved_by = Some(validator.clone());
            DepositRequests::<T>::insert(&request_id, &request);

            // Update total minted
            TotalMinted::<T>::mutate(|total| *total = total.saturating_add(request.native_amount));

            Self::deposit_event(Event::DepositApproved {
                request_id,
                user: request.recipient.clone(),
                validator,
                native_amount: request.native_amount,
            });

            Ok(())
        }

        /// Validator rejects deposit request
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::reject_deposit())]
        pub fn reject_deposit(
            origin: OriginFor<T>,
            request_id: T::Hash,
        ) -> DispatchResult {
            let validator = ensure_signed(origin)?;

            let mut request = DepositRequests::<T>::get(&request_id)
                .ok_or(Error::<T>::RequestNotFound)?;

            ensure!(request.status == DepositStatus::Pending, Error::<T>::AlreadyProcessed);

            // Update status
            request.status = DepositStatus::Rejected;
            DepositRequests::<T>::insert(&request_id, &request);

            Self::deposit_event(Event::DepositRejected {
                request_id,
                validator,
            });

            Ok(())
        }

        /// User requests withdrawal
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::request_withdrawal())]
        pub fn request_withdrawal(
            origin: OriginFor<T>,
            external_wallet: Vec<u8>,
            coin_name: Vec<u8>,
            native_amount: BalanceOf<T>,
            ratio: u128,
        ) -> DispatchResult {
            let user = ensure_signed(origin)?;

            // Validate inputs
            ensure!(!native_amount.is_zero(), Error::<T>::InvalidAmount);
            ensure!(ratio > 0, Error::<T>::InvalidRatio);

            // Check user has sufficient balance
            let user_balance = T::Currency::free_balance(&user);
            ensure!(user_balance >= native_amount, Error::<T>::InsufficientBalance);

            // Create bounded vectors
            let bounded_wallet: BoundedVec<u8, T::MaxWalletLength> = external_wallet.try_into()
                .map_err(|_| Error::<T>::DataTooLong)?;
            let bounded_coin_name: BoundedVec<u8, T::MaxCoinNameLength> = coin_name.try_into()
                .map_err(|_| Error::<T>::DataTooLong)?;

            // Calculate external amount: native_amount / ratio
            let native_amount_u128: u128 = native_amount.try_into()
                .map_err(|_| Error::<T>::ArithmeticOverflow)?;
            let external_amount = native_amount_u128.checked_div(ratio)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Generate request ID
            let counter = RequestCounter::<T>::get();
            RequestCounter::<T>::put(counter.saturating_add(1));
            let current_block = <frame_system::Pallet<T>>::block_number();
            let request_id = <T::Hashing as Hash>::hash_of(&(&user, counter, current_block));

            // Create withdrawal request
            let request = WithdrawalRequest {
                user: user.clone(),
                external_wallet: bounded_wallet.clone(),
                coin_name: bounded_coin_name.clone(),
                native_amount,
                external_amount,
                ratio,
                status: WithdrawalStatus::Pending,
                submitted_at: current_block,
                processed_by: None,
            };

            // Store request
            WithdrawalRequests::<T>::insert(&request_id, &request);

            // Track user's withdrawals
            UserWithdrawals::<T>::try_mutate(&user, |withdrawals| {
                withdrawals.try_push(request_id)
                    .map_err(|_| Error::<T>::ArithmeticOverflow)
            })?;

            Self::deposit_event(Event::WithdrawalRequested {
                request_id,
                user,
                external_wallet: bounded_wallet,
                native_amount,
                external_amount,
            });

            Ok(())
        }

        /// Processor completes withdrawal after sending external coins
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::complete_withdrawal())]
        pub fn complete_withdrawal(
            origin: OriginFor<T>,
            request_id: T::Hash,
        ) -> DispatchResult {
            let processor = ensure_signed(origin)?;

            let mut request = WithdrawalRequests::<T>::get(&request_id)
                .ok_or(Error::<T>::RequestNotFound)?;

            ensure!(request.status == WithdrawalStatus::Pending, Error::<T>::AlreadyProcessed);

            // Burn tokens from user's account
            // This decreases total issuance
            let _negative_imbalance = T::Currency::withdraw(
                &request.user,
                request.native_amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Update request status
            request.status = WithdrawalStatus::Completed;
            request.processed_by = Some(processor.clone());
            WithdrawalRequests::<T>::insert(&request_id, &request);

            // Update total burned
            TotalBurned::<T>::mutate(|total| *total = total.saturating_add(request.native_amount));

            Self::deposit_event(Event::WithdrawalCompleted {
                request_id,
                user: request.user.clone(),
                processor,
                native_amount: request.native_amount,
            });

            Ok(())
        }

        /// Processor rejects withdrawal request
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reject_withdrawal())]
        pub fn reject_withdrawal(
            origin: OriginFor<T>,
            request_id: T::Hash,
        ) -> DispatchResult {
            let processor = ensure_signed(origin)?;

            let mut request = WithdrawalRequests::<T>::get(&request_id)
                .ok_or(Error::<T>::RequestNotFound)?;

            ensure!(request.status == WithdrawalStatus::Pending, Error::<T>::AlreadyProcessed);

            // Update status
            request.status = WithdrawalStatus::Rejected;
            WithdrawalRequests::<T>::insert(&request_id, &request);

            Self::deposit_event(Event::WithdrawalRejected {
                request_id,
                processor,
            });

            Ok(())
        }
        /// Execute any pallet call with a specific fee coin
		/// This is a wrapper that temporarily sets the fee coin for one transaction
		#[pallet::call_index(6)] // Adjust index as needed
		#[pallet::weight(T::WeightInfo::call_multicoin())]
		pub fn call_multicoin(
			origin: OriginFor<T>,
			call: Box<T::RuntimeCall>,
		) -> DispatchResult {
			let _who = ensure_signed(origin.clone())?;
			// Execute the call
			let result = call.dispatch(origin);
			result.map(|_| ()).map_err(|e| e.error)
		}

    }

    impl<T: Config> Pallet<T> {
        /// Get the pallet's custody account ID
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Get net supply (minted - burned)
        pub fn net_supply() -> BalanceOf<T> {
            let minted = Self::total_minted();
            let burned = Self::total_burned();
            minted.saturating_sub(burned)
        }
    }

    pub trait WeightInfo {
        fn request_deposit() -> Weight;
        fn approve_deposit() -> Weight;
        fn reject_deposit() -> Weight;
        fn request_withdrawal() -> Weight;
        fn complete_withdrawal() -> Weight;
        fn reject_withdrawal() -> Weight;
        fn call_multicoin() -> Weight;
    }

    impl WeightInfo for () {
        fn request_deposit() -> Weight {
            Weight::from_parts(50_000_000, 0)
        }
        fn approve_deposit() -> Weight {
            Weight::from_parts(60_000_000, 0)
        }
        fn reject_deposit() -> Weight {
            Weight::from_parts(30_000_000, 0)
        }
        fn request_withdrawal() -> Weight {
            Weight::from_parts(50_000_000, 0)
        }
        fn complete_withdrawal() -> Weight {
            Weight::from_parts(60_000_000, 0)
        }
        fn reject_withdrawal() -> Weight {
            Weight::from_parts(30_000_000, 0)
        }

        fn call_multicoin() -> Weight {
            Weight::from_parts(30_000_000, 0)
        }
    }
}