#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

#[contracttype]
pub enum DataKey {
    Oracle,
    Evidence(String),
}

#[contract]
pub struct EvidenceAttestationContract;

#[contractimpl]
impl EvidenceAttestationContract {
    pub fn initialize(e: Env, oracle: Address) {
        if e.storage().persistent().has(&DataKey::Oracle) {
            panic!("Already initialized");
        }
        e.storage().persistent().set(&DataKey::Oracle, &oracle);
    }

    pub fn attest_evidence(e: Env, evidence_id: String, sha256_hash: String) {
        let oracle: Address = e.storage().persistent().get(&DataKey::Oracle).unwrap();
        oracle.require_auth();

        if e.storage()
            .persistent()
            .has(&DataKey::Evidence(evidence_id.clone()))
        {
            panic!("Evidence already attested");
        }

        e.storage()
            .persistent()
            .set(&DataKey::Evidence(evidence_id.clone()), &sha256_hash);
        e.events()
            .publish((symbol_short!("Attest"), evidence_id), sha256_hash);
    }

    pub fn verify_evidence(e: Env, evidence_id: String, sha256_hash: String) -> bool {
        if let Some(stored_hash) = e
            .storage()
            .persistent()
            .get::<_, String>(&DataKey::Evidence(evidence_id))
        {
            return stored_hash == sha256_hash;
        }
        false
    }
}
