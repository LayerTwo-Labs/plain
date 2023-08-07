use crate::consensus::drivechain::Drivechain;
use crate::consensus::types::*;
use bitcoin::hashes::Hash as _;
use std::net::SocketAddr;
use std::str::FromStr as _;

pub use crate::consensus::drivechain::MainClient;

#[derive(Clone)]
pub struct Miner {
    pub drivechain: Drivechain,
    block: Option<(Header, Body)>,
    sidechain_number: u8,
}

impl Miner {
    pub fn new(
        sidechain_number: u8,
        main_addr: SocketAddr,
        user: &str,
        password: &str,
    ) -> Result<Self, Error> {
        let drivechain = Drivechain::new(sidechain_number, main_addr, user, password)?;
        Ok(Self {
            drivechain,
            sidechain_number,
            block: None,
        })
    }

    pub async fn generate(&self) -> Result<(), Error> {
        self.drivechain
            .client
            .generate(1)
            .await
            .map_err(crate::consensus::drivechain::Error::from)?;
        Ok(())
    }

    pub async fn attempt_bmm(
        &mut self,
        amount: u64,
        height: u32,
        header: Header,
        body: Body,
    ) -> Result<(), Error> {
        let str_hash_prev = header.prev_main_hash.to_string();
        let critical_hash: [u8; 32] = header.hash().into();
        let critical_hash = bitcoin::BlockHash::from_byte_array(critical_hash);
        let value = self
            .drivechain
            .client
            .createbmmcriticaldatatx(
                bitcoin::Amount::from_sat(amount).into(),
                height,
                &critical_hash,
                self.sidechain_number,
                &str_hash_prev[str_hash_prev.len() - 8..],
            )
            .await
            .map_err(crate::consensus::drivechain::Error::from)?;
        bitcoin::Txid::from_str(value["txid"]["txid"].as_str().ok_or(Error::InvalidJson)?)
            .map_err(crate::consensus::drivechain::Error::from)?;
        assert_eq!(header.merkle_root, body.compute_merkle_root());
        self.block = Some((header, body));
        Ok(())
    }

    pub async fn confirm_bmm(&mut self) -> Result<Option<(Header, Body)>, Error> {
        if let Some((header, body)) = self.block.clone() {
            self.drivechain.verify_bmm(&header).await?;
            self.block = None;
            return Ok(Some((header, body)));
        }
        Ok(None)
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("drivechain error")]
    Drivechain(#[from] crate::consensus::drivechain::Error),
    #[error("invalid jaon")]
    InvalidJson,
}
