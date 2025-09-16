use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

// Helper function to create a test coin
fn create_test_coin(creator: u64, symbol: &str, name: &str) -> Result<u32, sp_runtime::DispatchError> {
	MultiCoin::create_coin(
		RuntimeOrigin::signed(creator),
		symbol.as_bytes().to_vec(),
		name.as_bytes().to_vec(),
		18, // decimals
		1000, // initial supply
	)?;
	
	// Get the coin ID (it should be the current NextCoinId - 1)
	let coin_id = MultiCoin::next_coin_id().saturating_sub(1);
	Ok(coin_id)
}

#[test]
fn create_coin_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create a coin
		assert_ok!(MultiCoin::create_coin(
			RuntimeOrigin::signed(1),
			b"BTC".to_vec(),
			b"Bitcoin".to_vec(),
			8,
			21_000_000
		));

		// Check that coin was created
		let coin_id = 0;
		let metadata = MultiCoin::coin_metadata(coin_id).unwrap();
		assert_eq!(metadata.symbol.to_vec(), b"BTC".to_vec());
		assert_eq!(metadata.name.to_vec(), b"Bitcoin".to_vec());
		assert_eq!(metadata.decimals, 8);
		assert_eq!(metadata.owner, 1);

		// Check initial balance
		assert_eq!(MultiCoin::balances(coin_id, 1), 21_000_000);
		assert_eq!(MultiCoin::total_supply(coin_id), 21_000_000);

		// Check that symbol mapping exists
		let symbol_bounded: frame_support::BoundedVec<u8, <Test as crate::Config>::MaxSymbolLength> = 
			b"BTC".to_vec().try_into().unwrap();
		assert_eq!(MultiCoin::symbol_to_id(symbol_bounded), Some(coin_id));

		// Check that minting permission was granted
		assert!(MultiCoin::mint_permissions(coin_id, 1));

		// Check next coin ID was incremented
		assert_eq!(MultiCoin::next_coin_id(), 1);

		// Check events
		System::assert_last_event(Event::CoinCreated {
			coin_id,
			symbol: b"BTC".to_vec(),
			name: b"Bitcoin".to_vec(),
			creator: 1,
			initial_supply: 21_000_000,
		}.into());
	});
}

#[test]
fn create_coin_fails_with_duplicate_symbol() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create first coin
		assert_ok!(MultiCoin::create_coin(
			RuntimeOrigin::signed(1),
			b"BTC".to_vec(),
			b"Bitcoin".to_vec(),
			8,
			21_000_000
		));

		// Try to create coin with same symbol
		assert_noop!(
			MultiCoin::create_coin(
				RuntimeOrigin::signed(2),
				b"BTC".to_vec(),
				b"Bitcoin Cash".to_vec(),
				8,
				21_000_000
			),
			Error::<Test>::SymbolAlreadyExists
		);
	});
}

#[test]
fn create_coin_fails_with_zero_supply() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			MultiCoin::create_coin(
				RuntimeOrigin::signed(1),
				b"ZERO".to_vec(),
				b"Zero Coin".to_vec(),
				18,
				0
			),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn create_coin_fails_with_exceeding_max_supply() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let max_supply = <Test as crate::Config>::MaxSupply::get();
		assert_noop!(
			MultiCoin::create_coin(
				RuntimeOrigin::signed(1),
				b"BIG".to_vec(),
				b"Big Coin".to_vec(),
				18,
				max_supply + 1
			),
			Error::<Test>::ExceedsMaxSupply
		);
	});
}

#[test]
fn transfer_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Transfer some coins
		assert_ok!(MultiCoin::transfer(
			RuntimeOrigin::signed(1),
			coin_id,
			2,
			100
		));

		// Check balances
		assert_eq!(MultiCoin::balances(coin_id, 1), 900);
		assert_eq!(MultiCoin::balances(coin_id, 2), 100);

		// Check event
		System::assert_last_event(Event::Transfer {
			coin_id,
			from: 1,
			to: 2,
			amount: 100,
		}.into());
	});
}

#[test]
fn transfer_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Try to transfer more than balance
		assert_noop!(
			MultiCoin::transfer(
				RuntimeOrigin::signed(1),
				coin_id,
				2,
				2000
			),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn transfer_fails_with_nonexistent_coin() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			MultiCoin::transfer(
				RuntimeOrigin::signed(1),
				999, // Non-existent coin
				2,
				100
			),
			Error::<Test>::CoinNotFound
		);
	});
}

#[test]
fn transfer_fails_to_self() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		assert_noop!(
			MultiCoin::transfer(
				RuntimeOrigin::signed(1),
				coin_id,
				1, // Same as sender
				100
			),
			Error::<Test>::TransferToSelf
		);
	});
}

#[test]
fn transfer_fails_with_zero_amount() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		assert_noop!(
			MultiCoin::transfer(
				RuntimeOrigin::signed(1),
				coin_id,
				2,
				0
			),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn mint_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Mint coins to account 2
		assert_ok!(MultiCoin::mint(
			RuntimeOrigin::signed(1), // Creator has minting permission
			coin_id,
			2,
			500
		));

		// Check balances and total supply
		assert_eq!(MultiCoin::balances(coin_id, 2), 500);
		assert_eq!(MultiCoin::total_supply(coin_id), 1500);

		// Check event
		System::assert_last_event(Event::Minted {
			coin_id,
			to: 2,
			amount: 500,
		}.into());
	});
}

