use soroban_sdk::{contracttype, Address, String};

use super::ticker_asset::TickerAsset;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

// The configuration parameters for the contract.
pub struct CreateSubscription {
    // The owner address.
    pub owner: Address,
    // The asset 1.
    pub asset1: TickerAsset,
    // The asset 2.
    pub asset2: TickerAsset,
    // The threshold in percentage.
    pub threshold: u32,
    // The heartbeat in minutes.
    pub heartbeat: u32,
    // The webhook.
    pub webhook: String,
}
