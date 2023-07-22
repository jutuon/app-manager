/*
 * app-manager
 *
 * App manager API
 *
 * The version of the OpenAPI document: 0.1.0
 * 
 * Generated by: https://openapi-generator.tech
 */


/// 
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DownloadType {
    #[serde(rename = "Info")]
    Info,
    #[serde(rename = "EncryptedBinary")]
    EncryptedBinary,

}

impl ToString for DownloadType {
    fn to_string(&self) -> String {
        match self {
            Self::Info => String::from("Info"),
            Self::EncryptedBinary => String::from("EncryptedBinary"),
        }
    }
}

impl Default for DownloadType {
    fn default() -> DownloadType {
        Self::Info
    }
}




