use crate::{utils, Account, Error};
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ChainSpec {
    /// Id of the chain
    chain_id: u64,
    /// Preallocations for accounts
    accounts: HashMap<Address, Account>,
}

impl ChainSpec {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|e| e.into())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(data).map_err(|e| e.into())
    }

    pub fn iter_accounts(&self) -> std::collections::hash_map::Iter<'_, Address, Account> {
        self.accounts.iter()
    }
}

impl Default for ChainSpec {
    fn default() -> Self {
        let account1 = Account::new(100_000_000, 0);
        let account2 = Account::new(100_000_000, 0);
        let account3 = Account::new(100_000_000, 0);

        let pk1 = utils::u256_to_signing_key(&U256::from(1)).unwrap();
        let pk2 = utils::u256_to_signing_key(&U256::from(2)).unwrap();
        let pk3 = utils::u256_to_signing_key(&U256::from(3)).unwrap();

        let addr1 = utils::addr(&pk1);
        let addr2 = utils::addr(&pk2);
        let addr3 = utils::addr(&pk3);

        let mut map = HashMap::new();
        map.insert(addr1, account1);
        map.insert(addr2, account2);
        map.insert(addr3, account3);

        Self {
            chain_id: 1,
            accounts: map,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_serialize_of_chainspec() {
        let mut map = HashMap::new();
        map.insert(
            Address::from_str("0x0002938490238409382402938490238409382400").unwrap(),
            Account::new(10000, 0),
        );

        map.insert(
            Address::from_str("0x1112938490238409382402938490238409382400").unwrap(),
            Account::new(10000, 0),
        );

        let spec = ChainSpec {
            accounts: map,
            chain_id: 1,
        };

        let serialized = spec.serialize().unwrap();

        let deserialized = ChainSpec::deserialize(&serialized).unwrap();

        assert_eq!(spec, deserialized);
    }
}
