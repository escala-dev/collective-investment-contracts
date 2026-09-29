#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, token, String, BytesN};

// Interface for Custom Token Admin functions (like mint)
#[soroban_sdk::contractclient(name = "TokenAdminClient")]
pub trait TokenAdminInterface {
    fn mint(e: Env, to: Address, amount: i128);
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,             // Address
    USDCAddress,       // Address
    TokenAddress,      // Address (Contract A)
    EmissionConfig,    // EmissionConfig
    Proposal(String),  // ProposalStats - UPDATED TO STRING
    UserContribution(UserContributionKey), // i128
}

#[contracttype]
#[derive(Clone)]
pub struct EmissionConfig {
    pub current_supply: i128, // Tracking simplified logic, or we can query token::total_supply()
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
    pub proposal_id: String, // UPDATED TO STRING
    pub user: Address,
}

#[contract]
pub struct CollectiveVault;

#[contractimpl]
impl CollectiveVault {
    
    // --- Initialization ---

    // Now receiving the ALREADY DEPLOYED 'token_contract' address.
    // Flow:
    // 1. Deploy Token Contract (A)
    // 2. Deploy This Contract (B)
    // 3. Initialize Repartition/Minting on A (set Admin to B?)
    // 4. Initialize This B (passing A's address)
    pub fn initialize(
        e: Env, 
        admin: Address, 
        usdc_token: Address, 
        token_contract: Address
    ) {
        if e.storage().persistent().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        e.storage().persistent().set(&DataKey::Admin, &admin);
        e.storage().persistent().set(&DataKey::USDCAddress, &usdc_token);
        e.storage().persistent().set(&DataKey::TokenAddress, &token_contract);

        // Optional: We can track supply locally if needed, or rely on token contract
        let config = EmissionConfig {
            current_supply: 0, 
            emission_rate: 0,
        };
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);
    }

    // --- Upgrade ---
    // Allows admin to upgrade contract WASM in-place, preserving all state
    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        e.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    // --- Token Management (Via External Contract) ---

    // Admin sets rate locally (business rule)
    pub fn set_emission_rate(e: Env, new_rate: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut config: EmissionConfig = e.storage().persistent().get(&DataKey::EmissionConfig).unwrap();
        config.emission_rate = new_rate;
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);
    }

    // Admin triggers minting on the external Token Contract
    // Requirement: This Contract (B) must be the ADMIN of Token Contract (A).
    pub fn mint_additional_tokens(e: Env, to: Address, amount: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth(); // Only Admin can trigger the minting logic in Vault

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        // 1. Update Local Config (optional tracking)
        let mut config: EmissionConfig = e.storage().persistent().get(&DataKey::EmissionConfig).unwrap();
        config.current_supply += amount;
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);

        // 2. Call External Token Mint
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        // Use the Admin Client we defined
        let client = TokenAdminClient::new(&e, &token_addr);
        
        // This fails if CollectiveVault is not the Admin/Minter of token_addr
        client.mint(&to, &amount); 

        // Emit Event
        e.events().publish((symbol_short!("Mint"), to.clone()), amount);
    }

    // Allows users to deposit USDC and automatically receive minted Participation Tokens
    pub fn deposit_usdc(e: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        // 1. Transfer USDC from user to this vault
        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.transfer(&user, &e.current_contract_address(), &amount);

        // 2. Update Local Config (optional tracking)
        let mut config: EmissionConfig = e.storage().persistent().get(&DataKey::EmissionConfig).unwrap();
        config.current_supply += amount;
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);

        // 3. Call External Token Mint
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let client = TokenAdminClient::new(&e, &token_addr);
        client.mint(&user, &amount);

        // Emit Event
        e.events().publish((symbol_short!("Deposit"), user.clone()), amount);
    }

    // Allows Admin to withdraw funds if the Vault itself holds the balance (Treasury Mode)
    // This moves tokens FROM the Vault Contract's address TO the recipient.
    pub fn admin_withdraw_tokens(e: Env, to: Address, amount: i128) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if amount <= 0 {
             panic!("Amount must be positive");
        }

        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);

        // Vault is the 'from' address.
        // Execution acts as authorization for the contract's own address.
        token_client.transfer(&e.current_contract_address(), &to, &amount);
        
        e.events().publish((symbol_short!("Withdraw"), to), amount);
    }

    // --- Core Logic: Voting & Execution ---

    // --- Core Logic: Voting & Execution ---

    // 1. Vote: Users TRANSFER their tokens to the Vault (Locking them)
    pub fn vote_proposal(
        e: Env, 
        voter: Address, 
        proposal_id: String, 
        vote_amount: i128
    ) {
        voter.require_auth();

        if vote_amount <= 0 {
            panic!("Vote amount must be positive");
        }

        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);

        // LOCKING: Transfer from User -> Vault
        token_client.transfer(&voter, &e.current_contract_address(), &vote_amount);

        // User contribution tracking
        let user_key = UserContributionKey { proposal_id: proposal_id.clone(), user: voter.clone() };
        let user_contribution = e.storage().persistent().get(&DataKey::UserContribution(user_key.clone())).unwrap_or(0_i128);
        e.storage().persistent().set(&DataKey::UserContribution(user_key), &(user_contribution + vote_amount));

        // Update Proposal Stats
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
             stats.total_voters += 1; // New voter
        }
        e.storage().persistent().set(&key, &stats);

        e.events().publish((symbol_short!("DbVotes"), proposal_id.clone()), stats.total_votes);

        e.events().publish((symbol_short!("Vote"), proposal_id, voter), vote_amount);
    }

    // 2. Execute: Admin triggers the payout AND BURNS the locked tokens
    pub fn execute_proposal(
        e: Env,
        proposal_id: String,
        usdc_payout_address: Address,
        usdc_amount: i128
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
        
        // FINANCIAL VALIDATION: 1 Token Transfered >= 1 USDC Payout
        if stats.total_votes < usdc_amount {
             panic!("Insufficient funds raised (votes < amount)");
        }

        // BURN LOGIC: Now we burn the tokens that were locked in the Vault
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        
        // We burn ALL tokens voted for this proposal (or just the amount verified?)
        // To be clean, we burn exactly what was voted/locked for this proposal.
        token_client.burn(&e.current_contract_address(), &stats.total_votes);

        // Update Supply Tracking (Now they are truly gone)
        let mut config: EmissionConfig = e.storage().persistent().get(&DataKey::EmissionConfig).unwrap();
        config.current_supply -= stats.total_votes;
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);

        // Payout USDC
        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        
        usdc_client.transfer(&e.current_contract_address(), &usdc_payout_address, &usdc_amount);

        // Update State
        stats.executed = true;
        stats.usdc_amount_approved = usdc_amount;
        e.storage().persistent().set(&key, &stats);

        e.events().publish(
            (symbol_short!("Exec"), proposal_id), 
            usdc_amount
        );
    }

    // 3. Refund: Admin returns locked tokens to a voter when proposal is rejected
    pub fn refund_proposal(
        e: Env,
        proposal_id: String,
        voter: Address
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
            panic!("Proposal already executed, cannot refund");
        }

        // Read voter's contribution
        let user_key = UserContributionKey { proposal_id: proposal_id.clone(), user: voter.clone() };
        let contribution: i128 = e.storage().persistent().get(&DataKey::UserContribution(user_key.clone())).unwrap_or(0);

        if contribution <= 0 {
            panic!("No contribution found for this voter");
        }

        // Transfer tokens back from Vault to voter
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.transfer(&e.current_contract_address(), &voter, &contribution);

        // Clear the contribution record
        e.storage().persistent().set(&DataKey::UserContribution(user_key), &0_i128);

        // Update proposal stats
        stats.total_votes -= contribution;
        if stats.total_voters > 0 {
            stats.total_voters -= 1;
        }
        e.storage().persistent().set(&key, &stats);

        e.events().publish((symbol_short!("Refund"), proposal_id, voter), contribution);
    }

    // 4. Execute Partial: Burns a portion of locked tokens and pays proportional USDC (for milestones)
    pub fn execute_partial(
        e: Env,
        proposal_id: String,
        usdc_payout_address: Address,
        usdc_amount: i128,
        burn_amount: i128
    ) {
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if usdc_amount <= 0 || burn_amount <= 0 {
            panic!("Amounts must be positive");
        }

        // Burn the proportional share of locked tokens from the vault
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.burn(&e.current_contract_address(), &burn_amount);

        // Update supply tracking
        let mut config: EmissionConfig = e.storage().persistent().get(&DataKey::EmissionConfig).unwrap();
        config.current_supply -= burn_amount;
        e.storage().persistent().set(&DataKey::EmissionConfig, &config);

        // Pay USDC to the payout address (treasurer wallet)
        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.transfer(&e.current_contract_address(), &usdc_payout_address, &usdc_amount);

        e.events().publish((symbol_short!("Partial"), proposal_id), usdc_amount);
    }

    // --- View Functions ---

    pub fn get_proposal_stats(e: Env, proposal_id: String) -> ProposalStats {
        e.storage().persistent().get(&DataKey::Proposal(proposal_id)).unwrap_or(ProposalStats {
            total_votes: 0,
            total_voters: 0,
            executed: false,
            usdc_amount_approved: 0,
        })
    }

    // Helper to get address of the token this vault manages
    pub fn get_token_address(e: Env) -> Address {
        e.storage().persistent().get(&DataKey::TokenAddress).unwrap()
    }

    pub fn get_contribution_stats(e: Env, proposal_id: String, user: Address) -> i128 {
        let key = UserContributionKey { proposal_id, user };
        e.storage().persistent().get(&DataKey::UserContribution(key)).unwrap_or(0)
    }

    // Returns the USDC balance held by the Vault contract itself
    pub fn get_vault_usdc_balance(e: Env) -> i128 {
        let usdc_address: Address = e.storage().persistent().get(&DataKey::USDCAddress).unwrap();
        let usdc_client = token::Client::new(&e, &usdc_address);
        usdc_client.balance(&e.current_contract_address())
    }

    // Returns the PT token balance held by the Vault contract itself
    pub fn get_vault_token_balance(e: Env) -> i128 {
        let token_addr: Address = e.storage().persistent().get(&DataKey::TokenAddress).unwrap();
        let token_client = token::Client::new(&e, &token_addr);
        token_client.balance(&e.current_contract_address())
    }
}

#[cfg(test)]
mod test;
