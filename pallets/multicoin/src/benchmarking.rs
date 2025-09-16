//! Benchmarking setup for pallet-multi-coin

use super::*;

#[allow(unused)]
use crate::Pallet as MultiCoin;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use frame_support::traits::Currency;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_coin() {
		let caller: T::AccountId = whitelisted_caller();
		
		// Fund the caller with enough balance for deposit
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		let symbol = b"BENCH".to_vec();
		let name = b"Benchmark Token".to_vec();
		let decimals = 18u8;
		let initial_supply = 1_000_000u128;

		#[extrinsic_call]
		create_coin(
			RawOrigin::Signed(caller.clone()),
			symbol.clone(),
			name.clone(),
			decimals,
			initial_supply
		);

		// Verify the coin was created
		assert_eq!(NextCoinId::<T>::get(), 1);
		assert!(CoinMetadata::<T>::contains_key(0));
		assert_eq!(TotalSupply::<T>::get(0), initial_supply);
		assert_eq!(Balances::<T>::get(0, &caller), initial_supply);
	}

	#[benchmark]
	fn transfer() {
		let caller: T::AccountId = whitelisted_caller();
		let recipient: T::AccountId = account("recipient", 0, 0);
		
		// Setup: create a coin with a transfer fee
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"TEST".to_vec(),
			b"Test Token".to_vec(),
			18,
			1_000_000,
			None,
			None,
		));
	
		let coin_id = 0u32;
		assert_ok!(MultiCoin::<T>::set_fee_config(
			RawOrigin::Signed(caller.clone()).into(),
			coin_id,
			100, // Transfer fee
			50,  // Minimum balance
		));
	
		let amount = 500_000u128;
	
		#[extrinsic_call]
		transfer(RawOrigin::Signed(caller.clone()), coin_id, recipient.clone(), amount);
	
		// Verify the transfer
		assert_eq!(Balances::<T>::get(coin_id, &caller), 500_000 - 100); // Amount - fee
		assert_eq!(Balances::<T>::get(coin_id, &recipient), 500_000);
		assert_eq!(TotalSupply::<T>::get(coin_id), 1_000_000 - 100); // Fee burned
	}

	#[benchmark]
	fn mint() {
		let caller: T::AccountId = whitelisted_caller();
		let recipient: T::AccountId = account("recipient", 0, 0);
		
		// Setup: create a coin first
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"MINT".to_vec(),
			b"Mint Token".to_vec(),
			18,
			1_000_000,
		));

		let coin_id = 0u32;
		let amount = 500_000u128;

		#[extrinsic_call]
		mint(RawOrigin::Signed(caller), coin_id, recipient.clone(), amount);

		// Verify the mint
		assert_eq!(Balances::<T>::get(coin_id, &recipient), amount);
		assert_eq!(TotalSupply::<T>::get(coin_id), 1_500_000);
	}

	#[benchmark]
	fn burn() {
		let caller: T::AccountId = whitelisted_caller();
		
		// Setup: create a coin first
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"BURN".to_vec(),
			b"Burn Token".to_vec(),
			18,
			1_000_000,
		));

		let coin_id = 0u32;
		let amount = 300_000u128;

		#[extrinsic_call]
		burn(RawOrigin::Signed(caller.clone()), coin_id, amount);

		// Verify the burn
		assert_eq!(Balances::<T>::get(coin_id, &caller), 700_000);
		assert_eq!(TotalSupply::<T>::get(coin_id), 700_000);
	}

	#[benchmark]
	fn transfer_ownership() {
		let caller: T::AccountId = whitelisted_caller();
		let new_owner: T::AccountId = account("new_owner", 0, 0);
		
		// Setup: create a coin first
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"OWN".to_vec(),
			b"Ownership Token".to_vec(),
			18,
			1_000_000,
		));

		let coin_id = 0u32;

		#[extrinsic_call]
		transfer_ownership(RawOrigin::Signed(caller.clone()), coin_id, new_owner.clone());

		// Verify ownership transfer
		let coin_info = CoinMetadata::<T>::get(coin_id).unwrap();
		assert_eq!(coin_info.owner, new_owner);
		assert_eq!(MintPermissions::<T>::get(coin_id, &caller), false);
		assert_eq!(MintPermissions::<T>::get(coin_id, &new_owner), true);
	}

	#[benchmark]
	fn set_mint_permission() {
		let caller: T::AccountId = whitelisted_caller();
		let grantee: T::AccountId = account("grantee", 0, 0);
		
		// Setup: create a coin first
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"PERM".to_vec(),
			b"Permission Token".to_vec(),
			18,
			1_000_000,
		));

		let coin_id = 0u32;

		#[extrinsic_call]
		set_mint_permission(RawOrigin::Signed(caller), coin_id, grantee.clone(), true);

		// Verify permission granted
		assert_eq!(MintPermissions::<T>::get(coin_id, &grantee), true);
	}

	#[benchmark]
	fn set_fee_config() {
		let caller: T::AccountId = whitelisted_caller();
		
		// Setup: create a coin
		let deposit = T::CoinDeposit::get();
		T::Currency::make_free_balance_be(&caller, deposit + deposit);
		
		assert_ok!(MultiCoin::<T>::create_coin(
			RawOrigin::Signed(caller.clone()).into(),
			b"FEE".to_vec(),
			b"Fee Token".to_vec(),
			18,
			1_000_000,
			None,
			None,
		));

		let coin_id = 0u32;

		#[extrinsic_call]
		set_fee_config(RawOrigin::Signed(caller), coin_id, 100, 50);

		// Verify the fee config
		let coin_info = CoinMetadata::<T>::get(coin_id).unwrap();
		assert_eq!(coin_info.fee_config.transfer_fee, 100);
		assert_eq!(coin_info.fee_config.minimum_balance, 50);
	}

	impl_benchmark_test_suite!(MultiCoin, crate::mock::new_test_ext(), crate::mock::Test);
}
