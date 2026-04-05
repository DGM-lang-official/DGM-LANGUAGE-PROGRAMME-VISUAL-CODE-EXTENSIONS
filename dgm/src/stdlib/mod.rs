pub mod math;
pub mod io_mod;
pub mod os_mod;
pub mod json_mod;
pub mod time_mod;
pub mod http_mod;
pub mod crypto_mod;
pub mod regex_mod;
pub mod net_mod;
pub mod thread_mod;
pub mod fs_mod;
pub mod xml_mod;
pub mod security;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::DgmValue;

pub fn load_module(name: &str) -> Option<DgmValue> {
    let map = match name {
        "math" => math::module(),
        "io" => io_mod::module(),
        "os" => os_mod::module(),
        "json" => json_mod::module(),
        "time" => time_mod::module(),
        "http" => http_mod::module(),
        "crypto" => crypto_mod::module(),
        "regex" => regex_mod::module(),
        "net" => net_mod::module(),
        "thread" => thread_mod::module(),
        "fs" => fs_mod::module(),
        "xml" => xml_mod::module(),
        "security" => security::module(),
        _ => return None,
    };
    Some(DgmValue::Map(Rc::new(RefCell::new(map))))
}
