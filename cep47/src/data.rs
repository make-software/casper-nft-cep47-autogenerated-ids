use std::{collections::BTreeMap, convert::TryInto};

use casper_contract::{
    contract_api::{
        runtime,
        storage::{self, new_dictionary},
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{
    bytesrepr::{FromBytes, ToBytes},
    contracts::NamedKeys,
    ApiError, CLTyped, ContractPackageHash, Key, URef, U256,
};
use cep47_logic::{events::CEP47Event, Meta, TokenId};

const BALANCES_DICT: &str = "balances";
const TOKEN_OWNERS_DICT: &str = "owners";
const TOKEN_ISSUERS_DICT: &str = "issuers";
const METADATA_DICT: &str = "metadata";
const OWNED_TOKENS_BY_INDEX_DICT: &str = "owned_tokens_by_index";
const OWNED_INDEXES_BY_TOKEN_DICT: &str = "owned_indexes_by_token";

struct Dict {
    uref: URef,
}

impl Dict {
    pub fn at(name: &str) -> Dict {
        let key: Key = runtime::get_key(name).unwrap_or_revert();
        let uref: URef = *key.as_uref().unwrap_or_revert();
        Dict { uref }
    }

    pub fn get<T: CLTyped + FromBytes>(&self, key: &str) -> Option<T> {
        storage::dictionary_get(self.uref, key)
            .unwrap_or_revert()
            .unwrap_or_default()
    }

    pub fn set<T: CLTyped + ToBytes>(&self, key: &str, value: T) {
        storage::dictionary_put(self.uref, key, Some(value));
    }

    pub fn remove<T: CLTyped + ToBytes>(&self, key: &str) {
        storage::dictionary_put(self.uref, key, Option::<T>::None);
    }
}

pub struct OwnedTokens {
    tokens_dict: Dict,
    indexes_dict: Dict,
    balances_dict: Dict,
}

impl OwnedTokens {
    pub fn instance() -> OwnedTokens {
        OwnedTokens {
            tokens_dict: Dict::at(OWNED_TOKENS_BY_INDEX_DICT),
            indexes_dict: Dict::at(OWNED_INDEXES_BY_TOKEN_DICT),
            balances_dict: Dict::at(BALANCES_DICT),
        }
    }

    pub fn get_token_by_index(&self, owner: &Key, index: &U256) -> Option<TokenId> {
        self.tokens_dict.get(&key_and_value_to_str(owner, index))
    }

    pub fn get_index_by_token(&self, owner: &Key, value: &str) -> Option<U256> {
        self.indexes_dict
            .get(&key_and_value_to_str(owner, &value.to_string()))
    }

    pub fn get_balance(&self, owner: &Key) -> U256 {
        self.balances_dict
            .get(&key_to_str(owner))
            .unwrap_or_default()
    }

    pub fn set_balance(&self, owner: &Key, value: U256) {
        self.balances_dict.set(&key_to_str(owner), value);
    }

    pub fn set_token(&self, owner: &Key, value: &TokenId) {
        let length = self.get_balance(owner);
        self.indexes_dict
            .set(&key_and_value_to_str(owner, value), length);
        self.tokens_dict
            .set(&key_and_value_to_str(owner, &length), value.clone());
        self.set_balance(owner, length + 1);
    }

    pub fn remove_token(&self, owner: &Key, value: &TokenId) {
        let length = self.get_balance(owner);
        let index = self.get_index_by_token(owner, value).unwrap_or_revert();
        match length.cmp(&(index + 1)) {
            core::cmp::Ordering::Equal => {
                self.tokens_dict
                    .remove::<TokenId>(&key_and_value_to_str(owner, &(length - 1)));
                self.set_balance(owner, length - 1);
            }
            core::cmp::Ordering::Greater => {
                let last = self.get_token_by_index(owner, &(length - 1));
                self.indexes_dict.set(
                    &key_and_value_to_str(owner, &last.clone().unwrap_or_revert()),
                    index,
                );
                self.tokens_dict.set(
                    &key_and_value_to_str(owner, &index),
                    last.unwrap_or_revert(),
                );
                self.tokens_dict
                    .remove::<TokenId>(&key_and_value_to_str(owner, &(length - 1)));
                self.set_balance(owner, length - 1);
            }
            core::cmp::Ordering::Less => {}
        }
        self.indexes_dict
            .remove::<U256>(&key_and_value_to_str(owner, value));
    }
}

pub struct Owners {
    dict: Dict,
}

impl Owners {
    pub fn instance() -> Owners {
        Owners {
            dict: Dict::at(TOKEN_OWNERS_DICT),
        }
    }

    pub fn get(&self, key: &TokenId) -> Option<Key> {
        self.dict.get(key)
    }

    pub fn set(&self, key: &TokenId, value: Key) {
        self.dict.set(key, value);
    }

    pub fn remove(&self, key: &TokenId) {
        self.dict.remove::<Key>(key);
    }
}

pub struct Issuers {
    dict: Dict,
}

impl Issuers {
    pub fn instance() -> Issuers {
        Issuers {
            dict: Dict::at(TOKEN_ISSUERS_DICT),
        }
    }

    pub fn get(&self, key: &TokenId) -> Option<Key> {
        self.dict.get(key)
    }

    pub fn set(&self, key: &TokenId, value: Key) {
        self.dict.set(key, value);
    }

    pub fn remove(&self, key: &TokenId) {
        self.dict.remove::<Key>(key);
    }
}

pub struct Metadata {
    dict: Dict,
}

impl Metadata {
    pub fn instance() -> Metadata {
        Metadata {
            dict: Dict::at(METADATA_DICT),
        }
    }

    pub fn get(&self, key: &TokenId) -> Option<Meta> {
        self.dict.get(key)
    }

    pub fn set(&self, key: &TokenId, value: Meta) {
        self.dict.set(key, value);
    }

    pub fn remove(&self, key: &TokenId) {
        self.dict.remove::<Meta>(key);
    }
}

pub fn name() -> String {
    get_key("name").unwrap_or_revert()
}

pub fn symbol() -> String {
    get_key("symbol").unwrap_or_revert()
}

pub fn meta() -> Meta {
    get_key("meta").unwrap_or_revert()
}

pub fn total_supply() -> U256 {
    get_key("total_supply").unwrap_or_revert()
}

pub fn update_total_supply(total_supply: U256) {
    set_key("total_supply", total_supply);
}

pub fn is_paused() -> bool {
    get_key("paused").unwrap()
}

pub fn pause() {
    set_key("paused", true);
}

pub fn unpause() {
    set_key("paused", false);
}

pub fn get_nonce() -> u32 {
    get_key("nonce").unwrap_or_default()
}

pub fn set_nonce(nonce: u32) {
    set_key("nonce", nonce);
}

pub fn contract_package_hash() -> ContractPackageHash {
    get_key("contract_package_hash").unwrap_or_revert()
}

pub fn initial_named_keys(
    package_hash: ContractPackageHash,
    token_name: &str,
    token_symbol: &str,
    token_meta: Meta,
    paused: bool,
) -> NamedKeys {
    let mut named_keys = NamedKeys::new();
    named_keys.insert("name".to_string(), storage::new_uref(token_name).into());
    named_keys.insert("symbol".to_string(), storage::new_uref(token_symbol).into());
    named_keys.insert("meta".to_string(), storage::new_uref(token_meta).into());
    named_keys.insert(
        "total_supply".to_string(),
        storage::new_uref(U256::zero()).into(),
    );
    named_keys.insert("paused".to_string(), storage::new_uref(paused).into());
    named_keys.insert(
        "contract_package_hash".to_string(),
        storage::new_uref(package_hash).into(),
    );

    // Add empty dictionaries.
    add_empty_dict(&mut named_keys, BALANCES_DICT);
    add_empty_dict(&mut named_keys, TOKEN_OWNERS_DICT);
    add_empty_dict(&mut named_keys, TOKEN_ISSUERS_DICT);
    add_empty_dict(&mut named_keys, METADATA_DICT);
    add_empty_dict(&mut named_keys, OWNED_TOKENS_BY_INDEX_DICT);
    add_empty_dict(&mut named_keys, OWNED_INDEXES_BY_TOKEN_DICT);

    named_keys
}

fn add_empty_dict(named_keys: &mut NamedKeys, name: &str) {
    let dict = new_dictionary(name).unwrap_or_revert();
    runtime::remove_key(name);
    named_keys.insert(name.to_string(), dict.into());
}

fn key_to_str(key: &Key) -> String {
    match key {
        Key::Account(account) => account.to_string(),
        Key::Hash(package) => hex::encode(package),
        _ => runtime::revert(ApiError::UnexpectedKeyVariant),
    }
}

pub fn key_and_value_to_str<T: CLTyped + ToBytes>(key: &Key, value: &T) -> String {
    let mut bytes_a = key.to_bytes().unwrap_or_revert();
    let mut bytes_b = value.to_bytes().unwrap_or_revert();

    bytes_a.append(&mut bytes_b);

    let bytes = runtime::blake2b(bytes_a);
    hex::encode(bytes)
}

pub fn emit(cep47_event: &CEP47Event) {
    let mut events = Vec::new();
    let package = contract_package_hash();
    match cep47_event {
        CEP47Event::MetadataUpdate { token_id } => {
            let mut event = BTreeMap::new();
            event.insert("contract_package_hash", package.to_string());
            event.insert("event_type", cep47_event.type_name());
            event.insert("token_id", token_id.to_string());
            events.push(event);
        }
        CEP47Event::Transfer {
            sender,
            recipient,
            token_ids,
        } => {
            for token_id in token_ids {
                let mut event = BTreeMap::new();
                event.insert("contract_package_hash", package.to_string());
                event.insert("event_type", cep47_event.type_name());
                event.insert("sender", sender.to_string());
                event.insert("recipient", recipient.to_string());
                event.insert("token_id", token_id.to_string());
                events.push(event);
            }
        }
        CEP47Event::Mint {
            recipient,
            token_ids,
        } => {
            for token_id in token_ids {
                let mut event = BTreeMap::new();
                event.insert("contract_package_hash", package.to_string());
                event.insert("event_type", cep47_event.type_name());
                event.insert("recipient", recipient.to_string());
                event.insert("token_id", token_id.to_string());
                events.push(event);
            }
        }
        CEP47Event::Burn { owner, token_ids } => {
            for token_id in token_ids {
                let mut event = BTreeMap::new();
                event.insert("contract_package_hash", package.to_string());
                event.insert("event_type", cep47_event.type_name());
                event.insert("owner", owner.to_string());
                event.insert("token_id", token_id.to_string());
                events.push(event);
            }
        }
    };
    for event in events {
        let _: URef = storage::new_uref(event);
    }
}

fn get_key<T: FromBytes + CLTyped>(name: &str) -> Option<T> {
    match runtime::get_key(name) {
        None => None,
        Some(value) => {
            let key = value.try_into().unwrap_or_revert();
            let value = storage::read(key).unwrap_or_revert().unwrap_or_revert();
            Some(value)
        }
    }
}

fn set_key<T: ToBytes + CLTyped>(name: &str, value: T) {
    match runtime::get_key(name) {
        Some(key) => {
            let key_ref = key.try_into().unwrap_or_revert();
            storage::write(key_ref, value);
        }
        None => {
            let key = storage::new_uref(value).into();
            runtime::put_key(name, key);
        }
    }
}
