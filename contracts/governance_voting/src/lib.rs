#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,

    Delegation(DelegationKey),

    RoundStatus(String),
}

#[contracttype]
#[derive(Clone)]
pub struct DelegationKey {
    pub round_id: String,
    pub delegator: Address,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum RoundStatus {
    Active,
    Closed,
}

#[contract]
pub struct GovernanceVotingContract;

#[contractimpl]
impl GovernanceVotingContract {
    pub fn initialize(e: Env, admin: Address) {
        if e.storage().persistent().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        e.storage().persistent().set(&DataKey::Admin, &admin);
    }

    pub fn create_round(e: Env, round_id: String) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::RoundStatus(round_id.clone());
        if e.storage().persistent().has(&key) {
            panic!("Round already exists");
        }

        e.storage().persistent().set(&key, &RoundStatus::Active);
        e.events()
            .publish((symbol_short!("RndOpen"), round_id), 0_u32);
    }

    pub fn close_round(e: Env, round_id: String) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::RoundStatus(round_id.clone());
        let status: RoundStatus = e
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Round not found"));

        if status == RoundStatus::Closed {
            panic!("Round already closed");
        }

        e.storage().persistent().set(&key, &RoundStatus::Closed);
        e.events()
            .publish((symbol_short!("RndClose"), round_id), 0_u32);
    }

    pub fn delegate(e: Env, round_id: String, delegator: Address, delegatee: Address) {
        delegator.require_auth();

        let round_key = DataKey::RoundStatus(round_id.clone());
        let round_status: RoundStatus = e
            .storage()
            .persistent()
            .get(&round_key)
            .unwrap_or_else(|| panic!("Round not found"));
        if round_status != RoundStatus::Active {
            panic!("Round is not active");
        }

        if delegator == delegatee {
            panic!("Cannot delegate to yourself");
        }

        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id: round_id.clone(),
            delegator: delegator.clone(),
        });
        if e.storage().persistent().has(&delegation_key) {
            panic!("Already delegated in this round");
        }

        let mut current = delegatee.clone();
        let max_depth: u32 = 10;
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
                None => break,
            }
        }

        e.storage().persistent().set(&delegation_key, &delegatee);

        e.events()
            .publish((symbol_short!("Deleg"), round_id, delegator), delegatee);
    }

    pub fn revoke_delegation(e: Env, round_id: String, delegator: Address) {
        delegator.require_auth();

        let round_key = DataKey::RoundStatus(round_id.clone());
        let round_status: RoundStatus = e
            .storage()
            .persistent()
            .get(&round_key)
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

        e.events()
            .publish((symbol_short!("Revoke"), round_id, delegator), 0_u32);
    }

    pub fn get_delegation(e: Env, round_id: String, delegator: Address) -> Address {
        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id,
            delegator,
        });

        e.storage()
            .persistent()
            .get(&delegation_key)
            .unwrap_or_else(|| panic!("No delegation found"))
    }

    pub fn has_delegation(e: Env, round_id: String, delegator: Address) -> bool {
        let delegation_key = DataKey::Delegation(DelegationKey {
            round_id,
            delegator,
        });

        e.storage().persistent().has(&delegation_key)
    }

    pub fn get_round_status(e: Env, round_id: String) -> u32 {
        let key = DataKey::RoundStatus(round_id);
        match e.storage().persistent().get::<_, RoundStatus>(&key) {
            Some(RoundStatus::Active) => 1,
            Some(RoundStatus::Closed) => 2,
            None => 0,
        }
    }
}

#[cfg(test)]
mod test;
