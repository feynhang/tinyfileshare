pub mod common {
    pub const REG_SUCCEEDED: &str = "REG_SUCCEEDED";
    pub const UNEXPECTED_RESP_TAG: &str = "UNEXPECTED_RESPONSE_TAG";
    pub const REG_FAILED: &str = "REG_FAILED";
    pub const REG_REMOTE_FAILED: &str = "REG_REMOTE_FAILED";
    pub const REG_REMOTE_UNSUPPORTED: &str = "REG_REMOTE_UNSUPPORTED";
    pub const REG_REMOTE_REJECTED: &str = "REG_REMOTE_REJECTED";
}

pub mod remote {
    pub const CLIENT_REJECTED: &str = "CLIENT_REJECTED";
    pub const UNREGISTERED_HOST: &str = "UNREGISTERED_HOST";
    pub const NO_AVAILABLE_PORT: &str = "NO_AVAILABLE_PORT";
    pub const PORT_CONFIRM: &str = "PORT_CONFIRM";
    pub const INVALID_PORT: &str = "INVALID_PORT";
    pub const FILES_RECEIVED: &str = "FILES_RECEIVED";
    pub const UNEXPECTED_END_FLAG: &str = "UNEXPECTED_END_FLAG";
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";


}


pub mod local {
    pub const REMOTE_CLIENT_REJECTED: &str = "SEND_REMOTE_REJECTED";
    pub const UNREGISTERED_REMOTE: &str = "UNREGISTERED_REMOTE";
    pub const NO_AVAILABLE_PORT_REMOTE: &str = "NO_AVAILABLE_PORT_REMOTE";
    pub const UNREACHABLE_ADDRESS: &str = "UNREACHABLE_ADDRESS";
    pub const ALL_FILES_SENT_SUCCEEDED: &str = "ALL_FILES_SENT_SUCCEEDED";  
    pub const PROGRESS: &str = "PROGRESS";
    pub const UNEXPECTED_SEND_RESPONSE: &str = "UNEXPECTED_SEND_RESP";
    pub const FILES_SENT_SUCCEEDED: &str = "FILES_SENT_SUCCEEDED";
    pub const REG_LOCAL_FAILED: &str = "REG_LOCAL_FAILED";
    pub const REPLACED_ADDR: &str = "REPLACED_ADDR";
    pub const UNREGISTERED_LOCAL: &str = "UNREGISTERED_LOCAL";
    pub const ANY_PATH_INVALID: &str = "INVALID_PATHS";
    pub const UNEXPECTED_REMOTE_RESP_TAG: &str = "UNEXPECTED_REMOTE_RESPONSE_TAG";

}

pub mod client {
    pub const INVALID_RECV_DIR: &str = "INVALID_RECV_DIR";
    pub const RECV_ACCEPTED: &str = "RECV_ACCEPTED";
    pub const RECV_REJECTED: &str = "RECV_REJECTED";
    pub const REG_REJECTED: &str = "REG_REJECTED";
}