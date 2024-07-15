#![allow(non_upper_case_globals)]
use soroban_sdk::storage::{Instance, Persistent};
use soroban_sdk::{panic_with_error, Address, Env};

use crate::types;

use types::{error::Error, subscription::Subscription};
const ADMIN_KEY: &str = "admin";
const BASE_FEE: &str = "base_fee";
const LAST_SUBSCRIPTION_ID: &str = "last";
const TOKEN_KEY: &str = "token";

pub trait EnvExtensions {
    fn get_admin(&self) -> Option<Address>;

    fn set_admin(&self, admin: &Address);

    fn get_fee(&self) -> u64;

    fn set_fee(&self, base_fee: u64);

    fn get_token(&self) -> Address;

    fn set_token(&self, token: &Address);

    fn get_last_subscription_id(&self) -> u64;

    fn set_last_subscription_id(&self, last_subscription_id: u64);

    fn get_subscription(&self, subscription_id: u64) -> Option<Subscription>;

    fn set_subscription(&self, subscription_id: u64, subscription: &Subscription);

    fn panic_if_not_admin(&self);

    fn is_initialized(&self) -> bool;
}

impl EnvExtensions for Env {
    fn is_initialized(&self) -> bool {
        get_instance_storage(&self).has(&ADMIN_KEY)
    }

    fn get_admin(&self) -> Option<Address> {
        get_instance_storage(&self).get(&ADMIN_KEY)
    }

    fn set_admin(&self, admin: &Address) {
        get_instance_storage(&self).set(&ADMIN_KEY, admin);
    }

    fn get_fee(&self) -> u64 {
        get_instance_storage(&self).get(&BASE_FEE).unwrap_or(0)
    }

    fn set_fee(&self, base_fee: u64) {
        get_instance_storage(&self).set(&BASE_FEE, &base_fee);
    }

    fn get_token(&self) -> Address {
        get_instance_storage(&self).get(&TOKEN_KEY).unwrap()
    }

    fn set_token(&self, token: &Address) {
        get_instance_storage(&self).set(&TOKEN_KEY, token);
    }

    fn get_last_subscription_id(&self) -> u64 {
        get_instance_storage(&self)
            .get(&LAST_SUBSCRIPTION_ID)
            .unwrap_or(0)
    }

    fn set_last_subscription_id(&self, last_subscription_id: u64) {
        get_instance_storage(&self).set(&LAST_SUBSCRIPTION_ID, &last_subscription_id);
    }

    fn get_subscription(&self, subscription_id: u64) -> Option<Subscription> {
        get_persistent_storage(&self).get(&subscription_id)
    }

    fn set_subscription(&self, subscription_id: u64, subscription: &Subscription) {
        get_persistent_storage(&self).set(&subscription_id, subscription);
    }

    fn panic_if_not_admin(&self) {
        let admin = self.get_admin();
        if admin.is_none() {
            panic_with_error!(self, Error::Unauthorized);
        }
        admin.unwrap().require_auth()
    }
}

fn get_instance_storage(e: &Env) -> Instance {
    e.storage().instance()
}

fn get_persistent_storage(e: &Env) -> Persistent {
    e.storage().persistent()
}
