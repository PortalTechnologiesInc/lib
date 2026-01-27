use crate::protocol::model::Timestamp;
use hex;
use nostr;
use nostr::secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Invalid merkle root")]
    InvalidMerkleRoot,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Secp256k1 error: {0}")]
    Secp256k1(#[from] nostr::secp256k1::Error),

    #[error("Key error: {0}")]
    KeyError(#[from] nostr::key::Error),

    #[error("Data deserialization error: {0}")]
    DataDeserialization(#[from] RevealError),

    #[error("Certificate has expired")]
    Expired,

    #[error("Certificate is not yet valid")]
    NotYetValid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    InPerson,
    VideoCall,
    DocumentUpload,
    RegistryCheck,
    ThirdPartyVerification,
    #[serde(rename = "custom")]
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub version: u32,
    pub subject: nostr::PublicKey,
    pub data: CertificateData,
    pub metadata: CertificateMetadata,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCertificateData {
    pub version: u32,
    pub subject: nostr::PublicKey,
    pub metadata: CertificateMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialCertificate {
    pub version: u32,
    pub subject: nostr::PublicKey,
    pub metadata: CertificateMetadata,
    pub signature: String,
    pub merkle_proof: MerkleProofNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CertificateData {
    #[serde(rename = "personal")]
    Person(PersonData),
    #[serde(rename = "business")]
    Business(BusinessData),
    #[serde(rename = "custom")]
    Custom {
        #[serde(flatten)]
        data: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaltSequence {
    pub salt_size: usize,
    #[serde(with = "base64_serde")]
    pub value: Vec<u8>,
}

impl SaltSequence {
    pub fn new(salt_size: usize, value: Vec<u8>) -> Self {
        Self { salt_size, value }
    }

    pub fn get_salt(&self, n: usize) -> Option<&[u8]> {
        self.value.chunks_exact(self.salt_size).nth(n)
    }
}

// Helper module for base64 serialization
mod base64_serde {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        s.serialize_str(&base64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(base64)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone)]
pub struct MerkleRoot([u8; 32]);

impl MerkleRoot {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Serialize for MerkleRoot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = hex::encode(self.0);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for MerkleRoot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let hex_str = String::deserialize(deserializer)?;

        let bytes = hex::decode(&hex_str)
            .map_err(|e| Error::custom(format!("Invalid hex string: {}", e)))?;

        if bytes.len() != 32 {
            return Err(Error::custom(format!(
                "Invalid merkle root length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(MerkleRoot(arr))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "leaf_type")]
pub enum MerkleProofLeaf {
    #[serde(rename = "cleartext")]
    Cleartext {
        name: String,
        field: RevealableField,
        #[serde(with = "base64_serde")]
        salt: Vec<u8>,
    },
    #[serde(rename = "hidden")]
    Hidden {
        #[serde(with = "hex_serde")]
        hash: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MerkleProofNode {
    #[serde(rename = "leaf")]
    Leaf(MerkleProofLeaf),
    #[serde(rename = "blinded")]
    Blinded {
        #[serde(with = "hex_serde")]
        hash: Vec<u8>,
    },
    #[serde(rename = "parent")]
    Parent {
        left: Box<MerkleProofNode>,
        right: Box<MerkleProofNode>,
    },
}

// Helper module for hex serialization
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let hex = hex::encode(v);
        s.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let hex_str = String::deserialize(d)?;
        hex::decode(hex_str).map_err(serde::de::Error::custom)
    }
}

impl MerkleProofNode {
    pub fn compute_hash(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        match self {
            MerkleProofNode::Leaf(leaf) => match leaf {
                MerkleProofLeaf::Cleartext { name, field, salt } => field.to_hash(name, salt),
                MerkleProofLeaf::Hidden { hash } => hash.clone(),
            },
            MerkleProofNode::Blinded { hash } => hash.clone(),
            MerkleProofNode::Parent { left, right } => {
                let mut hasher = Sha256::new();
                hasher.update(&left.compute_hash());
                hasher.update(&right.compute_hash());
                hasher.finalize().to_vec()
            }
        }
    }

    /// Collects all revealed fields from the proof and constructs a PreparedCertificate.
    /// This is useful for working with partial certificates where only some fields are revealed.
    pub fn to_prepared_certificate(&self) -> PreparedCertificate {
        let mut fields = BTreeMap::new();
        self.collect_revealed_fields(&mut fields);
        PreparedCertificate {
            version: 1, // Default version, as we can't know the original version from just the proof
            fields,
        }
    }

    /// Helper method to recursively collect revealed fields from the proof
    fn collect_revealed_fields(&self, fields: &mut BTreeMap<String, RevealableField>) {
        match self {
            MerkleProofNode::Leaf(MerkleProofLeaf::Cleartext { name, field, .. }) => {
                fields.insert(name.clone(), field.clone());
            }
            MerkleProofNode::Parent { left, right } => {
                left.collect_revealed_fields(fields);
                right.collect_revealed_fields(fields);
            }
            // Hidden and blinded nodes are skipped as they don't contain revealed fields
            _ => {}
        }
    }
}

impl Certificate {
    /// Creates a new certificate with the correct merkle root computed from its fields
    pub fn new(
        version: u32,
        subject: nostr::PublicKey,
        data: CertificateData,
        metadata: CertificateMetadata,
        signature: String,
    ) -> Result<Self, RevealError> {
        // Create a temporary certificate to compute the merkle root
        let temp = Self {
            version,
            subject,
            data,
            metadata,
            signature,
        };

        // Prepare the certificate and compute its merkle root
        let prepared = temp.prepare_for_revealing()?;
        let root_bytes = prepared.compute_merkle_root(&temp.metadata.salt_sequence)?;

        // Create the final certificate with the computed merkle root
        let mut metadata = temp.metadata;
        metadata.merkle_root = MerkleRoot::new(
            root_bytes
                .try_into()
                .map_err(|_| RevealError::InvalidField)?,
        );

        Ok(Self {
            version: temp.version,
            subject: temp.subject,
            data: temp.data,
            metadata,
            signature: temp.signature,
        })
    }

    /// Prepares the certificate for selective revealing by breaking down all fields
    /// into individually revealable components with their own salts and hashes.
    pub fn prepare_for_revealing(&self) -> Result<PreparedCertificate, RevealError> {
        // Only serialize the data part
        let value = serde_json::to_value(&self.data)?;

        let mut fields = BTreeMap::new();
        flatten_json("", &value, &mut fields)?;

        Ok(PreparedCertificate {
            version: self.version,
            fields,
        })
    }

    pub fn get_signed_data(&self) -> SignedCertificateData {
        SignedCertificateData {
            version: self.version,
            subject: self.subject,
            metadata: self.metadata.clone(),
        }
    }

    pub fn sign(&mut self, issuer_key: &nostr::Keys) -> Result<(), SignError> {
        use sha2::{Digest, Sha256};

        // Check the key matches the issuer pubkey
        if self.metadata.issuer_pubkey != issuer_key.public_key() {
            return Err(SignError::InvalidKey);
        }

        // Check that the merkle root matches
        let prepared = self.prepare_for_revealing()?;
        if self.metadata.merkle_root.0.as_slice()
            != &prepared.compute_merkle_root(&self.metadata.salt_sequence)?
        {
            return Err(SignError::InvalidMerkleRoot);
        }

        // Check that the signature is currently empty
        if !self.signature.is_empty() {
            return Err(SignError::AlreadySigned);
        }

        let certificate = serde_json::to_string(&self.get_signed_data())
            .map_err(|e| SignError::Serialization(e))?;

        let mut hasher = Sha256::new();
        hasher.update(certificate.as_bytes());
        let message = nostr::secp256k1::Message::from_digest_slice(&hasher.finalize().to_vec())?;
        let signature = issuer_key.key_pair(&Secp256k1::new()).sign_schnorr(message);

        self.signature = hex::encode(signature.serialize());

        Ok(())
    }
}

impl PartialCertificate {
    pub fn get_signed_data(&self) -> SignedCertificateData {
        SignedCertificateData {
            version: self.version,
            subject: self.subject,
            metadata: self.metadata.clone(),
        }
    }

    pub fn verify(&self) -> Result<serde_json::Value, VerifyError> {
        use sha2::{Digest, Sha256};

        let certificate = serde_json::to_string(&self.get_signed_data())
            .map_err(|e| VerifyError::Serialization(e))?;

        // Check merkle root matches
        if self.metadata.merkle_root.0.as_slice() != &self.merkle_proof.compute_hash() {
            return Err(VerifyError::InvalidMerkleRoot);
        }

        // TODO: check timestamp

        let secp = Secp256k1::new();

        let mut hasher = Sha256::new();
        hasher.update(certificate.as_bytes());
        let message = nostr::secp256k1::Message::from_digest_slice(&hasher.finalize().to_vec())?;
        let signature = nostr::secp256k1::schnorr::Signature::from_slice(
            &hex::decode(&self.signature).map_err(|_| VerifyError::InvalidSignature)?,
        )
        .map_err(|e| VerifyError::Secp256k1(e))?;
        secp.verify_schnorr(&signature, &message, &self.metadata.issuer_pubkey.xonly()?)
            .map_err(|_| VerifyError::InvalidSignature)?;

        let json = self.merkle_proof.to_prepared_certificate().to_json()?;
        Ok(json)
    }
}

impl RevealableField {
    pub fn new<T: Serialize>(value: &T) -> Result<Self, RevealError> {
        let value = serde_json::to_value(value)?;
        Ok(Self { value })
    }

    pub fn to_hash(&self, field_name: &str, salt: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(field_name.as_bytes());
        hasher.update(self.value.to_string().as_bytes());
        hasher.update(salt);

        hasher.finalize().to_vec()
    }
}

fn flatten_json(
    prefix: &str,
    value: &serde_json::Value,
    fields: &mut BTreeMap<String, RevealableField>,
) -> Result<(), RevealError> {
    // TODO: Escape dots in field names to prevent ambiguity with nested field paths
    // For example: a field named "a.b" should be distinguishable from a nested field "a" with subfield "b"
    match value {
        serde_json::Value::Object(map) => {
            // Add the object itself as a field
            if !prefix.is_empty() {
                fields.insert(prefix.to_string(), RevealableField::new(value)?);
            }

            // Then add all its fields
            for (key, val) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_json(&new_prefix, val, fields)?;
            }
        }
        serde_json::Value::Array(arr) => {
            // Add the array itself as a field
            if !prefix.is_empty() {
                fields.insert(prefix.to_string(), RevealableField::new(value)?);
            }

            // Then add all its elements
            for (i, val) in arr.iter().enumerate() {
                let new_prefix = format!("{}.{}", prefix, i);
                flatten_json(&new_prefix, val, fields)?;
            }
        }
        _ => {
            if !prefix.is_empty() {
                fields.insert(prefix.to_string(), RevealableField::new(value)?);
            }
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RevealError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid field value")]
    InvalidField,

    #[error("Invalid salt")]
    InvalidSalt,

    #[error("Insufficient salts in sequence")]
    InsufficientSalts,
}

#[derive(Debug, Clone)]
pub struct PreparedCertificate {
    pub version: u32,
    pub fields: BTreeMap<String, RevealableField>,
}

impl PreparedCertificate {
    /// Computes the Merkle root by hashing all fields in order with their corresponding salts.
    /// The n-th salt from the sequence is applied to the n-th field as sorted by the BTreeMap.
    pub fn compute_merkle_root(
        &self,
        salt_sequence: &SaltSequence,
    ) -> Result<Vec<u8>, RevealError> {
        use sha2::{Digest, Sha256};

        // First compute all the field hashes in order
        let mut field_hashes = Vec::with_capacity(self.fields.len());
        for (i, (field_name, field)) in self.fields.iter().enumerate() {
            let salt = salt_sequence
                .get_salt(i)
                .ok_or(RevealError::InsufficientSalts)?;
            let hash = field.to_hash(field_name, salt);
            field_hashes.push(hash);
        }

        // If there are no fields, return a hash of an empty string
        if field_hashes.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update([]);
            return Ok(hasher.finalize().to_vec());
        }

        // Now build the Merkle tree bottom-up
        while field_hashes.len() > 1 {
            let mut next_level = Vec::with_capacity((field_hashes.len() + 1) / 2);

            // Process pairs of hashes
            for pair in field_hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                if let Some(second) = pair.get(1) {
                    hasher.update(second);
                } else {
                    // If odd number of hashes, duplicate the last one
                    hasher.update(&pair[0]);
                }
                next_level.push(hasher.finalize().to_vec());
            }

            field_hashes = next_level;
        }

        Ok(field_hashes.into_iter().next().unwrap())
    }

    /// Constructs a Merkle proof for the specified fields.
    ///
    /// The proof includes the revealed fields with their salts, and the minimum set of hashes
    /// needed to recompute the Merkle root.
    pub fn create_proof(
        &self,
        salt_sequence: &SaltSequence,
        reveal_fields: &[String],
    ) -> Result<MerkleProofNode, RevealError> {
        use sha2::{Digest, Sha256};

        // First compute all field hashes and create leaf nodes
        let mut leaves: Vec<MerkleProofNode> = Vec::with_capacity(self.fields.len());

        for (i, (field_name, field)) in self.fields.iter().enumerate() {
            let salt = salt_sequence
                .get_salt(i)
                .ok_or(RevealError::InsufficientSalts)?;

            let leaf = if reveal_fields.contains(field_name) {
                MerkleProofNode::Leaf(MerkleProofLeaf::Cleartext {
                    name: field_name.clone(),
                    field: field.clone(),
                    salt: salt.to_vec(),
                })
            } else {
                let hash = field.to_hash(field_name, salt);
                MerkleProofNode::Leaf(MerkleProofLeaf::Hidden { hash })
            };

            leaves.push(leaf);
        }

        // If there are no fields, return a hash of an empty string
        if leaves.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update([]);
            return Ok(MerkleProofNode::Blinded {
                hash: hasher.finalize().to_vec(),
            });
        }

        // Now build the Merkle tree
        while leaves.len() > 1 {
            let mut next_level = Vec::with_capacity((leaves.len() + 1) / 2);

            for pair in leaves.chunks(2) {
                let left = pair[0].clone();
                let right = if let Some(right) = pair.get(1) {
                    right.clone()
                } else {
                    // If odd number of nodes, duplicate the last one
                    left.clone()
                };

                next_level.push(MerkleProofNode::Parent {
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }

            leaves = next_level;
        }

        Ok(leaves.into_iter().next().unwrap())
    }

    /// Verifies a Merkle proof against the stored Merkle root
    pub fn verify_proof(&self, proof: &MerkleProofNode, merkle_root: &MerkleRoot) -> bool {
        let computed_root = proof.compute_hash();
        computed_root.as_slice() == merkle_root.as_bytes()
    }

    /// Reconstructs the original JSON value from the flattened fields.
    /// This is the reverse operation of `flatten_json`.
    pub fn to_json(&self) -> Result<serde_json::Value, RevealError> {
        use std::str::FromStr;

        let mut root = serde_json::Map::new();

        // Process each field path and reconstruct the nested structure
        for (path, field) in &self.fields {
            // Skip empty paths as they represent the root object itself
            if path.is_empty() {
                continue;
            }

            // Split the path into components
            let parts: Vec<&str> = path.split('.').collect();

            // Helper function to get or create an array at the given path
            fn get_or_create_array<'a>(
                obj: &'a mut serde_json::Map<String, serde_json::Value>,
                key: &str,
            ) -> Result<&'a mut Vec<serde_json::Value>, RevealError> {
                match obj
                    .entry(key.to_string())
                    .or_insert(serde_json::Value::Array(Vec::new()))
                {
                    serde_json::Value::Array(arr) => Ok(arr),
                    _ => Err(RevealError::InvalidField),
                }
            }

            // Helper function to get or create an object
            fn get_or_create_object<'a>(
                value: &'a mut serde_json::Value,
            ) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, RevealError>
            {
                match value {
                    serde_json::Value::Object(obj) => Ok(obj),
                    _ => {
                        *value = serde_json::Value::Object(serde_json::Map::new());
                        match value {
                            serde_json::Value::Object(obj) => Ok(obj),
                            _ => Err(RevealError::InvalidField),
                        }
                    }
                }
            }

            // Navigate the path and create intermediate objects/arrays
            let mut current = &mut root;
            for (i, &part) in parts[..parts.len() - 1].iter().enumerate() {
                if let Ok(idx) = usize::from_str(part) {
                    // Handle array index
                    let array = get_or_create_array(current, parts[i - 1])?;
                    while array.len() <= idx {
                        array.push(serde_json::Value::Object(serde_json::Map::new()));
                    }
                    current = get_or_create_object(&mut array[idx])?;
                } else {
                    // Handle object field
                    let next = current
                        .entry(part.to_string())
                        .or_insert(serde_json::Value::Object(serde_json::Map::new()));
                    current = get_or_create_object(next)?;
                }
            }

            // Handle the last part of the path
            let last = parts.last().ok_or(RevealError::InvalidField)?;
            if let Ok(idx) = usize::from_str(last) {
                // Handle array index
                let array = get_or_create_array(current, parts[parts.len() - 2])?;
                while array.len() <= idx {
                    array.push(serde_json::Value::Null);
                }
                array[idx] = field.value.clone();
            } else {
                // Handle object field
                current.insert(last.to_string(), field.value.clone());
            }
        }

        Ok(serde_json::Value::Object(root))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateMetadata {
    pub issuer_pubkey: nostr::PublicKey,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    pub verification_level: VerificationLevel,
    pub verification_method: VerificationMethod,
    pub salt_sequence: SaltSequence,
    pub merkle_root: MerkleRoot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonData {
    pub full_name: String,
    pub date_of_birth: String,
    pub nationality: String,
    pub document_type: String,
    pub document_number: String,
    pub place_of_birth: Option<String>,
    pub gender: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub address: Option<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessData {
    pub legal_name: String,
    pub trading_name: Option<String>,
    pub registration_number: String,
    pub tax_id: Option<String>,
    pub jurisdiction: String,
    pub incorporation_date: String,
    pub business_type: String,
    pub address: Address,
    pub contact: ContactInfo,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealableField {
    pub value: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("Invalid key")]
    InvalidKey,

    #[error("Invalid merkle root")]
    InvalidMerkleRoot,

    #[error("Certificate already signed")]
    AlreadySigned,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Signing error")]
    SigningFailed,

    #[error("Reveal error: {0}")]
    RevealError(#[from] RevealError),

    #[error("Secp256k1 error: {0}")]
    Secp256k1(#[from] nostr::secp256k1::Error),
}

#[cfg(test)]
mod tests {
    use nostr::nips::nip19::ToBech32;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_generate_key() {
        let key = nostr::Keys::generate();
        println!(
            "{} {:?}",
            key.secret_key().to_bech32().unwrap(),
            key.public_key.to_bech32()
        );
    }

    fn create_test_salt_sequence(num_salts: usize) -> SaltSequence {
        use rand::{RngCore, thread_rng};
        let salt_size = 32;
        let mut rng = thread_rng();
        let mut value = vec![0u8; salt_size * num_salts];
        rng.fill_bytes(&mut value);

        SaltSequence::new(salt_size, value)
    }

    const TEST_KEY: &str = "npub1e7s0pna3h389l5gz6ycjgxrf98kjxzvl3we6csf4upqygfkfzttqtyslhq";

    fn create_test_person_certificate() -> Certificate {
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();

        Certificate::new(
            1,
            subject,
            CertificateData::Person(PersonData {
                full_name: "John Doe".to_string(),
                date_of_birth: "1990-01-01".to_string(),
                nationality: "US".to_string(),
                document_type: "passport".to_string(),
                document_number: "123456789".to_string(),
                place_of_birth: Some("New York".to_string()),
                gender: Some("M".to_string()),
                issue_date: Some("2020-01-01".to_string()),
                expiry_date: Some("2030-01-01".to_string()),
                address: Some(Address {
                    street: "123 Main St".to_string(),
                    city: "New York".to_string(),
                    state: Some("NY".to_string()),
                    postal_code: "10001".to_string(),
                    country: "US".to_string(),
                }),
            }),
            CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::High,
                verification_method: VerificationMethod::InPerson,
                salt_sequence: create_test_salt_sequence(100), // Enough salts for all fields
                merkle_root: MerkleRoot::new([0u8; 32]), // This will be replaced with the computed root
            },
            "".to_string(),
        )
        .unwrap()
    }

    fn create_test_business_certificate() -> Certificate {
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();

        Certificate::new(
            1,
            subject,
            CertificateData::Business(BusinessData {
                legal_name: "Acme Corp".to_string(),
                trading_name: Some("Acme".to_string()),
                registration_number: "REG123456".to_string(),
                tax_id: Some("TAX123456".to_string()),
                jurisdiction: "Delaware".to_string(),
                incorporation_date: "2000-01-01".to_string(),
                business_type: "Corporation".to_string(),
                address: Address {
                    street: "456 Business Ave".to_string(),
                    city: "Dover".to_string(),
                    state: Some("DE".to_string()),
                    postal_code: "19901".to_string(),
                    country: "US".to_string(),
                },
                contact: ContactInfo {
                    email: Some("contact@acme.com".to_string()),
                    phone: Some("+1234567890".to_string()),
                },
                website: Some("https://acme.com".to_string()),
            }),
            CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::High,
                verification_method: VerificationMethod::DocumentUpload,
                salt_sequence: create_test_salt_sequence(100), // Enough salts for all fields
                merkle_root: MerkleRoot::new([0u8; 32]), // This will be replaced with the computed root
            },
            "test_signature".to_string(),
        )
        .unwrap()
    }

    fn create_test_custom_certificate() -> Certificate {
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();

        let custom_data = serde_json::json!({
            "custom_field": "custom_value",
            "nested": {
                "field1": "value1",
                "field2": 42
            },
            "array": ["item1", "item2", "item3"]
        });

        Certificate::new(
            1,
            subject,
            CertificateData::Custom { data: custom_data },
            CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::Medium,
                verification_method: VerificationMethod::Custom("test_method".to_string()),
                salt_sequence: create_test_salt_sequence(100), // Enough salts for all fields
                merkle_root: MerkleRoot::new([0u8; 32]), // This will be replaced with the computed root
            },
            "test_signature".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_prepare_person_certificate() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Check version
        assert_eq!(prepared.version, 1);

        // Check that essential fields exist
        let essential_fields = [
            "full_name",
            "date_of_birth",
            "nationality",
            "document_type",
            "document_number",
        ];

        for field in essential_fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing field: {}",
                field
            );
        }

        // Check optional fields
        let optional_fields = [
            "place_of_birth",
            "gender",
            "address.street",
            "address.city",
            "address.state",
        ];

        for field in optional_fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing optional field: {}",
                field
            );
        }
    }

    #[test]
    fn test_prepare_business_certificate() {
        let cert = create_test_business_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Check that essential business fields exist
        let business_fields = [
            "legal_name",
            "registration_number",
            "jurisdiction",
            "incorporation_date",
            "business_type",
            "address.street",
            "address.city",
            "address.country",
        ];

        for field in business_fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing field: {}",
                field
            );
        }

        // Check optional business fields
        let optional_fields = [
            "trading_name",
            "tax_id",
            "website",
            "contact.email",
            "contact.phone",
        ];

        for field in optional_fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing optional field: {}",
                field
            );
        }
    }

    #[test]
    fn test_prepare_custom_certificate() {
        let cert = create_test_custom_certificate();

        let prepared = cert.prepare_for_revealing().unwrap();

        // Check custom fields
        let custom_fields = [
            "custom_field",
            "nested.field1",
            "nested.field2",
            "array.0",
            "array.1",
            "array.2",
        ];

        for field in custom_fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing field: {}",
                field
            );
        }

        // Check that array values are correct
        assert_eq!(
            prepared
                .fields
                .get("array.0")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "item1"
        );
        assert_eq!(
            prepared
                .fields
                .get("array.1")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "item2"
        );
        assert_eq!(
            prepared
                .fields
                .get("array.2")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "item3"
        );

        // Check that nested object values are correct
        assert_eq!(
            prepared
                .fields
                .get("nested.field1")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "value1"
        );
        assert_eq!(
            prepared
                .fields
                .get("nested.field2")
                .unwrap()
                .value
                .as_u64()
                .unwrap(),
            42
        );
    }

    #[test]
    fn test_field_hashes_are_deterministic() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Verify that hashes are deterministic for each field
        for (i, (key, field)) in prepared.fields.iter().enumerate() {
            let salt = cert
                .metadata
                .salt_sequence
                .get_salt(i)
                .expect("Should have enough salts");
            let first_hash = field.to_hash(key, salt);
            let second_hash = field.to_hash(key, salt);
            assert_eq!(
                first_hash, second_hash,
                "Hash should be deterministic for {}",
                key
            );
            assert_eq!(first_hash.len(), 32, "Hash should be 32 bytes for {}", key);
        }
    }

    #[test]
    fn test_hash_verification() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Verify that the hash is deterministic for each field
        for (i, (key, field)) in prepared.fields.iter().enumerate() {
            let salt = cert
                .metadata
                .salt_sequence
                .get_salt(i)
                .expect("Should have enough salts");
            let first_hash = field.to_hash(key, salt);
            let second_hash = field.to_hash(key, salt);
            assert_eq!(
                first_hash, second_hash,
                "Hash should be deterministic for {}",
                key
            );
            assert_eq!(first_hash.len(), 32, "Hash should be 32 bytes for {}", key);
        }
    }

    #[test]
    fn test_edge_case_null_and_empty_values() {
        let merkle_root = MerkleRoot::new([0u8; 32]); // Default merkle root for testing
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let custom_data = serde_json::json!({
            "null_field": null,
            "empty_string": "",
            "empty_array": [],
            "empty_object": {},
            "array_with_nulls": [null, "value", null],
            "nested": {
                "null_field": null,
                "empty_string": ""
            }
        });

        let cert = Certificate {
            version: 1,
            subject,
            data: CertificateData::Custom { data: custom_data },
            metadata: CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::Medium,
                verification_method: VerificationMethod::Custom("test_method".to_string()),
                salt_sequence: create_test_salt_sequence(100), // Enough salts for all fields
                merkle_root,
            },
            signature: "test_signature".to_string(),
        };

        let prepared = cert.prepare_for_revealing().unwrap();

        // Check that all fields exist
        let fields = [
            "null_field",
            "empty_string",
            "empty_array",
            "empty_object",
            "array_with_nulls",
            "array_with_nulls.0",
            "array_with_nulls.1",
            "array_with_nulls.2",
            "nested.null_field",
            "nested.empty_string",
        ];

        for field in fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing field: {}",
                field
            );
        }

        // Check specific values
        assert!(prepared.fields.get("null_field").unwrap().value.is_null());
        assert_eq!(
            prepared
                .fields
                .get("empty_string")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            ""
        );
        assert!(
            prepared
                .fields
                .get("empty_array")
                .unwrap()
                .value
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            prepared
                .fields
                .get("empty_object")
                .unwrap()
                .value
                .as_object()
                .unwrap()
                .is_empty()
        );
        assert!(
            prepared
                .fields
                .get("array_with_nulls.0")
                .unwrap()
                .value
                .is_null()
        );
        assert_eq!(
            prepared
                .fields
                .get("array_with_nulls.1")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "value"
        );
        assert!(
            prepared
                .fields
                .get("array_with_nulls.2")
                .unwrap()
                .value
                .is_null()
        );
    }

    #[test]
    fn test_edge_case_large_nested_structure() {
        let merkle_root = MerkleRoot::new([0u8; 32]); // Default merkle root for testing
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        // Create a deeply nested structure
        let mut nested_value = serde_json::json!({
            "value": "deepest"
        });

        // Create 100 levels of nesting
        for i in (0..100).rev() {
            nested_value = serde_json::json!({
                format!("level_{}", i): nested_value
            });
        }

        let cert = Certificate {
            version: 1,
            subject,
            data: CertificateData::Custom { data: nested_value },
            metadata: CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::Medium,
                verification_method: VerificationMethod::Custom("test_method".to_string()),
                salt_sequence: create_test_salt_sequence(1),
                merkle_root,
            },
            signature: "test_signature".to_string(),
        };

        let prepared = cert.prepare_for_revealing().unwrap();

        // Check that we can access the deepest value
        let mut path = String::new();
        for i in 0..100 {
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(&format!("level_{}", i));
        }
        path.push_str(".value");

        assert!(prepared.fields.contains_key(&path), "Missing deepest field");
        assert_eq!(
            prepared.fields.get(&path).unwrap().value.as_str().unwrap(),
            "deepest"
        );
    }

    #[test]
    fn test_edge_case_special_characters() {
        let merkle_root = MerkleRoot::new([0u8; 32]); // Default merkle root for testing
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let custom_data = serde_json::json!({
            "field.with.dots": "value",
            "field\\with\\backslashes": "value",
            "field with spaces": "value",
            "field\nwith\nnewlines": "value",
            "field\"with\"quotes": "value",
            "field'with'quotes": "value",
            "field/with/slashes": "value",
            "array": ["item.with.dots", "item\\with\\backslashes", "item\nwith\nnewlines"],
            "nested": {
                "field.with.special\nchars": "value",
                "emoji": "🦀🔒💻"
            }
        });

        let cert = Certificate {
            version: 1,
            subject,
            data: CertificateData::Custom { data: custom_data },
            metadata: CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::Medium,
                verification_method: VerificationMethod::Custom("test_method".to_string()),
                salt_sequence: create_test_salt_sequence(100), // Enough salts for all fields
                merkle_root,
            },
            signature: "test_signature".to_string(),
        };

        let prepared = cert.prepare_for_revealing().unwrap();

        // Verify that all fields exist and their values are preserved
        let fields = [
            "field.with.dots",
            "field\\with\\backslashes",
            "field with spaces",
            "field\nwith\nnewlines",
            "field\"with\"quotes",
            "field'with'quotes",
            "field/with/slashes",
            "array.0",
            "array.1",
            "array.2",
            "nested.field.with.special\nchars",
            "nested.emoji",
        ];

        for field in fields {
            assert!(
                prepared.fields.contains_key(field),
                "Missing field: {}",
                field
            );
            assert!(
                prepared.fields[field].value.is_string(),
                "Field should be a string: {}",
                field
            );
        }

        // Check that emoji are preserved
        assert_eq!(
            prepared
                .fields
                .get("nested.emoji")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "🦀🔒💻"
        );

        // Verify that hashes are deterministic for all fields
        for (i, (key, field)) in prepared.fields.iter().enumerate() {
            let salt = cert
                .metadata
                .salt_sequence
                .get_salt(i)
                .expect("Should have enough salts");
            let first_hash = field.to_hash(key, salt);
            let second_hash = field.to_hash(key, salt);
            assert_eq!(
                first_hash, second_hash,
                "Hash should be deterministic for {}",
                key
            );
            assert_eq!(first_hash.len(), 32, "Hash should be 32 bytes for {}", key);
        }
    }

    #[test]
    fn test_merkle_root_is_deterministic() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        let root1 = prepared
            .compute_merkle_root(&cert.metadata.salt_sequence)
            .unwrap();
        let root2 = prepared
            .compute_merkle_root(&cert.metadata.salt_sequence)
            .unwrap();

        assert_eq!(root1, root2, "Merkle root should be deterministic");
        assert_eq!(root1.len(), 32, "Merkle root should be 32 bytes");
    }

    #[test]
    fn test_merkle_root_changes_with_different_salts() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        let root1 = prepared
            .compute_merkle_root(&cert.metadata.salt_sequence)
            .unwrap();

        // Create a different salt sequence
        let different_salt_sequence = create_test_salt_sequence(100);
        let root2 = prepared
            .compute_merkle_root(&different_salt_sequence)
            .unwrap();

        assert_ne!(
            root1, root2,
            "Merkle root should be different with different salts"
        );
    }

    #[test]
    fn test_merkle_root_with_empty_fields() {
        let merkle_root = MerkleRoot::new([0u8; 32]); // Default merkle root for testing
        let subject = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let issuer = nostr::PublicKey::from_str(TEST_KEY).unwrap();
        let custom_data = serde_json::json!({});
        let cert = Certificate {
            version: 1,
            subject,
            data: CertificateData::Custom { data: custom_data },
            metadata: CertificateMetadata {
                issuer_pubkey: issuer,
                issued_at: Timestamp::new(1234567890),
                expires_at: Timestamp::new(9876543210),
                verification_level: VerificationLevel::Medium,
                verification_method: VerificationMethod::Custom("test_method".to_string()),
                salt_sequence: create_test_salt_sequence(1),
                merkle_root,
            },
            signature: "test_signature".to_string(),
        };

        let prepared = cert.prepare_for_revealing().unwrap();
        let root = prepared
            .compute_merkle_root(&cert.metadata.salt_sequence)
            .unwrap();

        assert_eq!(
            root.len(),
            32,
            "Merkle root should be 32 bytes even for empty fields"
        );
    }

    #[test]
    fn test_merkle_root_insufficient_salts() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Create a salt sequence with too few salts
        let insufficient_salt_sequence = create_test_salt_sequence(1);
        let result = prepared.compute_merkle_root(&insufficient_salt_sequence);

        assert!(matches!(result, Err(RevealError::InsufficientSalts)));
    }

    #[test]
    fn test_merkle_proof() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Try to reveal a few fields
        let reveal_fields = vec![
            "full_name".to_string(),
            "nationality".to_string(),
            "address.city".to_string(),
        ];

        let proof = prepared
            .create_proof(&cert.metadata.salt_sequence, &reveal_fields)
            .unwrap();

        // Verify that the proof is valid
        assert!(prepared.verify_proof(&proof, &cert.metadata.merkle_root));

        // Verify that we can extract the revealed fields
        fn find_revealed_fields(node: &MerkleProofNode) -> Vec<(String, serde_json::Value)> {
            let mut fields = Vec::new();
            match node {
                MerkleProofNode::Leaf(MerkleProofLeaf::Cleartext { name, field, .. }) => {
                    fields.push((name.clone(), field.value.clone()));
                }
                MerkleProofNode::Parent { left, right } => {
                    fields.extend(find_revealed_fields(left));
                    fields.extend(find_revealed_fields(right));
                }
                _ => {}
            }
            fields
        }

        let revealed = find_revealed_fields(&proof);
        assert_eq!(revealed.len(), 3);

        let mut revealed_map: BTreeMap<_, _> = revealed.into_iter().collect();
        assert_eq!(
            revealed_map.remove("full_name").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(
            revealed_map
                .remove("nationality")
                .unwrap()
                .as_str()
                .unwrap(),
            "US"
        );
        assert_eq!(
            revealed_map
                .remove("address.city")
                .unwrap()
                .as_str()
                .unwrap(),
            "New York"
        );

        // Try to create an invalid proof by modifying a revealed field
        let mut modified_proof = proof.clone();
        fn modify_first_cleartext(node: &mut MerkleProofNode) {
            match node {
                MerkleProofNode::Leaf(MerkleProofLeaf::Cleartext { field, .. }) => {
                    field.value = serde_json::json!("Jane Doe");
                }
                MerkleProofNode::Parent { left, right } => {
                    modify_first_cleartext(left);
                    modify_first_cleartext(right);
                }
                _ => {}
            }
        }
        modify_first_cleartext(&mut modified_proof);

        // Verify that the modified proof is invalid
        assert!(!prepared.verify_proof(&modified_proof, &cert.metadata.merkle_root));
    }

    #[test]
    fn test_sign() {
        let mut cert = create_test_person_certificate();
        let issuer_key = nostr::Keys::generate();

        // Update the issuer pubkey to match our test key
        cert.metadata.issuer_pubkey = issuer_key.public_key();

        // First sign should succeed
        cert.sign(&issuer_key).expect("Failed to sign certificate");
        assert!(!cert.signature.is_empty());

        // Second sign should fail
        assert!(matches!(
            cert.sign(&issuer_key),
            Err(SignError::AlreadySigned)
        ));

        // Sign with wrong key should fail
        let wrong_key = nostr::Keys::generate();
        let mut cert = create_test_person_certificate();
        assert!(matches!(cert.sign(&wrong_key), Err(SignError::InvalidKey)));

        // Sign with invalid merkle root should fail
        let mut cert = create_test_person_certificate();
        cert.metadata.issuer_pubkey = issuer_key.public_key();
        cert.metadata.merkle_root = MerkleRoot::new([1u8; 32]); // Wrong root
        assert!(matches!(
            cert.sign(&issuer_key),
            Err(SignError::InvalidMerkleRoot)
        ));
    }

    #[test]
    fn test_merkle_proof_to_prepared_certificate() {
        let cert = create_test_person_certificate();
        let prepared = cert.prepare_for_revealing().unwrap();

        // Try to reveal a few fields
        let reveal_fields = vec![
            "full_name".to_string(),
            "nationality".to_string(),
            "address.city".to_string(),
        ];

        let proof = prepared
            .create_proof(&cert.metadata.salt_sequence, &reveal_fields)
            .unwrap();

        // Convert proof back to prepared certificate
        let revealed_cert = proof.to_prepared_certificate();

        // Check that only the revealed fields are present
        assert_eq!(revealed_cert.fields.len(), 3);
        assert!(revealed_cert.fields.contains_key("full_name"));
        assert!(revealed_cert.fields.contains_key("nationality"));
        assert!(revealed_cert.fields.contains_key("address.city"));

        // Check that the values match the original
        assert_eq!(
            revealed_cert
                .fields
                .get("full_name")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "John Doe"
        );
        assert_eq!(
            revealed_cert
                .fields
                .get("nationality")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "US"
        );
        assert_eq!(
            revealed_cert
                .fields
                .get("address.city")
                .unwrap()
                .value
                .as_str()
                .unwrap(),
            "New York"
        );

        // Convert back to JSON and verify structure
        let json = revealed_cert.to_json().unwrap();
        assert_eq!(json.get("full_name").unwrap().as_str().unwrap(), "John Doe");
        assert_eq!(json.get("nationality").unwrap().as_str().unwrap(), "US");
        assert_eq!(
            json.get("address")
                .unwrap()
                .get("city")
                .unwrap()
                .as_str()
                .unwrap(),
            "New York"
        );
    }

    #[test]
    fn test_partial_certificate_verification() {
        // Create and sign a test certificate
        let mut cert = create_test_person_certificate();
        let issuer_key = nostr::Keys::generate();
        cert.metadata.issuer_pubkey = issuer_key.public_key();
        cert.sign(&issuer_key).expect("Failed to sign certificate");

        // Prepare it for revealing and create a proof with some fields
        let prepared = cert.prepare_for_revealing().unwrap();
        let reveal_fields = vec![
            "full_name".to_string(),
            "nationality".to_string(),
            "address.city".to_string(),
        ];
        let proof = prepared
            .create_proof(&cert.metadata.salt_sequence, &reveal_fields)
            .unwrap();

        // Create a valid partial certificate
        let valid_partial = PartialCertificate {
            version: cert.version,
            subject: cert.subject,
            metadata: cert.metadata.clone(),
            signature: cert.signature.clone(),
            merkle_proof: proof,
        };

        // Test successful verification
        let result = valid_partial.verify().unwrap();
        assert_eq!(
            result.get("full_name").unwrap().as_str().unwrap(),
            "John Doe"
        );
        assert_eq!(result.get("nationality").unwrap().as_str().unwrap(), "US");
        assert_eq!(
            result
                .get("address")
                .unwrap()
                .get("city")
                .unwrap()
                .as_str()
                .unwrap(),
            "New York"
        );

        // Test invalid merkle root
        let mut invalid_root = valid_partial.clone();
        invalid_root.metadata.merkle_root = MerkleRoot::new([1u8; 32]); // Wrong root
        assert!(matches!(
            invalid_root.verify(),
            Err(VerifyError::InvalidMerkleRoot)
        ));

        // Test invalid signature
        let mut invalid_sig = valid_partial.clone();
        invalid_sig.signature = hex::encode([1u8; 64]); // Wrong signature
        assert!(matches!(
            invalid_sig.verify(),
            Err(VerifyError::InvalidSignature)
        ));

        // Test wrong issuer key
        let mut wrong_issuer = valid_partial.clone();
        wrong_issuer.metadata.issuer_pubkey = nostr::Keys::generate().public_key();
        assert!(matches!(
            wrong_issuer.verify(),
            Err(VerifyError::InvalidSignature)
        ));

        // Test tampered proof (modify a revealed field)
        let mut tampered = valid_partial.clone();
        fn modify_first_cleartext(node: &mut MerkleProofNode) {
            match node {
                MerkleProofNode::Leaf(MerkleProofLeaf::Cleartext { field, .. }) => {
                    field.value = serde_json::json!("Jane Doe");
                }
                MerkleProofNode::Parent { left, right } => {
                    modify_first_cleartext(left);
                    modify_first_cleartext(right);
                }
                _ => {}
            }
        }
        modify_first_cleartext(&mut tampered.merkle_proof);
        assert!(matches!(
            tampered.verify(),
            Err(VerifyError::InvalidMerkleRoot)
        ));
    }
}
