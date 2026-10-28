//! # Soroban Payout Transaction Builder & Signer
//!
//! Constructs the `payout(recipient, proof_hash)` contract invocation according
//! to the Phase 1 interface and signs it with the Oracle Ed25519 key.

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use stellar_strkey::ed25519::PrivateKey;
use stellar_strkey::Strkey;
use stellar_xdr::curr::{
    AccountId, DecoratedSignature, Hash, HostFunction, InvokeContractArgs, InvokeHostFunctionOp,
    Limits, Memo, MuxedAccount, Operation, OperationBody, Preconditions, PublicKey, ScAddress,
    ScBytes, ScSymbol, ScVal, SequenceNumber, Signature, SignatureHint, Transaction,
    TransactionEnvelope, TransactionExt, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, TransactionV1Envelope, Uint256, WriteXdr,
};

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Invalid Stellar strkey: {0}")]
    InvalidStrkey(String),
    #[error("XDR conversion error: {0}")]
    Xdr(String),
}

/// Builds and signs a Soroban `payout` transaction envelope.
pub struct PayoutTxBuilder;

impl PayoutTxBuilder {
    /// Builds an unsigned `InvokeHostFunctionOp` invocation for `payout(recipient, proof_hash)`.
    pub fn build_payout_op(
        contract_id: &str,
        recipient_addr: &str,
        proof_hash: &[u8; 32],
    ) -> Result<Operation, BuilderError> {
        let contract_bytes = match Strkey::from_string(contract_id) {
            Ok(Strkey::Contract(c)) => c.0,
            _ => return Err(BuilderError::InvalidStrkey(contract_id.to_string())),
        };

        let recipient_bytes = match Strkey::from_string(recipient_addr) {
            Ok(Strkey::PublicKeyEd25519(pk)) => pk.0,
            _ => return Err(BuilderError::InvalidStrkey(recipient_addr.to_string())),
        };

        let contract_sc_addr = ScAddress::Contract(Hash(contract_bytes));
        let recipient_sc_addr = ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(
            Uint256(recipient_bytes),
        )));

        let fn_name: ScSymbol = "payout"
            .try_into()
            .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?;

        let proof_bytes: ScBytes = proof_hash
            .to_vec()
            .try_into()
            .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?;

        let args = vec![ScVal::Address(recipient_sc_addr), ScVal::Bytes(proof_bytes)];

        let invoke_args = InvokeContractArgs {
            contract_address: contract_sc_addr,
            function_name: fn_name,
            args: args
                .try_into()
                .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?,
        };

        let op = Operation {
            source_account: None,
            body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
                host_function: HostFunction::InvokeContract(invoke_args),
                auth: vec![]
                    .try_into()
                    .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?,
            }),
        };

        Ok(op)
    }

    /// Builds and signs the transaction envelope returning the base64-encoded XDR.
    pub fn build_and_sign_envelope(
        signing_key: &SigningKey,
        network_passphrase: &str,
        contract_id: &str,
        recipient_addr: &str,
        proof_hash: &[u8; 32],
        sequence: i64,
        fee: u32,
    ) -> Result<(TransactionEnvelope, String), BuilderError> {
        let verifying_key = signing_key.verifying_key();
        let pub_bytes = verifying_key.to_bytes();

        let op = Self::build_payout_op(contract_id, recipient_addr, proof_hash)?;

        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(pub_bytes)),
            fee,
            seq_num: SequenceNumber(sequence),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![op]
                .try_into()
                .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?,
            ext: TransactionExt::V0,
        };

        let mut hasher = Sha256::new();
        hasher.update(network_passphrase.as_bytes());
        let network_id: [u8; 32] = hasher.finalize().into();

        let payload = TransactionSignaturePayload {
            network_id: Hash(network_id),
            tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
        };

        let payload_bytes = payload
            .to_xdr(Limits::none())
            .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?;

        let mut hash_payload = Sha256::new();
        hash_payload.update(&payload_bytes);
        let tx_hash: [u8; 32] = hash_payload.finalize().into();

        let signature = signing_key.sign(&tx_hash);

        let mut hint = [0u8; 4];
        hint.copy_from_slice(&pub_bytes[28..32]);

        let decorated_sig = DecoratedSignature {
            hint: SignatureHint(hint),
            signature: Signature(
                signature
                    .to_bytes()
                    .to_vec()
                    .try_into()
                    .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?,
            ),
        };

        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: vec![decorated_sig]
                .try_into()
                .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?,
        });

        let xdr_base64 = envelope
            .to_xdr_base64(Limits::none())
            .map_err(|e| BuilderError::Xdr(format!("{e:?}")))?;

        Ok((envelope, xdr_base64))
    }

    /// Helper to convert a Stellar secret key string (`S...`) into an `ed25519_dalek::SigningKey`.
    pub fn parse_signing_key(secret_strkey: &str) -> Result<SigningKey, BuilderError> {
        let pk = PrivateKey::from_string(secret_strkey)
            .map_err(|e| BuilderError::InvalidStrkey(format!("{e:?}")))?;
        Ok(SigningKey::from_bytes(&pk.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;
    use stellar_xdr::curr::ReadXdr;

    #[test]
    fn test_golden_payout_envelope_structure() {
        let raw_seed = [42u8; 32];
        let private_key = PrivateKey(raw_seed);
        let secret = private_key.as_unredacted().to_string();
        let signing_key = PayoutTxBuilder::parse_signing_key(&secret).unwrap();

        let contract_id = "CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF";
        let recipient = "GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT";
        let proof_hash = [0xabu8; 32];
        let passphrase = "Test SDF Network ; September 2015";

        let (_envelope, xdr_base64) = PayoutTxBuilder::build_and_sign_envelope(
            &signing_key,
            passphrase,
            contract_id,
            recipient,
            &proof_hash,
            100,
            1000,
        )
        .unwrap();

        assert!(!xdr_base64.is_empty());

        let decoded = TransactionEnvelope::from_xdr_base64(xdr_base64, Limits::none()).unwrap();
        match decoded {
            TransactionEnvelope::Tx(v1) => {
                assert_eq!(v1.tx.fee, 1000);
                assert_eq!(v1.tx.seq_num.0, 100);
                assert_eq!(v1.signatures.len(), 1);

                let op = &v1.tx.operations[0];
                match &op.body {
                    OperationBody::InvokeHostFunction(invoke) => match &invoke.host_function {
                        HostFunction::InvokeContract(args) => {
                            assert_eq!(args.function_name.0.to_string(), "payout");
                            assert_eq!(args.args.len(), 2);
                        }
                        _ => panic!("Expected InvokeContract"),
                    },
                    _ => panic!("Expected InvokeHostFunction"),
                }

                // Verify signature against public key
                let sig = &v1.signatures[0];
                let vk = signing_key.verifying_key();
                let dalek_sig =
                    ed25519_dalek::Signature::from_slice(sig.signature.0.as_slice()).unwrap();

                let mut hasher = Sha256::new();
                hasher.update(passphrase.as_bytes());
                let network_id: [u8; 32] = hasher.finalize().into();

                let payload = TransactionSignaturePayload {
                    network_id: Hash(network_id),
                    tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(
                        v1.tx.clone(),
                    ),
                };
                let payload_bytes = payload.to_xdr(Limits::none()).unwrap();

                let mut hash_payload = Sha256::new();
                hash_payload.update(&payload_bytes);
                let tx_hash: [u8; 32] = hash_payload.finalize().into();

                assert!(vk.verify(&tx_hash, &dalek_sig).is_ok());
            }
            _ => panic!("Expected TransactionEnvelope::Tx"),
        }
    }
}
