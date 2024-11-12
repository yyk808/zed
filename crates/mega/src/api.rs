use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MountRequest<'a> {
    pub path: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MountResponse {
    pub status: String,
    pub mount: MountInfo,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MountInfo {
    pub hash: String,
    pub path: String,
    pub inode: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MountsResponse {
    pub status: String,
    pub mounts: Vec<MountInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UmountRequest<'a> {
    pub path: Option<&'a str>,
    pub inode: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UmountResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigResponse {
    pub status: String,
    pub config: ConfigInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigInfo {
    pub mega_url: String,
    pub mount_path: String,
    pub store_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigRequest {
    pub mega_url: Option<String>,
    pub mount_path: Option<String>,
    pub store_path: Option<String>,
}