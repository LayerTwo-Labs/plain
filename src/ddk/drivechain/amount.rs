use bitcoin::util::amount::serde::SerdeAmount;
use bitnames_types::bitcoin;
use std::ops::{Deref, DerefMut};

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
