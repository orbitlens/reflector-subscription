#![no_std]

mod extensions;
mod types;

use extensions::env_extensions::EnvExtensions;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token::{self, TokenClient}, Address, BytesN, Env, Symbol, Vec};
use types::{config_data::ConfigData, create_subscription::CreateSubscription, error::Error, subscription::{self, Subscription}};

const SUBS: Symbol = symbol_short!("SUBS");

// 1 day in seconds
const DAY: u64 = 86400;

// Minimum fee factor for the activation
const MIN_FEE_FACTOR: u64 = 1;

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {

    // Admin only

    // Initializes the contract. Can be invoked only once.
    //
    // # Arguments
    //
    // * `config` - Contract configuration
    //
    // # Panics
    //
    // Panics if the contract is already initialized
    pub fn config(e: Env, config: ConfigData) {
        config.admin.require_auth();
        if e.is_initialized() {
            e.panic_with_error(Error::AlreadyInitialized);
        }

        e.set_admin(&config.admin);
        e.set_fee(config.fee);
        e.set_token(&config.token);
        e.set_last_subscription_id(0);
    }

    // Sets the base fee for the contract. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `fee` - New base fee
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn set_fee(e: Env, fee: u64) {
        e.panic_if_not_admin();
        e.set_fee(fee);
    }

    // Triggers the subscription. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `subscription_ids` - Subscription IDs to trigger
    // * `is_heartbeat` - If true, the trigger is a heartbeat
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn trigger(e: Env, subscription_ids: Vec<u64>, is_heartbeat: bool) {
        e.panic_if_not_admin();
        let now = e.ledger().timestamp();
        for subscription_id in subscription_ids.iter() {
            if let Some(mut subscription) = e.get_subscription(subscription_id) {
                subscription.last_notification = now;
                e.set_subscription(subscription_id, &subscription);
            }
        }
        let event = if is_heartbeat {
            symbol_short!("heartbeat")
        } else {
            symbol_short!("triggered")
        };
        e.events().publish((SUBS, event), subscription_ids)
    }

    // Updates the contract source code. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `admin` - Admin account address
    // * `wasm_hash` - WASM hash of the contract source code
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn update_contract(e: Env, wasm_hash: BytesN<32>) {
        e.panic_if_not_admin();
        e.deployer().update_current_contract_wasm(wasm_hash)
    }

    // Withdraws funds from the contract, and updates balance of subscriptions. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `subscription_ids` - Subscription ID
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn charge(e: Env, subscription_ids: Vec<u64>) {
        e.panic_if_not_admin();
        let mut total_charge: u64 = 0;
        let now = e.ledger().timestamp();
        let fee = e.get_fee();
        for subscription_id in subscription_ids.iter() {
            if let Some(mut subscription) = e.get_subscription(subscription_id) {
                let days = (now - subscription.last_change) / DAY;
                if days == 0 {
                    continue;
                }
                let mut charge = days * fee;
                if subscription.balance < charge {
                    charge = subscription.balance;
                }
                subscription.balance -= charge;
                subscription.last_change = now;
                if subscription.balance < fee { // Deactivate the subscription if the balance is less than the fee
                    subscription.is_active = false;
                }
                e.set_subscription(subscription_id, &subscription);
                
                total_charge += charge;
            }
        }
        //Publish the events
        e.events().publish((SUBS, symbol_short!("charged")), subscription_ids);

        //Burn the tokens
        get_token_client(&e).burn(
            &e.current_contract_address(),
            &(total_charge as i128),
        );
    }

    

    // Public

    // Creates a new subscription.
    //
    // # Arguments
    //
    // * `new_subscription` - Subscription data
    // * `amount` - Initial deposit amount
    //
    // # Returns
    //
    // Subscription ID
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the amount is less than the base fee
    // Panics if the caller doesn't match the owner address
    // Panics if the token transfer fails
    // Panics if the subscription is invalid
    pub fn create_subscription(
        e: Env,
        new_subscription: CreateSubscription,
        amount: u64,
    ) -> u64 {
        panin_if_not_initialized(&e);
        // Check the authorization
        new_subscription.owner.require_auth();

        // Check the amount
        let fee = e.get_fee();
        if amount < fee * MIN_FEE_FACTOR {
            e.panic_with_error(Error::InvalidAmount);
        }

        // Transfer the tokens to the contract
        get_token_client(&e).transfer(
            &new_subscription.owner,
            &e.current_contract_address(),
            &(amount as i128),
        );

        //todo: check if the subscription is valid and the amount is enough
        let subscription_id = e.get_last_subscription_id() + 1;
        let subscription = Subscription {
            owner: new_subscription.owner,
            asset1: new_subscription.asset1,
            asset2: new_subscription.asset2,
            threshold: new_subscription.threshold,
            heartbeat: new_subscription.heartbeat,
            webhook: new_subscription.webhook,
            balance: amount,
            is_active: true,
            last_change: e.ledger().timestamp(),
            last_notification: 0
        };
        e.set_subscription(subscription_id, &subscription);
        e.set_last_subscription_id(subscription_id);
        e.events()
            .publish((SUBS, symbol_short!("created")), subscription_id);
        return subscription_id;
    }

    // Deposits funds to the subscription.
    //
    // # Arguments
    //
    // * `from` - Sender address
    // * `subscription_id` - Subscription ID
    // * `amount` - Amount to deposit
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the amount is zero
    // Panics if the subscription does not exist
    // Panics if the token transfer fails
    pub fn deposit(e: Env, from: Address, subscription_id: u64, amount: u64) {
        panin_if_not_initialized(&e);
        from.require_auth();
        if amount == 0 {
            e.panic_with_error(Error::InvalidAmount);
        }
        let mut subscription = e.get_subscription(subscription_id).unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound));
        if !subscription.is_active && amount < e.get_fee() * MIN_FEE_FACTOR {
            e.panic_with_error(Error::InvalidAmount);
        }
        get_token_client(&e).transfer(
            &from,
            &e.current_contract_address(),
            &(amount as i128)
        );
        subscription.balance += amount;
        e.set_subscription(subscription_id, &subscription);
        e.events().publish((SUBS, symbol_short!("deposit")), (subscription_id, amount));
    }

    // Gets the subscription by ID.
    //
    // # Arguments
    //
    // * `subscription_id` - Subscription ID
    //
    // # Returns
    //
    // Subscription data
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    pub fn get_subscription(e: Env, subscription_id: u64) -> Subscription {
        panin_if_not_initialized(&e);
        e.get_subscription(subscription_id).unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound))
    }

    // Returns admin address of the contract.
    //
    // # Returns
    //
    // Contract admin account address
    pub fn admin(e: Env) -> Option<Address> {
        e.get_admin()
    }

    // Returns current protocol version of the contract.
    //
    // # Returns
    //
    // Contract protocol version
    pub fn version(_e: Env) -> u32 {
        env!("CARGO_PKG_VERSION")
            .split(".")
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    }

    // Returns the base fee of the contract.
    // 
    // # Returns
    //
    // Base fee
    pub fn fee(e: Env) -> u64 {
        panin_if_not_initialized(&e);
        e.get_fee()
    }

    // Returns the token address of the contract.
    //
    // # Returns
    //
    // Token address
    pub fn token(e: Env) -> Address {
        panin_if_not_initialized(&e);
        e.get_token()
    }
}

fn panin_if_not_initialized(e: &Env) {
    if !e.is_initialized() {
        panic_with_error!(e, Error::NotInitialized);
    }
}

fn get_token_client(e: &Env) -> TokenClient {
    TokenClient::new(e, &e.get_token())
}

mod test;