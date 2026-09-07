use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("storage failure")]
    StorageFailure,

    #[error("snapshot failure")]
    SnapshotFailure,

    #[error("recovery failure")]
    RecoveryFailure,

    #[error("serialization failure")]
    SerializationFailure,

    #[error("cryptographic failure")]
    CryptoFailure,
}
