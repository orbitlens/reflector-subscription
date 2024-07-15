use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

// The configuration parameters for the contract.
pub struct ContractConfig {
    // The admin address.
    pub admin: Address,
    // The base asset for the prices.
    pub token: Address,
    // The base fee for the contract.
    pub fee: u64,
}
