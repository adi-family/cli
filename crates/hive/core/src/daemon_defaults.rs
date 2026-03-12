//! Centralized default values for the hive daemon.

pub const DNS_BIND: &str = "127.0.0.1:15353";
pub const DNS_UPSTREAM: &str = "8.8.8.8:53";
pub const DNS_TTL: u32 = 60;
pub const LOG_BUFFER_CAPACITY: usize = 10000;
pub const LOG_LINES_LIMIT: usize = 100;
pub const PID_NAME: &str = "adi-hive.pid";
pub const SOCKET_NAME: &str = "adi-hive.sock";

