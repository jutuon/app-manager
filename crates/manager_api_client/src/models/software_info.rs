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
pub struct SoftwareInfo {
    #[serde(rename = "current_software")]
    pub current_software: Vec<crate::models::BuildInfo>,
}

impl SoftwareInfo {
    pub fn new(current_software: Vec<crate::models::BuildInfo>) -> SoftwareInfo {
        SoftwareInfo {
            current_software,
        }
    }
}


