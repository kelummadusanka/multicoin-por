# Guidance for AI coding agents — Multicoin with Proof-of-Reserve

This file contains focused, actionable information an AI coding agent needs to be productive in this repository.

Keep it short. Only follow discoverable patterns from the codebase.

1) Big-picture architecture (what to touch and why)
- This repository is a Substrate-based node template extended with two custom FRAME pallets:
  - `pallets/multicoin` — multi-coin asset management (create, mint, burn, transfer, fee config).
  - `pallets/proof-of-reserve` — custody/deposit and withdrawal workflow integrating external assets.
- The runtime (`runtime/`) composes these pallets into the `Runtime` (see `runtime/src/lib.rs`). The node (`node/`) provides service, CLI and networking.

2) Important files to read before making changes
- `runtime/src/lib.rs` — how pallets are wired into the runtime and runtime-level constants (UNIT, block times, pallet indices).
- `pallets/multicoin/src/lib.rs` — primary logic for coins, storage layout (`CoinMetadata`, `Balances`, `TotalSupply`, `SymbolToId`), and dispatchables like `create_coin`, `mint`, `transfer`.
- `pallets/proof-of-reserve/src/lib.rs` — deposit/withdraw request lifecycle, custody account handling, `DepositRequests` and `WithdrawalRequests` storages.
- `node/src/service.rs` and `node/src/main.rs` — node bootstrap, consensus, RPC wiring and tasks.
- `Cargo.toml` (workspace) — which crates are members and versions to reference for build/test commands.

3) Build, run, and test patterns (concrete commands)
- Build full workspace (compiles native runtime + node):
  - Use cargo in workspace root: `cargo build --release`
- Build only a pallet for unit tests / quick compilation:
  - `cargo test -p pallet-multicoin` or `cargo test -p pallet-proof-of-reserve`
- Run runtime unit / pallet tests (fast): `cargo test -p pallet-multicoin --lib` (or `--features runtime-benchmarks` when needed)
- Note: Substrate projects may require specific nightly/toolchain; check `env-setup/rust-toolchain.toml` and `env-setup/README.md` for environment setup.

4) Project-specific conventions and patterns
- Pallet crate names: `pallet-multicoin`, `pallet-proof-of-reserve`, `pallet-template` (see crate Cargo.toml files).
- Storage uses strongly typed bounded vectors (`BoundedVec<_, T::Max...>`) for limits — prefer using those when adding storage fields.
- Events and Errors follow FRAME conventions in each pallet's `Event` and `Error` enums; new dispatchables must emit events on success.
- Runtime includes pallets by index — changing pallet public API may require updating `runtime/src/lib.rs` and bumping runtime `spec_version`/`spec_name` if ABI changes.
- Fee handling: `pallets/multicoin` includes fee config per-coin (see `FeeConfig`, `can_pay_tx_fees`) — be careful when modifying fee-related logic.

5) Integration points & external dependencies
- Custody/cross-chain integration point is `pallets/proof-of-reserve` (external tx ids, external wallets). Any changes must preserve hashing/ID generation: request IDs use `T::Hashing::hash_of`.
- Node/service wiring uses Substrate service APIs in `node/src/service.rs`. Long-running tasks, telemetry and consensus setup live there.
- Runtime-native Wasm blob is generated into `OUT_DIR` for std builds; changes to pallet storage shape require runtime migrations.

6) Small examples to reference when generating code
- To find coin metadata by symbol: `SymbolToId::<T>::get(&bounded_symbol)` (see `pallets/multicoin/src/lib.rs`).
- To create a bounded vector from `Vec<u8>`: `let bounded: BoundedVec<u8, T::MaxSymbolLength> = symbol.try_into().map_err(|_| Error::<T>::SymbolTooLong)?;`
- To transfer native balance from custody in proof-of-reserve: `T::Currency::transfer(&custody_account, &request.user, request.native_amount, ExistenceRequirement::AllowDeath)?;`

7) Safety, migrations, and tests expectations
- Preserve storage keys and types when changing storage layout. If you must change layout, add a runtime migration using `OnRuntimeUpgrade` and bump `StorageVersion` (see `STORAGE_VERSION` in `pallets/multicoin`).
- Unit tests live next to pallets under `src/tests.rs` or `tests.rs` inside the pallet; run `cargo test -p pallet-multicoin` to exercise them.

8) When in doubt
- Read `runtime/src/lib.rs` to see how any change will affect the running chain.
- Prefer minimal, well-scoped diffs: change the pallet unit tests first, then runtime wiring, then node if necessary.

If any section is unclear or you'd like the file to contain more examples (CLI usage, CI, or how to run a local node), tell me which area to expand and I will iterate.
