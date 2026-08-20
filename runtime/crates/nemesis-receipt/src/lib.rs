use std::collections::BTreeSet;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use minicbor::{Decoder, Encoder};
use thiserror::Error;

const RECEIPT_SCHEMA: &str = "nemesis.receipt/v1";
const COSE_PROTECTED_EDDSA: &[u8] = &[0xa1, 0x01, 0x27];
const MAX_CLAIMS: usize = 64;
const MAX_RESIDUALS: usize = 64;
const MAX_TEXT: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpistemicStatus {
    Verified,
    Believed,
    Unknown,
}

impl EpistemicStatus {
    fn code(self) -> u8 {
        match self {
            Self::Verified => 0,
            Self::Believed => 1,
            Self::Unknown => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self, ReceiptError> {
        match code {
            0 => Ok(Self::Verified),
            1 => Ok(Self::Believed),
            2 => Ok(Self::Unknown),
            _ => Err(ReceiptError::Malformed(
                "invalid epistemic status".to_owned(),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimReceipt {
    pub id: String,
    pub mandatory: bool,
    pub status: EpistemicStatus,
    pub evidence_digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptPayload {
    pub mission_id: String,
    pub event_sequence: u64,
    pub terminal_state: String,
    pub source_digest: [u8; 32],
    pub ledger_head: [u8; 32],
    pub claims: Vec<ClaimReceipt>,
    pub residuals: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedReceipt {
    pub payload: ReceiptPayload,
    pub payload_cbor: Vec<u8>,
}

impl VerifiedReceipt {
    pub fn is_bound_to(&self, source_digest: &[u8; 32]) -> bool {
        &self.payload.source_digest == source_digest
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReceiptError {
    #[error("receipt encoding failed: {0}")]
    Encode(String),
    #[error("malformed receipt: {0}")]
    Malformed(String),
    #[error("unsupported receipt or COSE header")]
    Unsupported,
    #[error("receipt signature verification failed")]
    Signature,
    #[error("mandatory completion claims are not all VERIFIED")]
    IncompleteClaims,
}

fn valid_id(value: &str, prefix: &str) -> bool {
    value.len() == 26
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

fn validate_shape(payload: &ReceiptPayload) -> Result<(), ReceiptError> {
    if !valid_id(&payload.mission_id, "mis_")
        || payload.event_sequence == 0
        || payload.claims.is_empty()
        || payload.claims.len() > MAX_CLAIMS
        || payload.residuals.len() > MAX_RESIDUALS
    {
        return Err(ReceiptError::Malformed(
            "payload identity, sequence, or collection bounds are invalid".to_owned(),
        ));
    }
    let mut claim_ids = BTreeSet::new();
    for claim in &payload.claims {
        if !valid_label(&claim.id) || !claim_ids.insert(claim.id.as_str()) {
            return Err(ReceiptError::Malformed(
                "claim identifiers must be unique bounded labels".to_owned(),
            ));
        }
    }
    if payload
        .residuals
        .iter()
        .any(|residual| residual.is_empty() || residual.len() > MAX_TEXT)
    {
        return Err(ReceiptError::Malformed(
            "residual text is empty or oversized".to_owned(),
        ));
    }
    Ok(())
}

fn encode_payload(payload: &ReceiptPayload) -> Result<Vec<u8>, ReceiptError> {
    validate_shape(payload)?;
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes);
    encoder
        .array(8)
        .and_then(|encoder| encoder.str(RECEIPT_SCHEMA))
        .and_then(|encoder| encoder.str(&payload.mission_id))
        .and_then(|encoder| encoder.u64(payload.event_sequence))
        .and_then(|encoder| encoder.str(&payload.terminal_state))
        .and_then(|encoder| encoder.bytes(&payload.source_digest))
        .and_then(|encoder| encoder.bytes(&payload.ledger_head))
        .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    encoder
        .array(payload.claims.len() as u64)
        .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    for claim in &payload.claims {
        encoder
            .array(4)
            .and_then(|encoder| encoder.str(&claim.id))
            .and_then(|encoder| encoder.bool(claim.mandatory))
            .and_then(|encoder| encoder.u8(claim.status.code()))
            .and_then(|encoder| encoder.bytes(&claim.evidence_digest))
            .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    }
    encoder
        .array(payload.residuals.len() as u64)
        .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    for residual in &payload.residuals {
        encoder
            .str(residual)
            .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    }
    Ok(bytes)
}

fn signature_structure(payload_cbor: &[u8]) -> Result<Vec<u8>, ReceiptError> {
    let mut bytes = Vec::new();
    Encoder::new(&mut bytes)
        .array(4)
        .and_then(|encoder| encoder.str("Signature1"))
        .and_then(|encoder| encoder.bytes(COSE_PROTECTED_EDDSA))
        .and_then(|encoder| encoder.bytes(&[]))
        .and_then(|encoder| encoder.bytes(payload_cbor))
        .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    Ok(bytes)
}

pub fn sign1(payload: &ReceiptPayload, signing_key: &SigningKey) -> Result<Vec<u8>, ReceiptError> {
    let payload_cbor = encode_payload(payload)?;
    let to_sign = signature_structure(&payload_cbor)?;
    let signature = signing_key.sign(&to_sign).to_bytes();
    let mut receipt = Vec::new();
    Encoder::new(&mut receipt)
        .array(4)
        .and_then(|encoder| encoder.bytes(COSE_PROTECTED_EDDSA))
        .and_then(|encoder| encoder.map(0))
        .and_then(|encoder| encoder.bytes(&payload_cbor))
        .and_then(|encoder| encoder.bytes(&signature))
        .map_err(|error| ReceiptError::Encode(error.to_string()))?;
    Ok(receipt)
}

fn exact_array(decoder: &mut Decoder<'_>, expected: u64) -> Result<(), ReceiptError> {
    match decoder.array() {
        Ok(Some(actual)) if actual == expected => Ok(()),
        Ok(_) => Err(ReceiptError::Malformed(format!(
            "expected array of length {expected}"
        ))),
        Err(error) => Err(ReceiptError::Malformed(error.to_string())),
    }
}

fn exact_map(decoder: &mut Decoder<'_>, expected: u64) -> Result<(), ReceiptError> {
    match decoder.map() {
        Ok(Some(actual)) if actual == expected => Ok(()),
        Ok(_) => Err(ReceiptError::Malformed(format!(
            "expected map of length {expected}"
        ))),
        Err(error) => Err(ReceiptError::Malformed(error.to_string())),
    }
}

fn decode_digest(decoder: &mut Decoder<'_>) -> Result<[u8; 32], ReceiptError> {
    let bytes = decoder
        .bytes()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?;
    bytes
        .try_into()
        .map_err(|_| ReceiptError::Malformed("digest must contain 32 bytes".to_owned()))
}

fn decode_payload(bytes: &[u8]) -> Result<ReceiptPayload, ReceiptError> {
    let mut decoder = Decoder::new(bytes);
    exact_array(&mut decoder, 8)?;
    if decoder
        .str()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        != RECEIPT_SCHEMA
    {
        return Err(ReceiptError::Unsupported);
    }
    let mission_id = decoder
        .str()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        .to_owned();
    let event_sequence = decoder
        .u64()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?;
    let terminal_state = decoder
        .str()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        .to_owned();
    let source_digest = decode_digest(&mut decoder)?;
    let ledger_head = decode_digest(&mut decoder)?;
    let claim_count = decoder
        .array()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        .ok_or_else(|| ReceiptError::Malformed("indefinite claim array".to_owned()))?;
    if claim_count == 0 || claim_count > MAX_CLAIMS as u64 {
        return Err(ReceiptError::Malformed("invalid claim count".to_owned()));
    }
    let mut claims = Vec::with_capacity(claim_count as usize);
    for _ in 0..claim_count {
        exact_array(&mut decoder, 4)?;
        claims.push(ClaimReceipt {
            id: decoder
                .str()
                .map_err(|error| ReceiptError::Malformed(error.to_string()))?
                .to_owned(),
            mandatory: decoder
                .bool()
                .map_err(|error| ReceiptError::Malformed(error.to_string()))?,
            status: EpistemicStatus::from_code(
                decoder
                    .u8()
                    .map_err(|error| ReceiptError::Malformed(error.to_string()))?,
            )?,
            evidence_digest: decode_digest(&mut decoder)?,
        });
    }
    let residual_count = decoder
        .array()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        .ok_or_else(|| ReceiptError::Malformed("indefinite residual array".to_owned()))?;
    if residual_count > MAX_RESIDUALS as u64 {
        return Err(ReceiptError::Malformed("invalid residual count".to_owned()));
    }
    let mut residuals = Vec::with_capacity(residual_count as usize);
    for _ in 0..residual_count {
        residuals.push(
            decoder
                .str()
                .map_err(|error| ReceiptError::Malformed(error.to_string()))?
                .to_owned(),
        );
    }
    if decoder.position() != bytes.len() {
        return Err(ReceiptError::Malformed("trailing payload bytes".to_owned()));
    }
    let payload = ReceiptPayload {
        mission_id,
        event_sequence,
        terminal_state,
        source_digest,
        ledger_head,
        claims,
        residuals,
    };
    validate_shape(&payload)?;
    Ok(payload)
}

pub fn verify_sign1(
    receipt: &[u8],
    verifying_key: &VerifyingKey,
) -> Result<VerifiedReceipt, ReceiptError> {
    let mut decoder = Decoder::new(receipt);
    exact_array(&mut decoder, 4)?;
    let protected = decoder
        .bytes()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?;
    if protected != COSE_PROTECTED_EDDSA {
        return Err(ReceiptError::Unsupported);
    }
    exact_map(&mut decoder, 0)?;
    let payload_cbor = decoder
        .bytes()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?
        .to_vec();
    let signature_bytes = decoder
        .bytes()
        .map_err(|error| ReceiptError::Malformed(error.to_string()))?;
    let signature = Signature::from_slice(signature_bytes).map_err(|_| ReceiptError::Signature)?;
    if decoder.position() != receipt.len() {
        return Err(ReceiptError::Malformed("trailing COSE bytes".to_owned()));
    }
    let to_verify = signature_structure(&payload_cbor)?;
    verifying_key
        .verify_strict(&to_verify, &signature)
        .map_err(|_| ReceiptError::Signature)?;
    let payload = decode_payload(&payload_cbor)?;
    if payload.terminal_state != "COMPLETE"
        || payload
            .claims
            .iter()
            .any(|claim| claim.mandatory && claim.status != EpistemicStatus::Verified)
    {
        return Err(ReceiptError::IncompleteClaims);
    }
    Ok(VerifiedReceipt {
        payload,
        payload_cbor,
    })
}