#[test]
fn mint_fails_without_permission() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Try to mint without permission
		assert_noop!(
			MultiCoin::mint(
				RuntimeOrigin::signed(2), // Account 2 doesn't have permission
				coin_id,
				2,
				500
			),
			Error::<Test>::NoMintPermission
		);
	});
}

#[test]
fn burn_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Burn some coins
		assert_ok!(MultiCoin::burn(
			RuntimeOrigin::signed(1),
			coin_id,
			200
		));

		// Check balance and total supply
		assert_eq!(MultiCoin::balances(coin_id, 1), 800);
		assert_eq!(MultiCoin::total_supply(coin_id), 800);

		// Check event
		System::assert_last_event(Event::Burned {
			coin_id,
			from: 1,
			amount: 200,
		}.into());
	});
}

#[test]
fn burn_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Try to burn more than balance
		assert_noop!(
			MultiCoin::burn(
				RuntimeOrigin::signed(1),
				coin_id,
				2000
			),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn transfer_ownership_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Transfer ownership
		assert_ok!(MultiCoin::transfer_ownership(
			RuntimeOrigin::signed(1),
			coin_id,
			2
		));

		// Check new owner
		let metadata = MultiCoin::coin_metadata(coin_id).unwrap();
		assert_eq!(metadata.owner, 2);

		// Check that minting permission was transferred
		assert!(!MultiCoin::mint_permissions(coin_id, 1));
		assert!(MultiCoin::mint_permissions(coin_id, 2));

		// Check event
		System::assert_last_event(Event::OwnershipTransferred {
			coin_id,
			old_owner: 1,
			new_owner: 2,
		}.into());
	});
}

#[test]
fn transfer_ownership_fails_if_not_owner() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Try to transfer ownership without being owner
		assert_noop!(
			MultiCoin::transfer_ownership(
				RuntimeOrigin::signed(2),
				coin_id,
				3
			),
			Error::<Test>::NotAuthorized
		);
	});
}

#[test]
fn set_mint_permission_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Grant minting permission to account 2
		assert_ok!(MultiCoin::set_mint_permission(
			RuntimeOrigin::signed(1),
			coin_id,
			2,
			true
		));

		// Check permission
		assert!(MultiCoin::mint_permissions(coin_id, 2));

		// Account 2 should now be able to mint
		assert_ok!(MultiCoin::mint(
			RuntimeOrigin::signed(2),
			coin_id,
			3,
			100
		));

		// Revoke permission
		assert_ok!(MultiCoin::set_mint_permission(
			RuntimeOrigin::signed(1),
			coin_id,
			2,
			false
		));

		// Check permission was revoked
		assert!(!MultiCoin::mint_permissions(coin_id, 2));

		// Account 2 should no longer be able to mint
		assert_noop!(
			MultiCoin::mint(
				RuntimeOrigin::signed(2),
				coin_id,
				3,
				100
			),
			Error::<Test>::NoMintPermission
		);
	});
}

#[test]
fn set_mint_permission_fails_if_not_owner() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Try to set permission without being owner
		assert_noop!(
			MultiCoin::set_mint_permission(
				RuntimeOrigin::signed(2),
				coin_id,
				3,
				true
			),
			Error::<Test>::NotAuthorized
		);
	});
}

#[test]
fn multiple_coins_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create multiple coins
		let btc_id = create_test_coin(1, "BTC", "Bitcoin").unwrap();
		let eth_id = create_test_coin(2, "ETH", "Ethereum").unwrap();
		let ada_id = create_test_coin(3, "ADA", "Cardano").unwrap();

		// Verify all coins were created with different IDs
		assert_eq!(btc_id, 0);
		assert_eq!(eth_id, 1);
		assert_eq!(ada_id, 2);

		// Check balances
		assert_eq!(MultiCoin::balances(btc_id, 1), 1000);
		assert_eq!(MultiCoin::balances(eth_id, 2), 1000);
		assert_eq!(MultiCoin::balances(ada_id, 3), 1000);

		// Check owners
		assert_eq!(MultiCoin::coin_metadata(btc_id).unwrap().owner, 1);
		assert_eq!(MultiCoin::coin_metadata(eth_id).unwrap().owner, 2);
		assert_eq!(MultiCoin::coin_metadata(ada_id).unwrap().owner, 3);

		// Cross-transfers should work
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(1), btc_id, 2, 100));
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(2), eth_id, 1, 200));

		// Check updated balances
		assert_eq!(MultiCoin::balances(btc_id, 1), 900);
		assert_eq!(MultiCoin::balances(btc_id, 2), 100);
		assert_eq!(MultiCoin::balances(eth_id, 1), 200);
		assert_eq!(MultiCoin::balances(eth_id, 2), 800);
	});
}

