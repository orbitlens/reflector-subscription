use soroban_sdk::{contracttype, String};

use super::asset::Asset;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

// The ticker asset.
pub struct TickerAsset {
    // The admin address.
    pub asset: Asset,
    // The source of the asset
    pub source: String
}