pub use crate::types::address::*;
pub use crate::types::hashes::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutPoint {
    // Created by transactions.
    Regular { txid: Txid, vout: u32 },
    // Created by block bodies.
    Coinbase { merkle_root: MerkleRoot, vout: u32 },
    // Created by mainchain deposits.
    Deposit(bitcoin::OutPoint),
}

impl std::fmt::Display for OutPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regular { txid, vout } => write!(f, "regular {txid} {vout}"),
            Self::Coinbase { merkle_root, vout } => write!(f, "coinbase {merkle_root} {vout}"),
            Self::Deposit(bitcoin::OutPoint { txid, vout }) => write!(f, "deposit {txid} {vout}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output<C> {
    pub address: Address,
    pub content: Content<C>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content<C> {
    Custom(C),
    Value(u64),
    Withdrawal {
        value: u64,
        main_fee: u64,
        main_address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    },
}

impl<C> Content<C> {
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }
    pub fn is_withdrawal(&self) -> bool {
        matches!(self, Self::Withdrawal { .. })
    }
}

impl<C> GetAddress for Output<C> {
    #[inline(always)]
    fn get_address(&self) -> Address {
        self.address
    }
}

impl<C: GetValue> GetValue for Output<C> {
    #[inline(always)]
    fn get_value(&self) -> u64 {
        self.content.get_value()
    }
}

impl<C: GetValue> GetValue for Content<C> {
    #[inline(always)]
    fn get_value(&self) -> u64 {
        match self {
            Self::Custom(custom) => custom.get_value(),
            Self::Value(value) => *value,
            Self::Withdrawal { value, .. } => *value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction<C> {
    pub inputs: Vec<OutPoint>,
    pub outputs: Vec<Output<C>>,
}

impl<C: Serialize> Transaction<C> {
    pub fn txid(&self) -> Txid {
        hash(self).into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilledTransaction<C> {
    pub transaction: Transaction<C>,
    pub spent_utxos: Vec<Output<C>>,
}

impl<C: GetValue> FilledTransaction<C> {
    pub fn get_value_in(&self) -> u64 {
        self.spent_utxos.iter().map(GetValue::get_value).sum()
    }

    pub fn get_value_out(&self) -> u64 {
        self.transaction
            .outputs
            .iter()
            .map(GetValue::get_value)
            .sum()
    }

    pub fn get_fee(&self) -> Option<u64> {
        let value_in = self.get_value_in();
        let value_out = self.get_value_out();
        if value_in < value_out {
            None
        } else {
            Some(value_in - value_out)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizedTransaction<A, C> {
    pub transaction: Transaction<C>,
    /// Authorization is called witness in Bitcoin.
    pub authorizations: Vec<A>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<A, C> {
    pub coinbase: Vec<Output<C>>,
    pub transactions: Vec<Transaction<C>>,
    pub authorizations: Vec<A>,
}

impl<A, C: Clone + GetValue + Serialize> Body<A, C> {
    pub fn new(
        authorized_transactions: Vec<AuthorizedTransaction<A, C>>,
        coinbase: Vec<Output<C>>,
    ) -> Self {
        let mut authorizations = Vec::with_capacity(
            authorized_transactions
                .iter()
                .map(|t| t.transaction.inputs.len())
                .sum(),
        );
        let mut transactions = Vec::with_capacity(authorized_transactions.len());
        for at in authorized_transactions.into_iter() {
            authorizations.extend(at.authorizations);
            transactions.push(at.transaction);
        }
        Self {
            coinbase,
            transactions,
            authorizations,
        }
    }

    pub fn compute_merkle_root(&self) -> MerkleRoot {
        // FIXME: Compute actual merkle root instead of just a hash.
        hash(&(&self.coinbase, &self.transactions)).into()
    }

    pub fn get_inputs(&self) -> Vec<OutPoint> {
        self.transactions
            .iter()
            .flat_map(|tx| tx.inputs.iter())
            .copied()
            .collect()
    }

    pub fn get_outputs(&self) -> HashMap<OutPoint, Output<C>> {
        let mut outputs = HashMap::new();
        let merkle_root = self.compute_merkle_root();
        for (vout, output) in self.coinbase.iter().enumerate() {
            let vout = vout as u32;
            let outpoint = OutPoint::Coinbase { merkle_root, vout };
            outputs.insert(outpoint, output.clone());
        }
        for transaction in &self.transactions {
            let txid = transaction.txid();
            for (vout, output) in transaction.outputs.iter().enumerate() {
                let vout = vout as u32;
                let outpoint = OutPoint::Regular { txid, vout };
                outputs.insert(outpoint, output.clone());
            }
        }
        outputs
    }

    pub fn get_coinbase_value(&self) -> u64 {
        self.coinbase.iter().map(|output| output.get_value()).sum()
    }
}

pub trait GetAddress {
    fn get_address(&self) -> Address;
}

pub trait GetValue {
    fn get_value(&self) -> u64;
}

impl GetValue for () {
    fn get_value(&self) -> u64 {
        0
    }
}

pub trait Verify<C> {
    type Error;
    fn verify_transaction(transaction: &AuthorizedTransaction<Self, C>) -> Result<(), Self::Error>
    where
        Self: Sized;
    fn verify_body(body: &Body<Self, C>) -> Result<(), Self::Error>
    where
        Self: Sized;
}