#[test]
fn helper_functions_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		// Test balance_of
		assert_eq!(MultiCoin::balance_of(&1, coin_id), 1000);
		assert_eq!(MultiCoin::balance_of(&2, coin_id), 0);

		// Test total_supply_of
		assert_eq!(MultiCoin::total_supply_of(coin_id), 1000);

		// Test get_coin_metadata
		let metadata = MultiCoin::get_coin_metadata(coin_id).unwrap();
		assert_eq!(metadata.symbol.to_vec(), b"TEST".to_vec());
		assert_eq!(metadata.name.to_vec(), b"Test Coin".to_vec());

		// Test get_coin_id_by_symbol
		assert_eq!(MultiCoin::get_coin_id_by_symbol(b"TEST"), Some(coin_id));
		assert_eq!(MultiCoin::get_coin_id_by_symbol(b"NOTFOUND"), None);

		// Test has_mint_permission
		assert!(MultiCoin::has_mint_permission(coin_id, &1));
		assert!(!MultiCoin::has_mint_permission(coin_id, &2));
	});
}

#[test]
fn mint_exceeding_max_supply_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let coin_id = create_test_coin(1, "TEST", "Test Coin").unwrap();

		let max_supply = <Test as crate::Config>::MaxSupply::get();
		let current_supply = MultiCoin::total_supply(coin_id);
		let amount_to_exceed = max_supply - current_supply + 1;

		assert_noop!(
			MultiCoin::mint(
				RuntimeOrigin::signed(1),
				coin_id,
				2,
				amount_to_exceed
			),
			Error::<Test>::ExceedsMaxSupply
		);
	});
}

#[test]
fn coin_creation_with_insufficient_deposit_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create account with insufficient balance for deposit
		// Note: The account needs at least CoinDeposit amount
		// In our test setup, account 4 has 0 balance
		assert_noop!(
			MultiCoin::create_coin(
				RuntimeOrigin::signed(4),
				b"POOR".to_vec(),
				b"Poor Coin".to_vec(),
				18,
				1000
			),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn workflow_test() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Create a workflow token
		let coin_id = create_test_coin(1, "WORK", "Workflow Token").unwrap();

		// Transfer some tokens to workers
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(1), coin_id, 2, 250));
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(1), coin_id, 3, 250));

		// Grant minting permission to a manager
		assert_ok!(MultiCoin::set_mint_permission(RuntimeOrigin::signed(1), coin_id, 2, true));

		// Manager can now mint rewards
		assert_ok!(MultiCoin::mint(RuntimeOrigin::signed(2), coin_id, 3, 100));

		// Workers can transfer tokens between themselves
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(2), coin_id, 3, 50));
		assert_ok!(MultiCoin::transfer(RuntimeOrigin::signed(3), coin_id, 2, 25));

		// Final balances
		assert_eq!(MultiCoin::balances(coin_id, 1), 500); // Owner: 1000 - 250 - 250
		assert_eq!(MultiCoin::balances(coin_id, 2), 225); // Worker1: 250 - 50 + 25
		assert_eq!(MultiCoin::balances(coin_id, 3), 375); // Worker2: 250 + 100 + 50 - 25

		// Total supply increased due to minting
		assert_eq!(MultiCoin::total_supply(coin_id), 1100); // 1000 + 100
	});

}

#[test]
fn transfer_with_fee_works() {
    new_test_ext().execute_with(|| {
        let caller: u64 = 1; // From mock balances
        let recipient: u64 = 2;
        let coin_id = 0;

        // Create coin
        assert_ok!(MultiCoin::<Test>::create_coin(
            RawOrigin::Signed(caller).into(),
            b"TEST".to_vec(),
            b"Test Token".to_vec(),
            18,
            1_000_000,
            None,
            None,
        ));

        // Set fee config
        assert_ok!(MultiCoin::<Test>::set_fee_config(
            RawOrigin::Signed(caller).into(),
            coin_id,
            100,
            50,
        ));

        // Transfer
        assert_ok!(MultiCoin::<Test>::transfer(
            RawOrigin::Signed(caller).into(),
            coin_id,
            recipient,
            500_000,
        ));

        // Verify balances and supply
        assert_eq!(Balances::<Test>::get(coin_id, &caller), 1_000_000 - 500_000 - 100);
        assert_eq!(Balances::<Test>::get(coin_id, &recipient), 500_000);
        assert_eq!(TotalSupply::<Test>::get(coin_id), 1_000_000 - 100);
    });
}

#[test]
fn set_fee_config_fails_non_owner() {
    new_test_ext().execute_with(|| {
        let caller: u64 = 1;
        let non_owner: u64 = 2;
        let coin_id = 0;

        // Create coin
        assert_ok!(MultiCoin::<Test>::create_coin(
            RawOrigin::Signed(caller).into(),
            b"TEST".to_vec(),
            b"Test Token".to_vec(),
            18,
            1_000_000,
            None,
            None,
        ));

        // Non-owner tries to set fee
        assert_noop!(
            MultiCoin::<Test>::set_fee_config(
                RawOrigin::Signed(non_owner).into(),
                coin_id,
                100,
                50,
            ),
            Error::<Test>::NotAuthorized
        );
    });
}
