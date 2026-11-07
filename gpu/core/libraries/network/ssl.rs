//! SSL certificate-list types, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/ssl.h`.

/// `OrbisSslCaCerts`; opaque in the header.
#[repr(C)]
pub struct OrbisSslCaCerts {
    pub _opaque: [u8; 0],
}

/// `OrbisSslCaList`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisSslCaList {
    pub certs: *mut core::ffi::c_void,
    pub certs_num: i32,
}

/// The SSL entry points; the HLE loader implements them.
pub trait SslApi {
    fn init(&mut self, pool_size: usize) -> i32;
    fn term(&mut self) -> i32;
    fn create_ssl_connection(&mut self) -> i32;
    fn delete_ssl_connection(&mut self, id: i32) -> i32;
    fn connect(&mut self, id: i32) -> i32;
    fn close(&mut self, id: i32) -> i32;
    fn get_ca_certs(&mut self, ssl_ctx_id: i32, certs: &mut OrbisSslCaCerts) -> i32;
    fn get_ca_list(&mut self) -> i32;
    fn get_ssl_error(&mut self, id: i32) -> i32;
    fn get_serial_number(&mut self, id: i32, buffer: &mut [u8], size: &mut u64) -> i32;
    fn send(&mut self, id: i32, data: &[u8]) -> i32;
    fn recv(&mut self, id: i32, data: &mut [u8]) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ca_list_layout_matches_the_header() {
        assert_eq!(core::mem::size_of::<OrbisSslCaList>(), 16);
        assert_eq!(core::mem::offset_of!(OrbisSslCaList, certs_num), 8);
        assert_eq!(core::mem::size_of::<OrbisSslCaCerts>(), 0);
    }
}

/// The `ssl.cpp` runtime state.
#[derive(Debug, Default)]
pub struct SslState {
    pub initialized: bool,
    pub next_connection: i32,
    pub connections: std::collections::HashMap<i32, SslConnection>,
    pub certs: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SslConnection {
    pub ctx_id: i32,
    pub connected: bool,
    pub pending: bool,
}

impl SslState {
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        self.next_connection = 1;
        0
    }

    pub fn term(&mut self) -> i32 {
        self.initialized = false;
        self.connections.clear();
        self.certs.clear();
        0
    }

    pub fn create_connection(&mut self, ctx_id: i32) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_connection;
        self.next_connection += 1;
        self.connections.insert(
            id,
            SslConnection {
                ctx_id,
                ..SslConnection::default()
            },
        );
        Some(id)
    }

    pub fn connect(&mut self, id: i32) -> i32 {
        match self.connections.get_mut(&id) {
            Some(connection) => {
                connection.connected = true;
                0
            }
            None => -1,
        }
    }

    pub fn close(&mut self, id: i32) -> i32 {
        if self.connections.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn load_cert(&mut self, cert: Vec<u8>) -> usize {
        self.certs.push(cert);
        self.certs.len() - 1
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn ssl_connections_and_certificates_are_tracked() {
        let mut ssl = SslState::default();
        assert!(ssl.create_connection(1).is_none());
        assert_eq!(ssl.init(), 0);
        let id = ssl.create_connection(1).unwrap();
        assert_eq!(ssl.connect(id), 0);
        assert!(ssl.connections[&id].connected);
        assert_eq!(ssl.load_cert(vec![1]), 0);
        assert_eq!(ssl.close(id), 0);
        assert_eq!(ssl.close(id), -1);
    }
}
