use super::{
    Digest, Felt, Hasher, ProvenTransaction, TransactionResult, Vec, Word, WORD_SIZE, ZERO,
};
use crate::utils::serde::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};

// TRANSACTION ID
// ================================================================================================

/// A unique identifier of a transaction.
///
/// Transaction ID is computed as:
///
/// hash(init_account_hash, final_account_hash, input_notes_hash, output_notes_hash)
///
/// This achieves the following properties:
/// - Transactions are identical if and only if they have the same ID.
/// - Computing transaction ID can be done solely from public transaction data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionId(Digest);

impl TransactionId {
    /// Returns a new [TransactionId] instantiated from the provided transaction components.
    pub fn new(
        init_account_hash: Digest,
        final_account_hash: Digest,
        input_notes_hash: Digest,
        output_notes_hash: Digest,
    ) -> Self {
        let mut elements = [ZERO; 4 * WORD_SIZE];
        elements[..4].copy_from_slice(init_account_hash.as_elements());
        elements[4..8].copy_from_slice(final_account_hash.as_elements());
        elements[8..12].copy_from_slice(input_notes_hash.as_elements());
        elements[12..].copy_from_slice(output_notes_hash.as_elements());
        Self(Hasher::hash_elements(&elements))
    }

    /// Returns the elements of this transaction ID.
    pub fn as_elements(&self) -> &[Felt] {
        self.0.as_elements()
    }

    /// Returns the digest defining this transaction ID.
    pub fn inner(&self) -> Digest {
        self.0
    }
}

// CONVERSIONS INTO TRANSACTION ID
// ================================================================================================

impl From<&ProvenTransaction> for TransactionId {
    fn from(tx: &ProvenTransaction) -> Self {
        // TODO: move input/output note hash computations into a more central location
        let input_notes_hash = {
            let mut elements: Vec<Felt> = Vec::with_capacity(tx.consumed_notes().len() * 8);
            for nullifier in tx.consumed_notes().iter() {
                elements.extend_from_slice(nullifier.as_elements());
                elements.extend_from_slice(&Word::default());
            }
            Hasher::hash_elements(&elements)
        };
        let output_notes_hash = {
            let mut elements: Vec<Felt> = Vec::with_capacity(tx.created_notes().len() * 8);
            for note in tx.created_notes().iter() {
                elements.extend_from_slice(note.note_hash().as_elements());
                elements.extend_from_slice(&Word::from(note.metadata()));
            }
            Hasher::hash_elements(&elements)
        };
        Self::new(
            tx.initial_account_hash(),
            tx.final_account_hash(),
            input_notes_hash,
            output_notes_hash,
        )
    }
}

impl From<&TransactionResult> for TransactionId {
    fn from(tx: &TransactionResult) -> Self {
        let input_notes_hash = tx.consumed_notes().commitment();
        let output_notes_hash = tx.created_notes().commitment();
        Self::new(
            tx.initial_account_hash(),
            tx.final_account_hash(),
            input_notes_hash,
            output_notes_hash,
        )
    }
}

impl From<Word> for TransactionId {
    fn from(value: Word) -> Self {
        Self(value.into())
    }
}

impl From<Digest> for TransactionId {
    fn from(value: Digest) -> Self {
        Self(value)
    }
}

// CONVERSIONS FROM TRANSACTION ID
// ================================================================================================

impl From<TransactionId> for Word {
    fn from(id: TransactionId) -> Self {
        id.0.into()
    }
}

impl From<TransactionId> for [u8; 32] {
    fn from(id: TransactionId) -> Self {
        id.0.into()
    }
}

impl From<&TransactionId> for Word {
    fn from(id: &TransactionId) -> Self {
        id.0.into()
    }
}

impl From<&TransactionId> for [u8; 32] {
    fn from(id: &TransactionId) -> Self {
        id.0.into()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for TransactionId {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&self.0.to_bytes());
    }
}

impl Deserializable for TransactionId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = Digest::read_from(source)?;
        Ok(Self(id))
    }
}
