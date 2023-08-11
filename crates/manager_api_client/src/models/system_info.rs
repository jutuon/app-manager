/*
 * app-manager
 *
 * App manager API
 *
 * The version of the OpenAPI document: 0.1.0
 * 
 * Generated by: https://openapi-generator.tech
 */




#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct SystemInfo {
    #[serde(rename = "info")]
    pub info: Vec<crate::models::CommandOutput>,
    #[serde(rename = "name")]
    pub name: String,
}

impl SystemInfo {
    pub fn new(info: Vec<crate::models::CommandOutput>, name: String) -> SystemInfo {
        SystemInfo {
            info,
            name,
        }
    }
}

