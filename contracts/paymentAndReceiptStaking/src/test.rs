#![cfg(test)]

use super::{PaymentAndReceiptContract, PaymentAndReceiptContractClient};
use soroban_sdk::{testutils::Logs, Env, Address, BytesN};

extern crate std;

#[test]
fn test_receive_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

    //let from_address = Address::from_account(&env, &BytesN::from_array(&env, &[0; 32]));
//let amount = 1000;
   // client.receive_payment( &from_address, amount);

    let date = env.ledger().timestamp();
   // assert_eq!(env.storage().instance().get::<(Address, i64), i128>(&(from_address.clone(), date)), Some(amount));

    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_schedule_payment() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

    //let to_address = Address::from_account(&env, &BytesN::from_array(&env, &[1; 32]));
    let amount: i128 = 2000;
    let send_date = env.ledger().timestamp() + 1000;
  //  client.schedule_payment(&to_address, amount, send_date);

    //assert_eq!(env.storage().instance().get::<(Address, i64), i128>(&(to_address.clone(), send_date)), Some(amount));

    std::println!("{}", env.logs().all().join("\n"));
}

#[test]
fn test_process_scheduled_payments() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PaymentAndReceiptContract);
    let client = PaymentAndReceiptContractClient::new(&env, &contract_id);

   // let to_address = Address::from_account(&env, &BytesN::from_array(&env, &[2; 32]));
  //  let amount = 2000;
   // let send_date = env.ledger().timestamp();
  //  client.schedule_payment(&env, &to_address, amount, send_date);

   // env.ledger().increment_timestamp(1);  // Simula el paso del tiempo
//client.process_scheduled_payments(&env);

  //  assert_eq!(env.storage().instance().get::<(Address, i64), i128>(&(to_address.clone(), send_date)), None);

    std::println!("{}", env.logs().all().join("\n"));
}
