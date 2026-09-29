# Collective Investment — Soroban contracts

Public Rust source code for the Soroban smart contracts used by the Collective
Investment project.

## Included contracts

| Contract | Rust package | Runtime WASM artifact |
| --- | --- | --- |
| Collective investment | `collective-investment-v2` | `collective_investment_v2.wasm` |
| Payment and receipt staking | `soroban_payment_and_receipt_staking_contract` | `payment_and_receipt_staking_contract.wasm` |
| Standard token | `standard-token` | `standard_token.wasm` |
| Evidence attestation | `evidence-attestation` | `evidence_attestation.wasm` |
| Governance voting | `governance-voting` | `governance_voting.wasm` |

The repository intentionally contains the Rust workspace and contract sources
only. Generated `target/` build output, deployed WASM binaries, application code,
credentials, and private keys are not included.

## Requirements

- Rust toolchain with the `wasm32-unknown-unknown` target
- Soroban CLI (Stellar CLI)

Install the Rust target:

```sh
rustup target add wasm32-unknown-unknown
```

## Build

Build all contracts with the release profile defined in the workspace:

```sh
cargo build --workspace --target wasm32-unknown-unknown --release
```

The generated WASM files are written below
`target/wasm32-unknown-unknown/release/`. Cargo normalizes Rust package names by
replacing hyphens with underscores in artifact filenames.

## Reproducibility

The release build above reproduces the SHA-256 hashes of the five WASM
artifacts used by the application:

| WASM artifact | SHA-256 |
| --- | --- |
| `collective_investment_v2.wasm` | `970a865ec81d499c20ba7c96ba60b90fa1473e12980b0c8a1db5accdff21e68a` |
| `evidence_attestation.wasm` | `5b624fe9f98b4e0ee6a88d73003797a9ee23cbca0e9e20bcd7e68653254f1274` |
| `governance_voting.wasm` | `daa2a9a5228016e50517693f017c198396bd17843610795e7b85e49828f25478` |
| `payment_and_receipt_staking_contract.wasm` | `8e9a37e5f76e33e8d9ab2925e9a0233a0213ceb98e7a65d453810412eebc9f9b` |
| `standard_token.wasm` | `35f8ee1495e97714e4b5d0cc53816a099baa075516a9d228f6c8b1c6c8418879` |

The payment contract package produces
`soroban_payment_and_receipt_staking_contract.wasm`; the application stores
that same binary under the shorter filename shown in the table.

## Source layout

```text
contracts/
├── collectiveInvestment/
├── evidence_attestation/
├── governance_voting/
├── paymentAndReceiptStaking/
└── standard_token/
```

Each directory contains its own `Cargo.toml` and `src/lib.rs`; existing unit
tests are included where available.
