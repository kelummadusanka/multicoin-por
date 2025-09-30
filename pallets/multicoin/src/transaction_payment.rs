use frame_support::{ensure, traits::Currency};
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    transaction_validity::{InvalidTransaction, TransactionValidityError},
};
use crate::{Balances, CoinMetadata, Config, CoinId, PreferredFeeCoin};

// Custom OnChargeTransaction implementation for multi-coin fee payments
pub struct MultiCoinFeeAdapter<T: Config + pallet_transaction_payment::Config> {
    _phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config + pallet_transaction_payment::Config> OnChargeTransaction<T> for MultiCoinFeeAdapter<T>
where
    <T::Currency as Currency<T::AccountId>>::Balance: Into<u128>,
{
    type Balance = <T::Currency as Currency<T::AccountId>>::Balance;
    type LiquidityInfo = Option<(T::AccountId, Option<CoinId>, u128)>; // Stores (who, coin_id, fee)

    fn can_withdraw_fee(
        who: &T::AccountId,
        call: &T::RuntimeCall,
        _info: &sp_runtime::traits::DispatchInfoOf<T::RuntimeCall>,
        fee: Self::Balance,
        _tip: Self::Balance,
    ) -> Result<(), TransactionValidityError> {
        /*if fee.is_zero() {
            return Ok(());
        }*/

        // Determine which coin to use for fee payment
        let selected_coin = Self::determine_fee_coin(call, who);
        let fee_u128: u128 = fee.saturated_into();

        if let Some(coin_id) = selected_coin {
            // Validate the selected coin can pay fees
            let coin_info = CoinMetadata::<T>::get(&coin_id).ok_or(InvalidTransaction::Payment)?;
            ensure!(coin_info.fee_config.can_pay_tx_fees, InvalidTransaction::Payment);

            // Check balance
            let current_balance = Balances::<T>::get(&coin_id, who);
            ensure!(current_balance >= fee_u128, InvalidTransaction::Payment);
            Ok(())
        } else {
            // Fall back to native currency (Balances pallet)
            let balance = T::Currency::free_balance(who);
            ensure!(balance >= fee, InvalidTransaction::Payment);
            Ok(())
        }
    }

    fn withdraw_fee(
        who: &T::AccountId,
        call: &T::RuntimeCall,
        _dispatch_info: &sp_runtime::traits::DispatchInfoOf<T::RuntimeCall>,
        fee: Self::Balance,
        _tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        /*if fee.is_zero() {
            return Ok(None);
        }*/

        // Determine which coin to use for fee payment
        let selected_coin = Self::determine_fee_coin(call, who);
        let fee_u128: u128 = fee.saturated_into();

        if let Some(coin_id) = selected_coin {
            let coin_info = CoinMetadata::<T>::get(&coin_id).ok_or(InvalidTransaction::Payment)?;
            ensure!(coin_info.fee_config.can_pay_tx_fees, InvalidTransaction::Payment);

            let current_balance = Balances::<T>::get(&coin_id, who);
            ensure!(current_balance >= fee_u128, InvalidTransaction::Payment);

            // Deduct fee from multicoin balance
            let new_balance = current_balance
                .checked_sub(fee_u128)
                .ok_or(InvalidTransaction::Payment)?;
            Balances::<T>::insert(&coin_id, who, new_balance);

            // Burn the fee (reduce total supply)
            let current_supply = crate::TotalSupply::<T>::get(&coin_id);
            let new_supply = current_supply.saturating_sub(fee_u128);
            crate::TotalSupply::<T>::insert(&coin_id, new_supply);

            Ok(Some((who.clone(), Some(coin_id), fee_u128)))
        } else {
            // Use native currency (Balances pallet)
            let balance = T::Currency::free_balance(who);
            ensure!(balance >= fee, InvalidTransaction::Payment);
            
            T::Currency::withdraw(
                who,
                fee,
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )
            .map_err(|_| InvalidTransaction::Payment)?;
            
            Ok(Some((who.clone(), None, fee_u128)))
        }
    }

    fn correct_and_deposit_fee(
        _who: &T::AccountId,
        _dispatch_info: &sp_runtime::traits::DispatchInfoOf<T::RuntimeCall>,
        _post_info: &sp_runtime::traits::PostDispatchInfoOf<T::RuntimeCall>,
        corrected_fee: Self::Balance,
        _tip: Self::Balance,
        liquidity_info: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some((who, coin_id, paid_fee)) = liquidity_info {
            let corrected_fee_u128: u128 = corrected_fee.saturated_into();
            if corrected_fee_u128 < paid_fee {
                let refund = paid_fee - corrected_fee_u128;
                if let Some(coin_id) = coin_id {
                    // Refund to multicoin balance
                    let current_balance = Balances::<T>::get(&coin_id, &who);
                    let new_balance = current_balance
                        .checked_add(refund)
                        .ok_or(InvalidTransaction::Payment)?;
                    Balances::<T>::insert(&coin_id, &who, new_balance);

                    // Increase total supply (un-burn the excess fee)
                    let current_supply = crate::TotalSupply::<T>::get(&coin_id);
                    let new_supply = current_supply
                        .checked_add(refund)
                        .ok_or(InvalidTransaction::Payment)?;
                    crate::TotalSupply::<T>::insert(&coin_id, new_supply);
                } else {
                    // Refund to native currency
                    let refund_balance: <T::Currency as Currency<T::AccountId>>::Balance = 
                        refund.try_into().map_err(|_| InvalidTransaction::Payment)?;
                    T::Currency::deposit_creating(&who, refund_balance);
                }
            }
        }
        Ok(())
    }
}

impl<T: Config + pallet_transaction_payment::Config> MultiCoinFeeAdapter<T>
where
    <T::Currency as Currency<T::AccountId>>::Balance: Into<u128>,
{
    /// Determine which coin to use for fee payment by extracting from RuntimeCall
    fn determine_fee_coin(call: &T::RuntimeCall, who: &T::AccountId) -> Option<CoinId> {
        // CRITICAL: Pattern matching RuntimeCall from within a pallet is not directly possible
        // because RuntimeCall is generated by construct_runtime! and is opaque to individual pallets.
        
        // WORKAROUND: Use SCALE encoding/decoding to inspect the call
        use codec::{Decode, Encode};
        
        // Encode the call to bytes
        let encoded = call.encode();
        
        // The encoded format is: [pallet_index, call_index, ...params]
        // We need to check if this is one of our multi-coin calls
        
        if encoded.len() >= 2 {
            // Check pallet and call indices (these depend on your runtime configuration)
            // You'll need to find the correct indices for your pallet in the runtime
            
            // Attempt to decode as our pallet calls
            // This is fragile and runtime-specific!
            
            // For transfer_with_immediate_fee_coin (call_index = 13)
            if let Ok(call_data) = <crate::Call<T> as Decode>::decode(&mut &encoded[1..]) {
                match call_data {
                    crate::Call::create_coin { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::mint { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::burn { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::transfer { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::set_fee_config { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::transfer_ownership { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::set_burn_permission { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::set_mint_permission { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::set_preferred_fee_coin { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    crate::Call::call_with_fee_coin { tx_fee_coin, .. } => {
                        if tx_fee_coin.is_some() {
                            return tx_fee_coin;
                        }
                    },
                    _ => {}
                }
            }
        }
        
        // Fall back to session fee coin
        /*if let Some(session_coin) = crate::SessionFeeCoin::<T>::get(who) {
            return Some(session_coin);
        }*/
        
        // Fall back to preferred fee coin
        PreferredFeeCoin::<T>::get(who)
    }
}

/// Helper trait for calls that specify a fee coin
pub trait CallWithFeeCoin<T: Config> {
    fn fee_coin(&self) -> Option<CoinId>;
}

// You would implement this for your runtime calls that support fee coin selection
// This is a placeholder - actual implementation depends on your call structure

