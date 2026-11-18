# Working on Oritech (native Horizon mod, SDK format)

> **Build slots (v2):** every build/test step goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot
> is free — never poll, never background a build, never end a turn holding a lease. It does **not**
> `cd` into the port: wrap the command (`bash -c 'cd ports/oritech && …'`), pass `BUILD_LOCAL=1`
> (the far box has no copy of this port), and never pass `--target-dir`. A timeout exits **75**.
> Read `../_shared/build-slots-usage.md` before running anything.

This repository is the port, not Horizon and not horizon-mixins. Never modify the original
checkout or player resources; do not commit `target/`, `.horizon/`, exported sources or Minecraft
assets.

**There is no staging step and no mixin plan.** One flat `.hmod` holds `horizon-mod.toml` (the only
identity, `api = "0.2"`, `[native]` pins), `Cargo.toml` (its dependencies are `horizon-mod-api`
from the fetched `.horizon/sdk/` and `serde_json`, which the carried rules module uses),
`src/lib.rs` (the one `#[mod_api::horizon_mod] impl Hooks` and the two block-entity Parts),
`src/support.rs` (the glue: declarations, saved state, ticks, the right-click, the energy move and
the commands) and `src/runtime.rs` (the pure logic, unit-tested). `assets/` and `data/` are the
carried content and travel in the same file. The retired `.hnmod` layout is history:
`../_shared/evidence/oritech/retired-hnmod-0.1.0/`.

Keep every gameplay rule and number in `src/runtime.rs` with its test; the glue only reads the game,
calls `runtime`, and writes the result back. **`src/runtime.rs` is byte-identical to the retired
port's file (SHA-256 `0451000b2c153b1be1bb6750096a8cbf71b9c49de956ad7bc7195333c770661e`) — do not
"tidy" it.** A declaration or readout test belongs next to its code in `src/support.rs`.

Validate, in this order, each inside a lease:

```sh
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step test \
    -- bash -c 'cd ports/oritech && CARGO_INCREMENTAL=0 cargo test --offline'
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step lint \
    -- bash -c 'cd ports/oritech && cargo fmt --check && CARGO_INCREMENTAL=0 cargo clippy --all-targets --offline -- -D warnings'
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step build \
    -- bash -c 'cd ports/oritech && CARGO_INCREMENTAL=0 cargo horizon-mod build'
cp target/horizon-mod/oritech.hmod ./oritech.hmod
cargo horizon-mod check oritech.hmod && cargo horizon-mod fits oritech.hmod --build nightly
```

`cargo horizon-mod build` packs; it does not compile against the game. The native compile is
`horizon-launcher native-build . --instance <i> --trust-native-code`, which is **not installed
here** — the compile gate is `cargo test` + `cargo clippy -D warnings`. The in-game leg needs a
launcher-built game carrying this package: do not claim an in-game result without one.

The package's one registry dependency is `serde_json`, so **keep `Cargo.lock`** (it is packed and
is this package's record of its pins) — the opposite of the lock-free ports. `cargo fmt --check`,
never `--all`: that formats the fetched SDK.
