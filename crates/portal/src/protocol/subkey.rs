use nostr::{Keys, PublicKey};
use nostr::{
    prelude::*,
    secp256k1::{Scalar, Secp256k1},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ops::Deref;

use crate::protocol::model::{Nonce, Timestamp};

use super::model::auth::SubkeyProof;

/// A subkey with its associated metadata
#[derive(Debug, Clone)]
pub struct Subkey {
    key: Keys,
    metadata: SubkeyMetadata,
    master_public: PublicKey,
}

impl Deref for Subkey {
    type Target = Keys;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl Subkey {
    pub fn new(key: Keys, metadata: SubkeyMetadata, master_public: PublicKey) -> Self {
        Self {
            key,
            metadata,
            master_public,
        }
    }

    pub fn metadata(&self) -> &SubkeyMetadata {
        &self.metadata
    }

    pub fn split(self) -> (Keys, SubkeyProof) {
        (
            self.key,
            SubkeyProof {
                main_key: self.master_public.into(),
                metadata: self.metadata,
            },
        )
    }
}

/// Trait for managing private subkeys
pub trait PrivateSubkeyManager {
    /// Creates a new subkey with the given metadata
    fn create_subkey(&self, metadata: &SubkeyMetadata) -> Result<Subkey, SubkeyError>;
}

/// Trait for verifying public subkeys
pub trait PublicSubkeyVerifier {
    /// Verifies that a public key is a valid subkey of the main key
    fn verify_subkey(
        &self,
        subkey: &PublicKey,
        metadata: &SubkeyMetadata,
    ) -> Result<(), SubkeyError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct SubkeyMetadata {
    pub name: String,
    pub nonce: Nonce,
    pub valid_from: Timestamp,
    pub expires_at: Timestamp,
    pub permissions: Vec<SubkeyPermission>,
    pub version: u8,
}

impl SubkeyMetadata {
    pub fn get_tweak(&self) -> Result<Scalar, SubkeyError> {
        // Serialize metadata to bytes
        let metadata_bytes = serde_json::to_vec(self)?;

        // Compute the tweaking factor by hashing the metadata
        let mut hasher = Sha256::new();
        hasher.update(&metadata_bytes);
        let hash: [u8; 32] = hasher
            .finalize()
            .try_into()
            .map_err(|_| SubkeyError::InvalidMetadata)?;
        let tweak = Scalar::from_be_bytes(hash).map_err(|_| SubkeyError::InvalidMetadata)?;
        Ok(tweak)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
pub enum SubkeyPermission {
    Auth,
    Payment,
}

#[derive(Debug, thiserror::Error)]
pub enum SubkeyError {
    #[error("Invalid metadata")]
    InvalidMetadata,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Secp256k1 error: {0}")]
    Secp256k1(#[from] nostr::secp256k1::Error),

    #[error("Key error: {0}")]
    Key(#[from] nostr::key::Error),
}

impl PrivateSubkeyManager for Keys {
    fn create_subkey(&self, metadata: &SubkeyMetadata) -> Result<Subkey, SubkeyError> {
        let secp = Secp256k1::new();
        let tweak = metadata.get_tweak()?;

        // Apply the tweak to the private key
        let mut secret_key =
            nostr::secp256k1::SecretKey::from_slice(&self.secret_key().secret_bytes())?;
        if secret_key.public_key(&secp).x_only_public_key().1 == nostr::secp256k1::Parity::Odd {
            secret_key = secret_key.negate();
        }

        let tweaked_key = secret_key.add_tweak(&tweak)?;
        let key_pair = Keys::new(nostr::SecretKey::from(tweaked_key));

        let master_public = self.public_key();

        Ok(Subkey::new(key_pair, metadata.clone(), master_public))
    }
}

impl PublicSubkeyVerifier for Keys {
    fn verify_subkey(
        &self,
        subkey: &PublicKey,
        metadata: &SubkeyMetadata,
    ) -> Result<(), SubkeyError> {
        self.public_key().verify_subkey(subkey, metadata)
    }
}

impl PublicSubkeyVerifier for PublicKey {
    fn verify_subkey(
        &self,
        subkey: &PublicKey,
        metadata: &SubkeyMetadata,
    ) -> Result<(), SubkeyError> {
        let tweak = metadata.get_tweak()?;

        // Add the tweak * G to the public key
        let secp = Secp256k1::new();
        let (tweaked, _) = self.xonly()?.add_tweak(&secp, &tweak)?;

        // Check if the resulting public key matches
        if tweaked == subkey.xonly()? {
            Ok(())
        } else {
            Err(SubkeyError::InvalidMetadata)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::Nonce;

    fn create_test_metadata(
        name: &str,
        valid_from: u64,
        expires_at: u64,
        permissions: Vec<SubkeyPermission>,
    ) -> SubkeyMetadata {
        SubkeyMetadata {
            name: name.to_string(),
            nonce: Nonce::new([0u8; 32]),
            valid_from: Timestamp::new(valid_from),
            expires_at: Timestamp::new(expires_at),
            permissions,
            version: 1,
        }
    }

    #[test]
    fn test_basic_subkey_creation_and_verification() {
        // Create a main key pair
        let main_key = Keys::generate();
        let metadata = create_test_metadata("test_subkey", 0, u64::MAX, vec![]);

        // Create a subkey using the private trait
        let subkey = main_key.create_subkey(&metadata).unwrap();

        // Test that we can access both the metadata and the key methods
        assert_eq!(subkey.metadata().name, "test_subkey");
        assert_eq!(subkey.metadata().version, 1);

        // Test that Deref works and we can use Keys methods directly
        let pubkey = subkey.public_key();

        // Verify using the main private key
        main_key.verify_subkey(&pubkey, &metadata).unwrap();

        // Verify using just the main public key
        main_key
            .public_key()
            .verify_subkey(&pubkey, &metadata)
            .unwrap();
    }

    #[test]
    fn test_subkey_with_different_permissions() {
        let main_key = Keys::generate();

        // Test different permission combinations
        let permission_sets = vec![
            vec![SubkeyPermission::Auth],
            vec![SubkeyPermission::Payment],
            vec![SubkeyPermission::Auth, SubkeyPermission::Payment],
        ];

        for permissions in permission_sets {
            let metadata = create_test_metadata(
                &format!("subkey_with_{:?}", permissions),
                0,
                u64::MAX,
                permissions.clone(),
            );

            let subkey = main_key.create_subkey(&metadata).unwrap();
            assert_eq!(subkey.metadata().permissions, permissions);

            // Verify with both private and public key
            main_key
                .verify_subkey(&subkey.public_key(), &metadata)
                .unwrap();
            main_key
                .public_key()
                .verify_subkey(&subkey.public_key(), &metadata)
                .unwrap();
        }
    }

    #[test]
    fn test_subkey_deterministic_derivation() {
        let main_key = Keys::generate();
        let metadata = create_test_metadata(
            "deterministic_test",
            0,
            u64::MAX,
            vec![SubkeyPermission::Auth],
        );

        // Create two subkeys with the same metadata
        let subkey1 = main_key.create_subkey(&metadata).unwrap();
        let subkey2 = main_key.create_subkey(&metadata).unwrap();

        // They should be identical
        assert_eq!(subkey1.public_key(), subkey2.public_key());
        assert_eq!(
            subkey1.secret_key().secret_bytes(),
            subkey2.secret_key().secret_bytes()
        );
    }

    #[test]
    fn test_subkey_verification_failures() {
        let main_key = Keys::generate();
        let wrong_key = Keys::generate();
        let metadata =
            create_test_metadata("test_failures", 0, u64::MAX, vec![SubkeyPermission::Auth]);

        // Create a valid subkey
        let subkey = main_key.create_subkey(&metadata).unwrap();

        // Test with wrong metadata
        let wrong_metadata =
            create_test_metadata("wrong_name", 0, u64::MAX, vec![SubkeyPermission::Auth]);
        assert!(matches!(
            main_key.verify_subkey(&subkey.public_key(), &wrong_metadata),
            Err(SubkeyError::InvalidMetadata)
        ));

        // Test with wrong main key
        assert!(matches!(
            wrong_key.verify_subkey(&subkey.public_key(), &metadata),
            Err(SubkeyError::InvalidMetadata)
        ));

        // Test with wrong subkey
        let wrong_subkey = wrong_key.create_subkey(&metadata).unwrap();
        assert!(matches!(
            main_key.verify_subkey(&wrong_subkey.public_key(), &metadata),
            Err(SubkeyError::InvalidMetadata)
        ));

        // Test verification with wrong public key
        assert!(matches!(
            wrong_key
                .public_key()
                .verify_subkey(&subkey.public_key(), &metadata),
            Err(SubkeyError::InvalidMetadata)
        ));
    }

    #[test]
    fn test_subkey_with_different_nonces() {
        let main_key = Keys::generate();
        let mut metadata1 =
            create_test_metadata("nonce_test", 0, u64::MAX, vec![SubkeyPermission::Auth]);
        let mut metadata2 = metadata1.clone();

        // Set different nonces
        metadata1.nonce = Nonce::new([1u8; 32]);
        metadata2.nonce = Nonce::new([2u8; 32]);

        // Create subkeys with different nonces
        let subkey1 = main_key.create_subkey(&metadata1).unwrap();
        let subkey2 = main_key.create_subkey(&metadata2).unwrap();

        // They should be different
        assert_ne!(subkey1.public_key(), subkey2.public_key());
        assert_ne!(
            subkey1.secret_key().secret_bytes(),
            subkey2.secret_key().secret_bytes()
        );

        // But both should verify correctly
        main_key
            .verify_subkey(&subkey1.public_key(), &metadata1)
            .unwrap();
        main_key
            .verify_subkey(&subkey2.public_key(), &metadata2)
            .unwrap();
    }

    #[test]
    fn test_multiple_subkeys_from_same_parent() {
        let main_key = Keys::generate();
        let mut subkeys = Vec::new();

        // Create multiple subkeys
        for i in 0..5 {
            let metadata = create_test_metadata(
                &format!("subkey_{}", i),
                0,
                u64::MAX,
                vec![SubkeyPermission::Auth],
            );
            let subkey = main_key.create_subkey(&metadata).unwrap();
            subkeys.push((subkey, metadata));
        }

        // Verify all subkeys are different
        for i in 0..subkeys.len() {
            for j in i + 1..subkeys.len() {
                assert_ne!(subkeys[i].0.public_key(), subkeys[j].0.public_key());
            }
        }

        // Verify all subkeys with both private and public key
        for (subkey, metadata) in subkeys {
            main_key
                .verify_subkey(&subkey.public_key(), &metadata)
                .unwrap();
            main_key
                .public_key()
                .verify_subkey(&subkey.public_key(), &metadata)
                .unwrap();
        }
    }
}
