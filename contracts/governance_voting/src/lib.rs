#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

// ─── Storage Keys ───────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    /// Delegation record: maps (round_id, delegator) -> delegatee
    Delegation(DelegationKey),
    /// Round status: maps round_id -> RoundStatus
    RoundStatus(String),
}

#[contracttype]
#[derive(Clone)]
pub struct DelegationKey {
    pub round_id: String,
    pub delegator: Address,
}

/// Tracks the status of a voting round for delegation purposes.
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum RoundStatus {
    Active,
    Closed,
}

// ─── Contract ───────────────────────────────────────────────────────────────

#[contract]
pub struct GovernanceVotingContract;

#[contractimpl]
impl GovernanceVotingContract {

    // ── Initialization ──────────────────────────────────────────────────

    /// Initialize the contract with an admin address.
    /// Can only be called once.
    pub fn initialize(e: Env, admin: Address) {
        if e.storage().persistent().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        e.storage().persistent().set(&DataKey::Admin, &admin);
    }

    // ── Round Management ────────────────────────────────────────────────

    /// Register a voting round as active. Only admin can create rounds.
    pub fn create_round(e: Env, round_id: String) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::RoundStatus(round_id.clone());
        if e.storage().persistent().has(&key) {
            panic!("Round already exists");
        }

        e.storage().persistent().set(&key, &RoundStatus::Active);
        e.events().publish((symbol_short!("RndOpen"), round_id), 0_u32);
    }

    /// Close a voting round. Only admin. No more delegations after close.
    pub fn close_round(e: Env, round_id: String) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::RoundStatus(round_id.clone());
        let status: RoundStatus = e.storage().persistent().get(&key)
            .unwrap_or_else(|| panic!("Round not found"));

        if status == RoundStatus::Closed {
            panic!("Round already closed");
        }

        e.storage().persistent().set(&key, &RoundStatus::Closed);
        e.events().publish((symbol_short!("RndClose"), round_id), 0_u32);
    }

    // ── Delegation ──────────────────────────────────────────────────────

    /// Delegate voting power from delegator to delegatee for a specific round.
    ///
    /// Invariants enforced:
    /// 1. Round must be active.
    /// 2. delegator != delegatee (no self-delegation).
    /// 3. delegator must not have an existing active delegation.
    /// 4. No cycles: delegatee must not have delegated (directly or
    ///    transitively) back to delegator.
    ///
    /// The delegator must authorize this call.
    pub fn delegate(e: Env, round_id: String, delegator: Address, delegatee: Address) {
        delegator.require_auth();

        // Rule 1: Round must be active
        let round_key = DataKey::RoundStatus(round_id.clone());
        let round_status: RoundStatus = e.storage().persistent().get(&round_key)
            .unwrap_or_else(|| panic!("Round not found"));
        if round_status != RoundStatus::Active {
            panic!("Round is not active");
        }

        // Rule 2: No self-delegation
        if delegator == delegatee {
            panic!("Cannot delegate to yourself");
        }

        // Rule 3: No existing active delegation for this delegator in this round
        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id: round_id.clone(),
            delegator: delegator.clone(),
        });
        if e.storage().persistent().has(&delegation_key) {
            panic!("Already delegated in this round");
        }

        // Rule 4: Cycle detection — walk the chain from delegatee
        // If delegatee (or anyone they delegated to) eventually points
        // back to delegator, it's a cycle.
        let mut current = delegatee.clone();
        let max_depth: u32 = 10; // Prevent unbounded loop
        let mut depth: u32 = 0;
        loop {
            if depth >= max_depth {
                panic!("Delegation chain too deep");
            }

            let check_key = DataKey::Delegation(DelegationKey {
                round_id: round_id.clone(),
                delegator: current.clone(),
            });

            match e.storage().persistent().get::<_, Address>(&check_key) {
                Some(next) => {
                    if next == delegator {
                        panic!("Delegation would create a cycle");
                    }
                    current = next;
                    depth += 1;
                }
                None => break, // End of chain, no cycle
            }
        }

        // All checks passed — store delegation
        e.storage().persistent().set(&delegation_key, &delegatee);

        e.events().publish(
            (symbol_short!("Deleg"), round_id, delegator),
            delegatee,
        );
    }

    /// Revoke an existing delegation. Only the delegator can revoke.
    /// Round must still be active.
    pub fn revoke_delegation(e: Env, round_id: String, delegator: Address) {
        delegator.require_auth();

        // Round must be active to revoke
        let round_key = DataKey::RoundStatus(round_id.clone());
        let round_status: RoundStatus = e.storage().persistent().get(&round_key)
            .unwrap_or_else(|| panic!("Round not found"));
        if round_status != RoundStatus::Active {
            panic!("Round is closed, cannot revoke delegation");
        }

        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id: round_id.clone(),
            delegator: delegator.clone(),
        });

        if !e.storage().persistent().has(&delegation_key) {
            panic!("No active delegation found");
        }

        e.storage().persistent().remove(&delegation_key);

        e.events().publish(
            (symbol_short!("Revoke"), round_id, delegator),
            0_u32,
        );
    }

    // ── View Functions ──────────────────────────────────────────────────

    /// Get the delegatee for a given delegator in a round.
    /// Returns the delegatee address, or panics if no delegation exists.
    pub fn get_delegation(e: Env, round_id: String, delegator: Address) -> Address {
        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id,
            delegator,
        });

        e.storage().persistent().get(&delegation_key)
            .unwrap_or_else(|| panic!("No delegation found"))
    }

    /// Check if a delegator has an active delegation in a round.
    pub fn has_delegation(e: Env, round_id: String, delegator: Address) -> bool {
        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id,
            delegator,
        });

        e.storage().persistent().has(&delegation_key)
    }

    /// Get the round status.
    pub fn get_round_status(e: Env, round_id: String) -> u32 {
        let key = DataKey::RoundStatus(round_id);
        match e.storage().persistent().get::<_, RoundStatus>(&key) {
            Some(RoundStatus::Active) => 1,
            Some(RoundStatus::Closed) => 2,
            None => 0, // Not found
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test;
