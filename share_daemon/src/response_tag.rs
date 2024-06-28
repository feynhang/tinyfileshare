pub mod common {
    pub const REG_SUCCEEDED: &str = "REG_SUCCEEDED";
    pub const UNEXPECTED_RESP: &str = "UNEXPECTED_RESPONSE";
    pub const INVALID_HOSTNAME: &str = "INVALID_HOSTNAME";
}

pub mod remote {
    pub const UNREGISTERED_HOST: &str = "UNREGISTERED_HOST";
    pub const NO_AVAILABLE_PORT: &str = "NO_AVAILABLE_PORT";
    pub const PORT_CONFIRM: &str = "PORT_CONFIRM";
    pub const INVALID_PORT: &str = "INVALID_PORT";
    pub const FILES_RECEIVED: &str = "FILES_RECEIVED";
    pub const UNEXPECTED_END_FLAG: &str = "UNEXPECTED_END_FLAG";
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";


}


pub mod local {
    pub const REMOTE_UNREGISTERED: &str = "REMOTE_UNREGISTERED";
    pub const REMOTE_NO_AVAILABLE_PORT: &str = "REMOTE_NO_AVAILABLE_PORT";
    pub const UNREACHABLE_ADDRESS: &str = "UNREACHABLE_ADDRESS";
    pub const ALL_FILES_SENT_SUCCEEDED: &str = "ALL_FILES_SENT_SUCCEEDED";  
    pub const PROGRESS: &str = "PROGRESS";
    pub const UNEXPECTED_SEND_RESPONSE: &str = "UNEXPECTED_SEND_RESP";
    pub const FILES_SENT_SUCCEEDED: &str = "FILES_SENT_SUCCEEDED";
    pub const REG_LOCAL_FAILED: &str = "REG_LOCAL_FAILED";
    pub const REPLACED_ADDR: &str = "REPLACED_ADDR";
    pub const UNREGISTERED_HOSTNAME: &str = "UNREGISTERED_HOSTNAME";
    pub const ANY_PATH_INVALID: &str = "INVALID_PATHS";
    pub const UNEXPECTED_REMOTE_RESP_TAG: &str = "UNEXPECTED_REMOTE_RESPONSE_TAG";

}
