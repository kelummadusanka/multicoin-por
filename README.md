# Multi-Coin Pallet

A comprehensive Substrate pallet that enables a blockchain to natively support and manage multiple coins on a single runtime. Each coin is treated as a native asset with individual state, supply, and economic logic.

## Features

### 🪙 Multi-Coin Definition & Registration
- Create new native coins with unique metadata (symbol, name, decimals)
- Set initial supply and minting/burning permissions
- On-chain registry of all defined coins with metadata
- Symbol-to-ID mapping for easy lookup

### 👑 Coin Ownership & Roles
- Define admin/creator roles for each coin
- Transfer coin ownership between accounts
- Assign mint/burn privileges on a per-coin basis
- Role-based permission system for coin management

### 💸 Native Coin Transfers
- Native coin transfers via `transfer(coin_id, from, to, amount)`
- Balance queries: `balance_of(account, coin_id)`
- Total supply queries: `total_supply(coin_id)`
- Overflow protection and safety checks

### 🔥 Controlled Minting and Burning
- Permission-based minting system
- Controlled burning from account balances
- Event logging for all mint/burn operations
- Supply cap enforcement

### 📊 On-Chain Metadata and Symbol Lookup
- Complete metadata stored on-chain
- Efficient lookups by symbol or coin ID
- Bounded storage with configurable limits
- Symbol uniqueness enforcement

### 🛡️ Storage Optimization
- Efficient storage design using `StorageDoubleMap`
- Configurable limits to prevent blockchain bloat
- Deposit-based coin creation to prevent spam
- Memory-efficient data structures

## Architecture

### Storage Items

```rust
// Coin metadata storage
CoinMetadata<T: Config> = StorageMap<CoinId, CoinInfo<...>>

// Balance storage: CoinId -> AccountId -> Balance
Balances<T: Config> = StorageDoubleMap<CoinId, AccountId, u128>

// Total supply tracking
TotalSupply<T: Config> = StorageMap<CoinId, u128>

// Symbol to ID mapping for lookups  
SymbolToId<T: Config> = StorageMap<Symbol, CoinId>

// Minting permissions: CoinId -> AccountId -> bool
MintPermissions<T: Config> = StorageDoubleMap<CoinId, AccountId, bool>
```

### Key Types

```rust
// Unique identifier for each coin
pub type CoinId = u32;

// Comprehensive coin information
pub struct CoinInfo<Symbol, Name, AccountId, Balance> {
    pub symbol: Symbol,      // e.g., "BTC", "ETH"
    pub name: Name,          // e.g., "Bitcoin", "Ethereum"
    pub decimals: u8,        // Number of decimal places
    pub owner: AccountId,    // Current owner of the coin
    pub deposit: Balance,    // Deposit paid for creation
}
```

## Configuration

### Pallet Config Traits

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<...>;
    type WeightInfo: WeightInfo;
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    
    // Configuration constants
    type MaxSymbolLength: Get<u32>;    // Max symbol length (e.g., 12)
    type MaxNameLength: Get<u32>;      // Max name length (e.g., 64) 
    type MaxCoins: Get<u32>;           // Max coins allowed (e.g., 10,000)
    type CoinDeposit: Get<Balance>;    // Deposit for coin creation
    type MaxSupply: Get<u128>;         // Maximum supply per coin
}
```

### Runtime Configuration

```rust
// Multi-Coin pallet configuration
parameter_types! {
    pub const MaxSymbolLength: u32 = 12;
    pub const MaxNameLength: u32 = 64;
    pub const MaxCoins: u32 = 10_000;
    pub const CoinDeposit: Balance = 10 * UNIT;  // 10 tokens
    pub const MaxCoinSupply: u128 = u128::MAX;
}

