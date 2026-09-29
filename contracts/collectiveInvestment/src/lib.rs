#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env, String,
};

#[soroban_sdk::contractclient(name = "TokenAdminClient")]
pub trait TokenAdminInterface {
    fn mint(e: Env, to: Address, amount: i128);
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    USDCAddress,
    TokenAddress,
    EmissionConfig,
    Proposal(String),
    UserContribution(UserContributionKey),
}

#[contracttype]
#[derive(Clone)]
pub struct EmissionConfig {
    pub current_supply: i128,
    pub emission_rate: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct ProposalStats {
    pub total_votes: i128,
    pub total_voters: u32,
    pub executed: bool,
    pub usdc_amount_approved: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct UserContributionKey {
    pub proposal_id: String,
    pub user: Address,
}

#[contract]
pub struct CollectiveVault;

#[contractimpl]
impl CollectiveVault {
    pub fn initialize(e: Env, admin: Address, usdc_token: Address, token_contract: Address) {
        if e.storage().persistent().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        e.storage().persistent().set(&DataKey::Admin, &admin);
        e.storage()
            .persistent()
            .set(&DataKey::USDCAddress, &usdc_token);
        e.storage()
            .persistent()
            .set(&DataKey::TokenAddress, &token_contract);

        let config = EmissionConfig {
            current_supply: 0,
            emission_rate: 0,
        };
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);
    }

    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        e.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    pub fn set_emission_rate(e: Env, new_rate: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut config: EmissionConfig = e
            .storage()
            .persistent()
            .get(&DataKey::EmissionConfig)
            .unwrap();
        config.emission_rate = new_rate;
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);
    }

    pub fn mint_additional_tokens(e: Env, to: Address, amount: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let mut config: EmissionConfig = e
            .storage()
            .persistent()
            .get(&DataKey::EmissionConfig)
            .unwrap();
        config.current_supply += amount;
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();

        let client = TokenAdminClient::new(&e, &token_addr);

        client.mint(&to, &amount);

        e.events()
            .publish((symbol_short!("Mint"), to.clone()), amount);
    }

    pub fn deposit_usdc(e: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.transfer(&user, &e.current_contract_address(), &amount);

        let mut config: EmissionConfig = e
            .storage()
            .persistent()
            .get(&DataKey::EmissionConfig)
            .unwrap();
        config.current_supply += amount;
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let client = TokenAdminClient::new(&e, &token_addr);
        client.mint(&user, &amount);

        e.events()
            .publish((symbol_short!("Deposit"), user.clone()), amount);
    }

    pub fn admin_withdraw_tokens(e: Env, to: Address, amount: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);

        token_client.transfer(&e.current_contract_address(), &to, &amount);

