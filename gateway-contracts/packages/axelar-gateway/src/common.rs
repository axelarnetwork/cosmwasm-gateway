use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Order, LogAttribute};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Asc,
    Desc,
}

impl Into<Order> for OrderBy {
    fn into(self) -> Order {
        if self == OrderBy::Asc {
            Order::Ascending
        } else {
            Order::Descending
        }
    }
}

pub fn log_attribute<K: ToString, A: ToString, V: ToString>(key: K, attribute: A, value: V) -> LogAttribute {
    LogAttribute {
        key: format!("{}:{}", key.to_string(), attribute.to_string()),
        value: value.to_string(),
    }
}
