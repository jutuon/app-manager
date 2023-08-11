/*
 * app-manager
 *
 * App manager API
 *
 * The version of the OpenAPI document: 0.1.0
 * 
 * Generated by: https://openapi-generator.tech
 */

/// RebootQueryParam : Reboot computer directly after software update.



#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct RebootQueryParam {
    #[serde(rename = "reboot")]
    pub reboot: bool,
}

impl RebootQueryParam {
    /// Reboot computer directly after software update.
    pub fn new(reboot: bool) -> RebootQueryParam {
        RebootQueryParam {
            reboot,
        }
    }
}

