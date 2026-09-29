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

## Contract responsibilities

### Collective Investment Vault

`collective-investment-v2` coordinates the principal investment and treasury
workflow. It is initialized once with an administrator, the USDC token address,
and the participation-token contract address. The vault keeps proposal totals,
the number of voters, each member's contribution, execution state, approved
USDC amount, and a local participation-token supply and emission configuration.

Its public operations are:

- `initialize`: records the administrator and the two token contracts and
  creates the initial emission configuration.
- `upgrade`: allows the authenticated administrator to replace the contract
  WASM while preserving contract state.
- `set_emission_rate`: updates the locally stored token-emission rate.
- `mint_additional_tokens`: asks the external participation-token contract to
  mint tokens to a recipient and updates the vault's local supply counter. The
  vault must be authorized as the token contract's administrator/minter.
- `deposit_usdc`: transfers USDC from an authenticated member into the vault and
  mints an equal base-unit amount of participation tokens to that member.
- `admin_withdraw_tokens`: transfers participation tokens held by the vault to
  a recipient selected by the administrator.
- `vote_proposal`: locks a member's participation tokens in the vault, records
  the member's contribution, and updates the proposal's vote and voter totals.
- `execute_proposal`: after administrator authorization, verifies that locked
  votes cover the requested USDC payout, burns all tokens locked for the
  proposal, transfers USDC to the payout address, and marks the proposal as
  executed.
- `refund_proposal`: returns a member's locked tokens for a proposal that has
  not been executed and adjusts the stored vote statistics.
- `execute_partial`: supports milestone disbursements by burning a specified
  portion of tokens and transferring a specified USDC amount without marking
  the whole proposal as executed.
- `get_proposal_stats`, `get_contribution_stats`, `get_token_address`,
  `get_vault_usdc_balance`, and `get_vault_token_balance`: expose proposal,
  contribution, configuration, and treasury state.

State-changing operations emit `Mint`, `Deposit`, `Withdraw`, `DbVotes`,
`Vote`, `Exec`, `Refund`, or `Partial` events so off-chain services can follow
the contract lifecycle.

### Standard Participation Token

`standard-token` provides the participation token used by the vault. It stores
the administrator, balances, delegated spending allowances, and token metadata
(`name`, `symbol`, and `decimals`).

- `initialize` configures the token once.
- `mint` requires administrator authorization and credits the recipient.
- `burn` requires authorization from the token holder and destroys part of that
  holder's balance.
- `transfer` moves tokens after authorization from the source account.
- `approve` grants a spender an allowance with an optional expiration ledger.
- `allowance` returns zero after an allowance expires.
- `transfer_from` lets an authorized spender transfer within the available,
  unexpired allowance and reduces that allowance.
- `balance`, `name`, `symbol`, and `decimals` expose balances and metadata.

Mint, burn, transfer, and approval operations publish corresponding on-chain
events.

### Payment and Receipt Staking

`soroban_payment_and_receipt_staking_contract` records installment obligations
and scheduled payments. It maintains two collections: installment records with
an identifier, amount, period, date, and status; and payment records with a
wallet, amount, date, completion flag, and period.

- `initialize` stores the API key used by the scheduled-payment operations and
  prevents a second initialization.
- `installment_save_data` inserts a new installment with `pending` status.
- `payment_receipt_contract` changes matching installment records to
  `completed` when both identifier and amount match.
- `get_installment` returns all installment records.
- `get_installments_by_u64` filters installments by period.
- `get_payment_info` returns all scheduled-payment records.
- `promissory_payment` validates the supplied API key and records a scheduled
  payment.
- `promissory_payment_check` validates the API key, processes incomplete
  payments for a requested period, transfers tokens to each destination wallet,
  and marks those records as completed.

Payment execution uses the token contract address embedded in this contract and
logs the recipient, amount, and ledger timestamp. The API-key value is stored in
Soroban contract storage; callers and deployers should therefore not treat it as
an off-chain secret.

### Evidence Attestation

`evidence-attestation` provides an immutable reference for off-chain evidence.
It does not store evidence files; it stores a caller-provided SHA-256 value under
an evidence identifier.

- `initialize` records the authorized oracle once.
- `attest_evidence` requires oracle authorization, rejects an identifier that
  has already been attested, stores its hash, and emits an `Attest` event.
- `verify_evidence` compares a supplied hash with the hash stored for an
  identifier and returns a boolean result.

This allows an application to demonstrate that a file matches the version
previously anchored on-chain without publishing the file itself.

### Governance Voting Delegation

`governance-voting` manages delegation relationships within explicitly opened
voting rounds. It stores the administrator, each round's status, and the
delegate selected by each delegator for that round.

- `initialize` records the administrator once.
- `create_round` requires administrator authorization, rejects duplicate round
  identifiers, marks the round active, and emits `RndOpen`.
- `close_round` requires administrator authorization, closes an existing round,
  and emits `RndClose`; closed rounds no longer accept delegation changes.
- `delegate` requires the delegator's authorization and an active round. It
  prevents self-delegation, duplicate active delegation, cycles, and delegation
  chains deeper than ten links, then emits `Deleg`.
- `revoke_delegation` allows the authenticated delegator to remove an existing
  delegation while the round remains active and emits `Revoke`.
- `get_delegation` returns the selected delegate and fails when none exists.
- `has_delegation` checks whether a delegation exists.
- `get_round_status` returns `0` for an unknown round, `1` for an active round,
  and `2` for a closed round.

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

## Build provenance and hashes

The table distinguishes the WASM artifacts currently used by the application
from a fresh build of this comment-free source tree:

| WASM artifact | Active application SHA-256 | Comment-free source build SHA-256 |
| --- | --- | --- |
| `collective_investment_v2.wasm` | `970a865ec81d499c20ba7c96ba60b90fa1473e12980b0c8a1db5accdff21e68a` | `970a865ec81d499c20ba7c96ba60b90fa1473e12980b0c8a1db5accdff21e68a` |
| `evidence_attestation.wasm` | `5b624fe9f98b4e0ee6a88d73003797a9ee23cbca0e9e20bcd7e68653254f1274` | `5350da8ef1591f12ac20157b5e87dc8bb46e7a9bca96879ee21ecbbd6c77e3ba` |
| `governance_voting.wasm` | `daa2a9a5228016e50517693f017c198396bd17843610795e7b85e49828f25478` | `cb7b9d32dc28533741c78e250951fa3a4eebc26172b16bc796c4a2ebf668f417` |
| `payment_and_receipt_staking_contract.wasm` | `8e9a37e5f76e33e8d9ab2925e9a0233a0213ceb98e7a65d453810412eebc9f9b` | `8e9a37e5f76e33e8d9ab2925e9a0233a0213ceb98e7a65d453810412eebc9f9b` |
| `standard_token.wasm` | `35f8ee1495e97714e4b5d0cc53816a099baa075516a9d228f6c8b1c6c8418879` | `c4e59836e38573a9f0dabd8be74510ae212020eb3f06c8e60cca3f81332c8dfc` |

Removing Rust documentation comments can change Soroban contract-spec metadata
embedded in a WASM file, and source layout can affect generated metadata. That
is why three byte-for-byte hashes differ even though the executable Rust logic
and public operations remain unchanged. The two matching rows are
byte-for-byte identical to the active application artifacts.

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
