use super::Id;
use bencode_minimal::{Value, dict, int, list, str};
use super::Version;

pub struct Msg;

impl Msg {
    pub const Y: &str = "y";
    pub const Q: &str = "q";
    pub const R: &str = "r";
    pub const E: &str = "e";
    pub const T: &str = "t";
    pub const A: &str = "a";
    pub const V: &str = "v";
    pub const ID: &str = "id";
    pub const PING: &str = "ping";
    pub const TOKEN: &str = "token";
    pub const TARGET: &str = "target";
    pub const INFO_HASH: &str = "info_hash";
    pub const FIND_NODE: &str = "find_node";
    pub const GET_PEERS: &str = "get_peers";
    pub const ANNOUNCE_PEER: &str = "announce_peer";
    pub const NODES6: &str = "nodes6";

    pub fn error_204<'a>(t: &'a [u8]) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::E),
            Msg::E => list![
                int!(204),
                str!("Method Unknown"),
            ],
        }
    }

    pub fn ping_query<'a>(t: &'a [u8], id: &'a Id) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::Q),
            Msg::Q => str!(Msg::PING),
            Msg::A => dict! {
                Msg::ID => str!(id),
            }
        }
    }

    pub fn ping_response<'a>(t: &'a [u8], id: &'a Id) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::R),
            Msg::R => dict! {
                Msg::ID => str!(id),
            }
        }
    }

    pub fn find_node_query<'a>(t: &'a [u8], id: &'a Id, target: &'a Id) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::Q),
            Msg::Q => str!(Msg::FIND_NODE),
            Msg::A => dict! {
                Msg::ID => str!(id),
                Msg::TARGET => str!(target),
            }
        }
    }

    pub fn find_node_response<'a>(t: &'a [u8], id: &'a Id, nodes6: &'a [u8]) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::R),
            Msg::R => dict! {
                Msg::ID => str!(id),
                Msg::NODES6 => str!(nodes6),
            }
        }
    }

    pub fn get_peers_response<'a>(t: &'a [u8], id: &'a Id, token: &'a [u8], nodes6: &'a [u8]) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::R),
            Msg::R => dict! {
                Msg::ID => str!(id),
                Msg::TOKEN => str!(token),
                Msg::NODES6 => str!(nodes6),
            }
        }
    }

    pub fn announce_peer_response<'a>(t: &'a [u8], id: &'a Id) -> Value<'a> {
        dict! {
            Msg::T => str!(t),
            Msg::V => str!(Version::SELF.as_ref()),
            Msg::Y => str!(Msg::R),
            Msg::R => dict! {
                Msg::ID => str!(id),
            }
        }
    }
}
