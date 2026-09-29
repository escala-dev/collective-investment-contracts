#![cfg(test)]

use super::{PaymentAndReceiptContract, PaymentAndReceiptContractClient};
use soroban_sdk::{testutils::Logs, Address, BytesN, Env};

extern crate std;

#[test]
fn test_receive_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

    let date = env.ledger().timestamp();

    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_schedule_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

    let amount: i128 = 2000;
    let send_date = env.ledger().timestamp() + 1000;

    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_process_scheduled_payments() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

    std::println!("{}", env.logs().all().join("\n"));
}
