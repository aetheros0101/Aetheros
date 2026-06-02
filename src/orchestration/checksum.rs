use sha2::{
    Digest,
    Sha256,
};

pub fn execution_checksum(
    payload: &[u8],
) -> String {
    let mut hasher =
        Sha256::new();

    hasher.update(payload);
      hex::encode(hasher.finalize())
}