        e.events().publish((symbol_short!("Withdraw"), to), amount);
    }

    pub fn vote_proposal(e: Env, voter: Address, proposal_id: String, vote_amount: i128) {
        voter.require_auth();

        if vote_amount <= 0 {
            panic!("Vote amount must be positive");
        }

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);

        token_client.transfer(&voter, &e.current_contract_address(), &vote_amount);

        let user_key = UserContributionKey {
            proposal_id: proposal_id.clone(),
            user: voter.clone(),
        };
        let user_contribution = e
            .storage()
            .persistent()
            .get(&DataKey::UserContribution(user_key.clone()))
            .unwrap_or(0_i128);
        e.storage().persistent().set(
            &DataKey::UserContribution(user_key),
            &(user_contribution + vote_amount),
        );

        let key = DataKey::Proposal(proposal_id.clone());
        let mut stats = e.storage().persistent().get(&key).unwrap_or(ProposalStats {
            total_votes: 0,
            total_voters: 0,
            executed: false,
            usdc_amount_approved: 0,
        });

        if stats.executed {
            panic!("Proposal already executed");
        }

        stats.total_votes += vote_amount;
        if user_contribution == 0 {
            stats.total_voters += 1;
        }
        e.storage().persistent().set(&key, &stats);

        e.events().publish(
            (symbol_short!("DbVotes"), proposal_id.clone()),
            stats.total_votes,
        );

        e.events()
            .publish((symbol_short!("Vote"), proposal_id, voter), vote_amount);
    }

    pub fn execute_proposal(
        e: Env,
        proposal_id: String,
        usdc_payout_address: Address,
        usdc_amount: i128,
    ) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::Proposal(proposal_id.clone());
        let mut stats = e.storage().persistent().get(&key).unwrap_or(ProposalStats {
            total_votes: 0,
            total_voters: 0,
            executed: false,
            usdc_amount_approved: 0,
        });

        if stats.executed {
            panic!("Proposal already executed");
        }

        if stats.total_votes < usdc_amount {
            panic!("Insufficient funds raised (votes < amount)");
        }

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);

        token_client.burn(&e.current_contract_address(), &stats.total_votes);

        let mut config: EmissionConfig = e
            .storage()
            .persistent()
            .get(&DataKey::EmissionConfig)
            .unwrap();
        config.current_supply -= stats.total_votes;
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);

        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);

        usdc_client.transfer(
            &e.current_contract_address(),
            &usdc_payout_address,
            &usdc_amount,
        );

        stats.executed = true;
        stats.usdc_amount_approved = usdc_amount;
        e.storage().persistent().set(&key, &stats);

        e.events()
            .publish((symbol_short!("Exec"), proposal_id), usdc_amount);
    }

    pub fn refund_proposal(e: Env, proposal_id: String, voter: Address) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::Proposal(proposal_id.clone());
        let mut stats = e.storage().persistent().get(&key).unwrap_or(ProposalStats {
            total_votes: 0,
            total_voters: 0,
            executed: false,
            usdc_amount_approved: 0,
        });

        if stats.executed {
            panic!("Proposal already executed, cannot refund");
        }

        let user_key = UserContributionKey {
            proposal_id: proposal_id.clone(),
            user: voter.clone(),
        };
        let contribution: i128 = e
            .storage()
            .persistent()
            .get(&DataKey::UserContribution(user_key.clone()))
            .unwrap_or(0);

        if contribution <= 0 {
            panic!("No contribution found for this voter");
        }

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.transfer(&e.current_contract_address(), &voter, &contribution);

        e.storage()
            .persistent()
            .set(&DataKey::UserContribution(user_key), &0_i128);

        stats.total_votes -= contribution;
        if stats.total_voters > 0 {
            stats.total_voters -= 1;
        }
        e.storage().persistent().set(&key, &stats);

        e.events()
            .publish((symbol_short!("Refund"), proposal_id, voter), contribution);
    }

    pub fn execute_partial(
        e: Env,
        proposal_id: String,
        usdc_payout_address: Address,
        usdc_amount: i128,
        burn_amount: i128,
    ) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if usdc_amount <= 0 || burn_amount <= 0 {
            panic!("Amounts must be positive");
        }

        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.burn(&e.current_contract_address(), &burn_amount);

        let mut config: EmissionConfig = e
            .storage()
            .persistent()
            .get(&DataKey::EmissionConfig)
            .unwrap();
        config.current_supply -= burn_amount;
        e.storage()
            .persistent()
            .set(&DataKey::EmissionConfig, &config);

        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.transfer(
            &e.current_contract_address(),
            &usdc_payout_address,
            &usdc_amount,
        );

        e.events()
            .publish((symbol_short!("Partial"), proposal_id), usdc_amount);
    }

    pub fn get_proposal_stats(e: Env, proposal_id: String) -> ProposalStats {
        e.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or(ProposalStats {
                total_votes: 0,
                total_voters: 0,
                executed: false,
                usdc_amount_approved: 0,
            })
    }

    pub fn get_token_address(e: Env) -> Address {
        e.storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap()
    }

    pub fn get_contribution_stats(e: Env, proposal_id: String, user: Address) -> i128 {
        let key = UserContributionKey { proposal_id, user };
        e.storage()
            .persistent()
            .get(&DataKey::UserContribution(key))
            .unwrap_or(0)
    }

    pub fn get_vault_usdc_balance(e: Env) -> i128 {
        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.balance(&e.current_contract_address())
    }

    pub fn get_vault_token_balance(e: Env) -> i128 {
        let token_addr: Address = e
            .storage()
            .persistent()
            .get(&DataKey::TokenAddress)
            .unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.balance(&e.current_contract_address())
    }
}

#[cfg(test)]
mod test;
