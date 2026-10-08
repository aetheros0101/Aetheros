use serde::{Serialize, de::DeserializeOwned};

pub trait StructuredOutput: Serialize + DeserializeOwned + Send + Sync {}

impl<T> StructuredOutput for T where T: Serialize + DeserializeOwned + Send + Sync {}
