#![cfg(test)]

use crate::{CollectiveInvestmentToken, CollectiveInvestmentTokenClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

#[test]
fn test_initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, CollectiveInvestmentToken);
    let client = CollectiveInvestmentTokenClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let usdc_admin = Address::generate(&e);
    let usdc_token = e.register_stellar_asset_contract(usdc_admin);
    let mint_target = Address::generate(&e);

    client.initialize(&admin, &usdc_token, &mint_target);

    let balance = client.get_user_token_balance(&mint_target);
    assert_eq!(balance, 10_000_000_000_000_i128);
}

#[test]
fn test_mint_additional_tokens() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, CollectiveInvestmentToken);
    let client = CollectiveInvestmentTokenClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let usdc_admin = Address::generate(&e);
    let usdc_token = e.register_stellar_asset_contract(usdc_admin);
    let mint_target = Address::generate(&e);

    client.initialize(&admin, &usdc_token, &mint_target);

    let new_target = Address::generate(&e);
    client.mint_additional_tokens(&new_target, &500_000_000_i128);

    assert_eq!(client.get_user_token_balance(&new_target), 500_000_000_i128);
}

#[test]
fn test_proposal_voting_execution() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, CollectiveInvestmentToken);
    let client = CollectiveInvestmentTokenClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let usdc_admin = Address::generate(&e);
    let usdc_token_id = e.register_stellar_asset_contract(usdc_admin.clone());
    let usdc_token = token::Client::new(&e, &usdc_token_id);
    let usdc_admin_client = token::StellarAssetClient::new(&e, &usdc_token_id);

    let user = Address::generate(&e);

    client.initialize(&admin, &usdc_token_id, &user);

    usdc_admin_client.mint(&contract_id, &10_000_000_000_i128);

    let vote_amount = 20_000_000_i128;
    let payout_amount = 5_000_000_000_i128;
    let payout_target = Address::generate(&e);
    let proposal_id = 1_u64;

    assert_eq!(
        client.get_user_token_balance(&user),
        10_000_000_000_000_i128
    );
    assert_eq!(usdc_token.balance(&payout_target), 0_i128);

    client.execute_proposal_vote(
        &user,
        &proposal_id,
        &vote_amount,
        &payout_target,
        &payout_amount,
    );

    let expected_balance = 10_000_000_000_000_i128 - 10_000_000_i128;
    assert_eq!(client.get_user_token_balance(&user), expected_balance);

    assert_eq!(usdc_token.balance(&payout_target), payout_amount);

    let stats = client.get_proposal_stats(&proposal_id);
    assert_eq!(stats.executed, true);
    assert_eq!(stats.total_votes, vote_amount);

    let contrib = client.get_contribution_stats(&proposal_id, &user);
    assert_eq!(contrib, vote_amount);
}

#[test]
#[should_panic(expected = "Proposal already executed")]
fn test_double_execution_prevention() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, CollectiveInvestmentToken);
    let client = CollectiveInvestmentTokenClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    let usdc_token_id = e.register_stellar_asset_contract(admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&e, &usdc_token_id);

    let user = Address::generate(&e);
    client.initialize(&admin, &usdc_token_id, &user);
    token_admin_client.mint(&contract_id, &1000_i128);

    client.execute_proposal_vote(&user, &1, &1000_i128, &admin, &10_i128);

    client.execute_proposal_vote(&user, &1, &1000_i128, &admin, &10_i128);
}
