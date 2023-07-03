use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};


#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default, PartialEq, Eq)]
pub struct DataEncryptionKey {
    /// Base64 key
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, IntoParams)]
pub struct ServerNameText {
    pub server: String,
}
