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
pub struct BuildInfo {
    /// Build info output from the built binary.  Binary must support --build-info command line argument.
    #[serde(rename = "build_info")]
    pub build_info: String,
    #[serde(rename = "commit_sha")]
    pub commit_sha: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "timestamp")]
    pub timestamp: String,
}

impl BuildInfo {
    pub fn new(build_info: String, commit_sha: String, name: String, timestamp: String) -> BuildInfo {
        BuildInfo {
            build_info,
            commit_sha,
            name,
            timestamp,
        }
    }
}