impl pallet_multi_coin::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_multi_coin::weights::SubstrateWeight<Runtime>;
    type Currency = Balances;
    type MaxSymbolLength = MaxSymbolLength;
    type MaxNameLength = MaxNameLength;
    type MaxCoins = MaxCoins;
    type CoinDeposit = CoinDeposit;
    type MaxSupply = MaxCoinSupply;
}
```

## API Reference

### Dispatchable Functions

#### `create_coin`
Creates a new coin with specified metadata and initial supply.

```rust
pub fn create_coin(
    origin: OriginFor<T>,
    symbol: Vec<u8>,        // Coin symbol (e.g., "BTC")
    name: Vec<u8>,          // Coin name (e.g., "Bitcoin")
    decimals: u8,           // Number of decimals
    initial_supply: u128,   // Initial supply to mint
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Symbol must be unique and within length limits
- Name must be within length limits  
- Initial supply must be > 0 and <= MaxSupply
- Caller must have sufficient balance for deposit

**Events Emitted:**
- `CoinCreated { coin_id, symbol, name, creator, initial_supply }`

#### `transfer`
Transfers coins between accounts.

```rust
pub fn transfer(
    origin: OriginFor<T>,
    coin_id: CoinId,        // ID of coin to transfer
    to: T::AccountId,       // Recipient account
    amount: u128,           // Amount to transfer
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Coin must exist
- Sender must have sufficient balance
- Amount must be > 0
- Cannot transfer to self

**Events Emitted:**
- `Transfer { coin_id, from, to, amount }`

#### `mint`
Mints new coins to a specified account (requires minting permission).

```rust
pub fn mint(
    origin: OriginFor<T>,
    coin_id: CoinId,        // ID of coin to mint
    to: T::AccountId,       // Account to receive minted coins
    amount: u128,           // Amount to mint
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Caller must have minting permission for the coin
- Coin must exist
- Amount must be > 0
- Total supply after minting must not exceed MaxSupply

**Events Emitted:**
- `Minted { coin_id, to, amount }`

#### `burn`
Burns coins from the caller's account.

```rust
pub fn burn(
    origin: OriginFor<T>,
    coin_id: CoinId,        // ID of coin to burn
    amount: u128,           // Amount to burn
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Coin must exist
- Caller must have sufficient balance
- Amount must be > 0

**Events Emitted:**
- `Burned { coin_id, from, amount }`

#### `transfer_ownership`
Transfers ownership of a coin to another account.

```rust
pub fn transfer_ownership(
    origin: OriginFor<T>,
    coin_id: CoinId,        // ID of coin
    new_owner: T::AccountId,// New owner account
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Caller must be current owner of the coin
- Coin must exist

**Effects:**
- Updates coin owner
- Transfers minting permission from old to new owner

**Events Emitted:**
- `OwnershipTransferred { coin_id, old_owner, new_owner }`

#### `set_mint_permission`
Grants or revokes minting permission for an account.

```rust
pub fn set_mint_permission(
    origin: OriginFor<T>,
    coin_id: CoinId,        // ID of coin
    account: T::AccountId,  // Account to set permission for
    can_mint: bool,         // Whether account can mint
) -> DispatchResult
```

**Requirements:**
- Origin must be signed
- Caller must be owner of the coin
- Coin must exist

**Events Emitted:**
- `MintPermissionSet { coin_id, account, can_mint }`

### Public Functions (Read-only)

#### `balance_of`
Gets the balance of an account for a specific coin.

```rust
pub fn balance_of(account: &T::AccountId, coin_id: CoinId) -> u128
```

#### `total_supply_of`
Gets the total supply of a coin.

```rust
pub fn total_supply_of(coin_id: CoinId) -> u128
```

#### `get_coin_metadata`
Retrieves complete metadata for a coin.

```rust
pub fn get_coin_metadata(coin_id: CoinId) -> Option<CoinInfo<...>>
```

#### `get_coin_id_by_symbol`
Finds coin ID by symbol.

```rust
pub fn get_coin_id_by_symbol(symbol: &[u8]) -> Option<CoinId>
```

#### `has_mint_permission`
Checks if an account has minting permission for a coin.

```rust
pub fn has_mint_permission(coin_id: CoinId, account: &T::AccountId) -> bool
```

## Events

```rust
pub enum Event<T: Config> {
    /// A new coin was created [coin_id, symbol, name, creator, initial_supply]
    CoinCreated { coin_id: CoinId, symbol: Vec<u8>, name: Vec<u8>, creator: T::AccountId, initial_supply: u128 },
    
    /// Coins were transferred [coin_id, from, to, amount]
    Transfer { coin_id: CoinId, from: T::AccountId, to: T::AccountId, amount: u128 },
    
    /// Coins were minted [coin_id, to, amount]
    Minted { coin_id: CoinId, to: T::AccountId, amount: u128 },
    
    /// Coins were burned [coin_id, from, amount]
    Burned { coin_id: CoinId, from: T::AccountId, amount: u128 },
    
    /// Coin ownership was transferred [coin_id, old_owner, new_owner]
    OwnershipTransferred { coin_id: CoinId, old_owner: T::AccountId, new_owner: T::AccountId },
    
    /// Mint permission was set [coin_id, account, can_mint]
    MintPermissionSet { coin_id: CoinId, account: T::AccountId, can_mint: bool },
}
```

## Errors

```rust
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
}
```

## Usage Examples

### Creating a New Coin

```rust
// Create a Bitcoin-like coin
MultiCoin::create_coin(
    RuntimeOrigin::signed(creator_account),
    b"BTC".to_vec(),           // symbol
    b"Bitcoin".to_vec(),       // name
    8,                         // 8 decimal places
    21_000_000 * 10_u128.pow(8), // 21M BTC with 8 decimals
)?;
```

### Transferring Coins

```rust
// Transfer 1 BTC (assuming coin_id = 0, 8 decimals)
let coin_id = 0;
let amount = 1 * 10_u128.pow(8); // 1 BTC
MultiCoin::transfer(
    RuntimeOrigin::signed(sender),
    coin_id,
    recipient,
    amount,
)?;
```

### Minting Additional Supply

```rust
// Owner mints 1000 more coins
let coin_id = 0;
let amount = 1000 * 10_u128.pow(8);
MultiCoin::mint(
    RuntimeOrigin::signed(owner),
    coin_id,
    beneficiary,
    amount,
)?;
```

### Querying Balances

```rust
// Get account balance for a specific coin
let balance = MultiCoin::balance_of(&account, coin_id);

// Get total supply of a coin
let total_supply = MultiCoin::total_supply_of(coin_id);

// Find coin by symbol
let coin_id = MultiCoin::get_coin_id_by_symbol(b"BTC");
```

### Managing Permissions

```rust
// Grant minting permission to another account
MultiCoin::set_mint_permission(
    RuntimeOrigin::signed(owner),
    coin_id,
    minter_account,
    true, // grant permission
)?;

// Check if account can mint
let can_mint = MultiCoin::has_mint_permission(coin_id, &account);
```

## Integration Guide

### 1. Add to Workspace

Add the pallet to your workspace `Cargo.toml`:

```toml
[workspace]
members = [
    "pallets/multi-coin",
    # ... other pallets
]

[workspace.dependencies]
pallet-multi-coin = { path = "./pallets/multi-coin", default-features = false }
```

### 2. Runtime Integration

Add to runtime `Cargo.toml`:

```toml
[dependencies]
pallet-multi-coin.workspace = true

[features]
std = [
    "pallet-multi-coin/std",
    # ... other pallets
]
runtime-benchmarks = [
    "pallet-multi-coin/runtime-benchmarks",
    # ... other pallets
]
```

### 3. Configure Runtime

In `runtime/src/lib.rs`:

```rust
// Add to runtime construction
#[frame_support::runtime]
mod runtime {
    #[runtime::pallet_index(8)]
    pub type MultiCoin = pallet_multi_coin;
}
```

### 4. Configure Parameters

In `runtime/src/configs/mod.rs`, add configuration parameters and implementation.

## Testing

The pallet includes comprehensive tests covering:

- ✅ Coin creation and validation
- ✅ Transfer functionality and edge cases  
- ✅ Minting with permission checks
- ✅ Burning functionality
- ✅ Ownership transfer
- ✅ Permission management
- ✅ Multiple coin independence
- ✅ Storage limit enforcement
- ✅ Overflow protection
- ✅ Error handling

Run tests with:

```bash
cargo test -p pallet-multi-coin
```

## Benchmarking

The pallet includes benchmarks for all extrinsic functions:

- `create_coin`
- `transfer` 
- `mint`
- `burn`
- `transfer_ownership`
- `set_mint_permission`

Run benchmarks with:

```bash
cargo test -p pallet-multi-coin --features runtime-benchmarks
```

## Security Considerations

### Deposit Protection
- Coin creation requires a deposit to prevent spam
- Deposits are reserved from creator's account
- Consider deposit amount based on chain economics

### Permission Model
- Only coin owners can manage permissions
- Minting permissions are explicit and revocable
- Ownership transfer automatically updates permissions

### Supply Control
- Maximum supply limits prevent inflation attacks
- Burning reduces total supply permanently
- Overflow protection prevents arithmetic attacks

### Access Control
- Role-based permission system
- Signed origins required for all operations
- Authorization checks on sensitive operations

## Future Enhancements

Potential improvements for future versions:

- **Multi-Currency Fee Payment**: Allow paying transaction fees in any registered coin
- **Exchange Integration**: Built-in DEX functionality between coins
- **Governance Integration**: Community governance for coin parameters
- **Metadata Extensions**: Additional metadata fields (icon, website, etc.)
- **Transfer Fees**: Configurable transfer fees per coin
- **Freezing/Thawing**: Ability to freeze/unfreeze accounts
- **Batch Operations**: Batch transfers and mints for efficiency
- **Cross-Chain Support**: Integration with XCM for cross-chain transfers

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the MIT-0 License - see the LICENSE file for details.
