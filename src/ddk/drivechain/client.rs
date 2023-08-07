use bitcoin::amount::serde::SerdeAmount;
use jsonrpsee::proc_macros::rpc;
use std::ops::{Deref, DerefMut};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WithdrawalStatus {
    hash: bitcoin::Txid,
    nblocksleft: usize,
    nworkscore: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SpentWithdrawal {
    pub nsidechain: u8,
    pub hash: bitcoin::Txid,
    pub hashblock: bitcoin::BlockHash,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FailedWithdrawal {
    pub nsidechain: u8,
    pub hash: bitcoin::Txid,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    Upvote,
    Abstain,
    Downvote,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub hash: bitcoin::BlockHash,
    pub confirmations: usize,
    pub strippedsize: usize,
    pub size: usize,
    pub weight: usize,
    pub height: usize,
    pub version: i32,
    pub version_hex: String,
    pub merkleroot: bitcoin::hash_types::TxMerkleNode,
    pub tx: Vec<bitcoin::Txid>,
    pub time: u32,
    pub mediantime: u32,
    pub nonce: u32,
    pub bits: String,
    pub difficulty: f64,
    pub chainwork: String,
    pub previousblockhash: Option<bitcoin::BlockHash>,
    pub nextblockhash: Option<bitcoin::BlockHash>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deposit {
    pub hashblock: bitcoin::BlockHash,
    pub nburnindex: usize,
    pub ntx: usize,
    pub strdest: String,
    pub txhex: String,
}

#[rpc(client)]
pub trait Main {
    #[method(name = "stop")]
    async fn stop(&self) -> Result<String, jsonrpsee::core::Error>;
    // FIXME: Define a "Deposit Address" type.
    #[method(name = "listwithdrawalstatus")]
    async fn listwithdrawalstatus(
        &self,
        nsidechain: u8,
    ) -> Result<Vec<WithdrawalStatus>, jsonrpsee::core::Error>;
    #[method(name = "listspentwithdrawals")]
    async fn listspentwithdrawals(&self) -> Result<Vec<SpentWithdrawal>, jsonrpsee::core::Error>;
    #[method(name = "listfailedwithdrawals")]
    async fn listfailedwithdrawals(&self) -> Result<Vec<FailedWithdrawal>, jsonrpsee::core::Error>;
    #[method(name = "getblockcount")]
    async fn getblockcount(&self) -> Result<usize, jsonrpsee::core::Error>;
    #[method(name = "getbestblockhash")]
    async fn getbestblockhash(&self) -> Result<bitcoin::BlockHash, jsonrpsee::core::Error>;
    #[method(name = "getblock")]
    async fn getblock(
        &self,
        blockhash: &bitcoin::BlockHash,
        verbosity: Option<usize>,
    ) -> Result<Block, jsonrpsee::core::Error>;
    #[method(name = "createbmmcriticaldatatx")]
    async fn createbmmcriticaldatatx(
        &self,
        amount: AmountBtc,
        height: u32,
        criticalhash: &bitcoin::BlockHash,
        nsidechain: u8,
        prevbytes: &str,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error>;
    #[method(name = "verifybmm")]
    async fn verifybmm(
        &self,
        blockhash: &bitcoin::BlockHash,
        criticalhash: &bitcoin::BlockHash,
        nsidechain: u8,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error>;

    #[method(name = "listsidechaindepositsbyblock")]
    async fn listsidechaindepositsbyblock(
        &self,
        nsidechain: u8,
        end_blockhash: Option<bitcoin::BlockHash>,
        start_blockhash: Option<bitcoin::BlockHash>,
    ) -> Result<Vec<Deposit>, jsonrpsee::core::Error>;

    #[method(name = "receivewithdrawalbundle")]
    async fn receivewithdrawalbundle(
        &self,
        nsidechain: u8,
        // Raw transaction hex.
        rawtx: &str,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error>;

    #[method(name = "generate")]
    async fn generate(&self, num: u32) -> Result<serde_json::Value, jsonrpsee::core::Error>;

    #[method(name = "getnewaddress")]
    async fn getnewaddress(
        &self,
        account: &str,
        address_type: &str,
    ) -> Result<bitcoin::Address<bitcoin::address::NetworkUnchecked>, jsonrpsee::core::Error>;

    #[method(name = "createsidechaindeposit")]
    async fn createsidechaindeposit(
        &self,
        nsidechain: u8,
        depositaddress: &str,
        amount: AmountBtc,
        fee: AmountBtc,
    ) -> Result<serde_json::Value, jsonrpsee::core::Error>;
}

// Arguments:
// 1. "amount"         (numeric or string, required) The amount in BTC to be spent.
// 2. "height"         (numeric, required) The block height this transaction must be included in.
// Note: If 0 is passed in for height, current block height will be used
// 3. "criticalhash"   (string, required) h* you want added to a coinbase
// 4. "nsidechain"     (numeric, required) Sidechain requesting BMM
// 5. "prevbytes"      (string, required) a portion of the previous block hash

// FIXME: Make mainchain API machine friendly. Parsing human readable amounts
// here is stupid -- just take and return values in satoshi.
#[derive(Clone, Copy)]
pub struct AmountBtc(pub bitcoin::Amount);

impl From<bitcoin::Amount> for AmountBtc {
    fn from(other: bitcoin::Amount) -> AmountBtc {
        AmountBtc(other)
    }
}

impl From<AmountBtc> for bitcoin::Amount {
    fn from(other: AmountBtc) -> bitcoin::Amount {
        other.0
    }
}

impl Deref for AmountBtc {
    type Target = bitcoin::Amount;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AmountBtc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de> serde::Deserialize<'de> for AmountBtc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(AmountBtc(bitcoin::Amount::des_btc(deserializer)?))
    }
}

impl serde::Serialize for AmountBtc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.ser_btc(serializer)
    }
}
