#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String};

#[contract]
pub struct StandardToken;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Allowance(AllowanceDataKey),
    Balance(Address),
    Name,
    Symbol,
    Decimals,
}

#[contracttype]
#[derive(Clone)]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contractimpl]
impl StandardToken {
    pub fn initialize(e: Env, admin: Address, decimal: u32, name: String, symbol: String) {
        if e.storage().persistent().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        e.storage().persistent().set(&DataKey::Admin, &admin);
        e.storage().persistent().set(&DataKey::Name, &name);
        e.storage().persistent().set(&DataKey::Symbol, &symbol);
        e.storage().persistent().set(&DataKey::Decimals, &decimal);
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        if amount < 0 {
            panic!("negative amount");
        }
        let admin: Address = e.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0_i128);
        e.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &(balance + amount));

        e.events()
            .publish((symbol_short!("mint"), admin, to), amount);
    }

    pub fn burn(e: Env, from: Address, amount: i128) {
        if amount < 0 {
            panic!("negative amount");
        }
        from.require_auth();

        let balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0_i128);
        if balance < amount {
            panic!("insufficient balance");
        }
        e.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));

        e.events().publish((symbol_short!("burn"), from), amount);
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        if amount < 0 {
            panic!("negative amount");
        }
        from.require_auth();

        let from_balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0_i128);
        if from_balance < amount {
            panic!("insufficient balance");
        }
        e.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(from_balance - amount));

        let to_balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0_i128);
        e.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &(to_balance + amount));

        e.events()
            .publish((symbol_short!("transfer"), from, to), amount);
    }

    pub fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        if amount < 0 {
            panic!("negative amount");
        }
        from.require_auth();

        let key = DataKey::Allowance(AllowanceDataKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        e.storage().persistent().set(
            &key,
            &AllowanceValue {
                amount,
                expiration_ledger,
            },
        );

        e.events()
            .publish((symbol_short!("approve"), from, spender), amount);
    }

    pub fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        let key = DataKey::Allowance(AllowanceDataKey { from, spender });
        let val: AllowanceValue = e
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(AllowanceValue {
                amount: 0,
                expiration_ledger: 0,
            });
        if val.expiration_ledger > 0 && e.ledger().sequence() > val.expiration_ledger {
            0
        } else {
            val.amount
        }
    }

    pub fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        if amount < 0 {
            panic!("negative amount");
        }
        spender.require_auth();

        let key = DataKey::Allowance(AllowanceDataKey {
            from: from.clone(),
            spender: spender.clone(),
        });
        let val: AllowanceValue = e
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(AllowanceValue {
                amount: 0,
                expiration_ledger: 0,
            });

        if val.expiration_ledger > 0 && e.ledger().sequence() > val.expiration_ledger {
            panic!("allowance expired");
        }
        if val.amount < amount {
            panic!("insufficient allowance");
        }

        let from_balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0_i128);
        if from_balance < amount {
            panic!("insufficient balance");
        }

        e.storage().persistent().set(
            &key,
            &AllowanceValue {
                amount: val.amount - amount,
                expiration_ledger: val.expiration_ledger,
            },
        );

        e.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(from_balance - amount));
        let to_balance = e
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0_i128);
        e.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &(to_balance + amount));

        e.events()
            .publish((symbol_short!("transfer"), from, to), amount);
    }

    pub fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn name(e: Env) -> String {
        e.storage().persistent().get(&DataKey::Name).unwrap()
    }

    pub fn symbol(e: Env) -> String {
        e.storage().persistent().get(&DataKey::Symbol).unwrap()
    }

    pub fn decimals(e: Env) -> u32 {
        e.storage().persistent().get(&DataKey::Decimals).unwrap()
    }
}
