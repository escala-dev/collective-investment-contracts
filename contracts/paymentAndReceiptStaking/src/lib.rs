#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, log, Address, Env};
use soroban_sdk::{token, String, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Init,
    Balance,
    Intallemt,
    ApiKey,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentInfo {
    wallet: Address,
    amount: i128,
    date: u64,
    status: bool,
    period: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct InstallmentStruct {
    installements_id: String,
    amount: i128,
    period: u64,
    status: String,
    date: u64,
}

#[contract]
pub struct PaymentAndReceiptContract;

#[contractimpl]
impl PaymentAndReceiptContract {
    pub fn initialize(env: Env, api_key: String) {
        if env.storage().instance().has(&DataKey::Init) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::ApiKey, &api_key);
        env.storage().instance().set(&DataKey::Init, &true);
    }

    fn assert_api_key(env: &Env, provided: String) {
        let expected = env
            .storage()
            .instance()
            .get::<_, String>(&DataKey::ApiKey)
            .expect("API key not set");

        if expected != provided {
            panic!("Invalid API key");
        }
    }

    pub fn installment_save_data(
        env: Env,
        installements_id: String,
        amount: i128,
        period: u64,
        date: u64,
    ) {
        let status: String = String::from_str(&env, "pending");
        let installment_info = InstallmentStruct {
            installements_id,
            amount,
            period,
            status,
            date,
        };

        let mut payments: Vec<InstallmentStruct> = env
            .storage()
            .instance()
            .get(&DataKey::Intallemt)
            .unwrap_or(Vec::new(&env));

        payments.push_front(installment_info);
        env.storage().instance().set(&DataKey::Intallemt, &payments);
    }

    pub fn payment_receipt_contract(env: Env, installements_id: String, amount: i128) {
        let status = String::from_str(&env, "completed");

        let mut items: Vec<InstallmentStruct> = env
            .storage()
            .instance()
            .get(&DataKey::Intallemt)
            .unwrap_or(Vec::new(&env));

        for i in 0..items.len() {
            if let Some(mut item) = items.get(i) {
                if item.installements_id == installements_id && item.amount == amount {
                    item.status = status.clone();
                    items.set(i, item);
                }
            }
        }

        env.storage().instance().set(&DataKey::Intallemt, &items);
    }

    pub fn get_installment(env: &Env) -> Vec<InstallmentStruct> {
        env.storage()
            .instance()
            .get(&DataKey::Intallemt)
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_installments_by_u64(env: &Env, period: u64) -> Vec<InstallmentStruct> {
        let installments = env
            .storage()
            .instance()
            .get::<_, Vec<InstallmentStruct>>(&DataKey::Intallemt)
            .unwrap_or(Vec::new(env));

        let mut filtered = Vec::new(env);
        for inst in installments.iter() {
            if inst.period == period {
                filtered.push_front(inst);
            }
        }
        filtered
    }

    pub fn get_payment_info(env: Env) -> Vec<PaymentInfo> {
        env.storage()
            .instance()
            .get(&DataKey::Balance)
            .unwrap_or(Vec::new(&env))
    }

    pub fn promissory_payment(
        env: Env,
        api_key: String,
        wallet: Address,
        amount: i128,
        date: u64,
        status: bool,
        period: u64,
    ) {
        Self::assert_api_key(&env, api_key);

        let payment = PaymentInfo {
            wallet,
            amount,
            date,
            status,
            period,
        };

        let mut payments: Vec<PaymentInfo> = env
            .storage()
            .instance()
            .get(&DataKey::Balance)
            .unwrap_or(Vec::new(&env));

        payments.push_front(payment);
        env.storage().instance().set(&DataKey::Balance, &payments);
    }

    pub fn promissory_payment_check(env: Env, api_key: String, period: u64) {
        Self::assert_api_key(&env, api_key);

        let mut payments: Vec<PaymentInfo> = env
            .storage()
            .instance()
            .get(&DataKey::Balance)
            .unwrap_or(Vec::new(&env));

        for i in 0..payments.len() {
            if let Some(mut payment) = payments.get(i) {
                if payment.period == period && !payment.status {
                    Self::send_payment(&env, payment.wallet.clone(), payment.amount.clone());
                    payment.status = true;
                    payments.set(i, payment);
                }
            }
        }

        env.storage().instance().set(&DataKey::Balance, &payments);
    }

    fn send_payment(env: &Env, to: Address, amount: i128) {
        let token_address_str = String::from_str(
            &env,
            "CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA",
        );
        let token_address: Address = Address::from_string(&token_address_str);
        let client: token::TokenClient = token::Client::new(&env, &token_address);

        client.transfer(&env.current_contract_address(), &to, &amount);

        let now = env.ledger().timestamp();
        log!(
            &env,
            "Sent {} from contract to {} at {}",
            amount,
            to,
            now
        );
    }
}

mod test;
