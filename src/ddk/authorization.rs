use crate::types::blake3;
use crate::types::{Address, AuthorizedTransaction, Body, GetAddress, Transaction, Verify};
pub use ed25519_dalek::{Keypair, PublicKey, Signature, SignatureError, Signer, Verifier};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authorization {
    pub public_key: PublicKey,
    pub signature: Signature,
}

impl GetAddress for Authorization {
    fn get_address(&self) -> Address {
        get_address(&self.public_key)
    }
}

impl<C: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync> Verify<C> for Authorization {
    type Error = Error;
    fn verify_transaction(transaction: &AuthorizedTransaction<Self, C>) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        verify_authorized_transaction(transaction)?;
        Ok(())
    }

    fn verify_body(body: &Body<Self, C>) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        verify_authorizations(body)?;
        Ok(())
    }
}

pub fn get_address(public_key: &PublicKey) -> Address {
    let mut hasher = blake3::Hasher::new();
    let mut reader = hasher.update(&public_key.to_bytes()).finalize_xof();
    let mut output: [u8; 20] = [0; 20];
    reader.fill(&mut output);
    Address(output)
}

struct Package<'a> {
    messages: Vec<&'a [u8]>,
    signatures: Vec<Signature>,
    public_keys: Vec<PublicKey>,
}

pub fn verify_authorized_transaction<C: Clone + Serialize + Sync>(
    transaction: &AuthorizedTransaction<Authorization, C>,
) -> Result<(), Error> {
    let serialized_transaction = bincode::serialize(&transaction.transaction)?;
    let messages: Vec<_> = std::iter::repeat(serialized_transaction.as_slice())
        .take(transaction.authorizations.len())
        .collect();
    let (public_keys, signatures): (Vec<PublicKey>, Vec<Signature>) = transaction
        .authorizations
        .iter()
        .map(
            |Authorization {
                 public_key,
                 signature,
             }| (public_key, signature),
        )
        .unzip();
    ed25519_dalek::verify_batch(&messages, &signatures, &public_keys)?;
    Ok(())
}

pub fn verify_authorizations<C: Clone + Serialize + Sync>(
    body: &Body<Authorization, C>,
) -> Result<(), Error> {
    let input_numbers = body
        .transactions
        .iter()
        .map(|transaction| transaction.inputs.len());
    let serialized_transactions: Vec<Vec<u8>> = body
        .transactions
        .par_iter()
        .map(bincode::serialize)
        .collect::<Result<_, _>>()?;
    let serialized_transactions = serialized_transactions.iter().map(Vec::as_slice);
    let messages = input_numbers.zip(serialized_transactions).flat_map(
        |(input_number, serialized_transaction)| {
            std::iter::repeat(serialized_transaction).take(input_number)
        },
    );

    let pairs = body.authorizations.iter().zip(messages).collect::<Vec<_>>();

    let num_threads = rayon::current_num_threads();
    let num_authorizations = body.authorizations.len();
    let package_size = num_authorizations / num_threads;
    let mut packages: Vec<Package> = Vec::with_capacity(num_threads);
    for i in 0..num_threads {
        let mut package = Package {
            messages: Vec::with_capacity(package_size),
            signatures: Vec::with_capacity(package_size),
            public_keys: Vec::with_capacity(package_size),
        };
        for (authorization, message) in &pairs[i * package_size..(i + 1) * package_size] {
            package.messages.push(*message);
            package.signatures.push(authorization.signature);
            package.public_keys.push(authorization.public_key);
        }
        packages.push(package);
    }
    for (authorization, message) in &pairs[num_threads * package_size..] {
        packages[num_threads - 1].messages.push(*message);
        packages[num_threads - 1]
            .signatures
            .push(authorization.signature);
        packages[num_threads - 1]
            .public_keys
            .push(authorization.public_key);
    }
    assert_eq!(
        packages.iter().map(|p| p.signatures.len()).sum::<usize>(),
        body.authorizations.len()
    );
    packages
        .par_iter()
        .map(
            |Package {
                 messages,
                 signatures,
                 public_keys,
             }| ed25519_dalek::verify_batch(messages, signatures, public_keys),
        )
        .collect::<Result<(), SignatureError>>()?;
    Ok(())
}

pub fn sign<C: Clone + Serialize>(
    keypair: &Keypair,
    transaction: &Transaction<C>,
) -> Result<Signature, Error> {
    let message = bincode::serialize(&transaction)?;
    Ok(keypair.sign(&message))
}

pub fn authorize<C: Clone + Serialize>(
    addresses_keypairs: &[(Address, &Keypair)],
    transaction: Transaction<C>,
) -> Result<AuthorizedTransaction<Authorization, C>, Error> {
    let mut authorizations: Vec<Authorization> = Vec::with_capacity(addresses_keypairs.len());
    let message = bincode::serialize(&transaction)?;
    for (address, keypair) in addresses_keypairs {
        let hash_public_key = get_address(&keypair.public);
        if *address != hash_public_key {
            return Err(Error::WrongKeypairForAddress {
                address: *address,
                hash_public_key,
            });
        }
        let authorization = Authorization {
            public_key: keypair.public,
            signature: keypair.sign(&message),
        };
        authorizations.push(authorization);
    }
    Ok(AuthorizedTransaction {
        authorizations,
        transaction,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "wrong keypair for address: address = {address},  hash(public_key) = {hash_public_key}"
    )]
    WrongKeypairForAddress {
        address: Address,
        hash_public_key: Address,
    },
    #[error("ed25519_dalek error")]
    DalekError(#[from] SignatureError),
    #[error("bincode error")]
    BincodeError(#[from] bincode::Error),
}
