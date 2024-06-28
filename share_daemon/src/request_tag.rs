pub mod local {
    pub const SHARE: &str = "SHARE";
    pub const REG: &str = "REG";
    pub const INTERACTIVE: &str = "INTERACTIVE";
}


pub mod client {
    pub const REG_REMOTE: &str = "REG_REMOTE";
    pub const FILES_RECV: &str = "FILES_RECV";
}
pub mod remote {
    pub const PORT: &str = "PORT";
    // pub const REG_ME: &str = "REG_ME";
}


pub mod send_flag{
    pub const SEND_START: &str = "SEND_START";
    pub const SEND_END: &str = "SEND_END";
}
