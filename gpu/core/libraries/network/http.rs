//! HTTP library types, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/http.h`.

use crate::gpu::core::libraries::network::ssl::OrbisSslCaList;

pub const ORBIS_HTTP_NB_EVENT_IN: u32 = 0x0000_0001;
pub const ORBIS_HTTP_NB_EVENT_OUT: u32 = 0x0000_0002;
pub const ORBIS_HTTP_NB_EVENT_SOCK_ERR: u32 = 0x0000_0008;
pub const ORBIS_HTTP_NB_EVENT_HUP: u32 = 0x0000_0010;
pub const ORBIS_HTTP_NB_EVENT_RESOLVED: u32 = 0x0001_0000;
pub const ORBIS_HTTP_NB_EVENT_RESOLVER_ERR: u32 = 0x0002_0000;

pub const ORBIS_HTTPS_FLAG_SERVER_VERIFY: u32 = 0x01;
pub const ORBIS_HTTPS_FLAG_CLIENT_VERIFY: u32 = 0x02;
pub const ORBIS_HTTPS_FLAG_CN_CHECK: u32 = 0x04;
pub const ORBIS_HTTPS_FLAG_NOT_AFTER_CHECK: u32 = 0x08;
pub const ORBIS_HTTPS_FLAG_NOT_BEFORE_CHECK: u32 = 0x10;
pub const ORBIS_HTTPS_FLAG_KNOWN_CA_CHECK: u32 = 0x20;
pub const ORBIS_HTTPS_FLAG_SESSION_REUSE: u32 = 0x40;
pub const ORBIS_HTTPS_FLAG_SNI: u32 = 0x80;

pub const ORBIS_HTTPS_FLAG_SDK_DEFAULT: u32 = ORBIS_HTTPS_FLAG_SERVER_VERIFY
    | ORBIS_HTTPS_FLAG_CN_CHECK
    | ORBIS_HTTPS_FLAG_KNOWN_CA_CHECK
    | ORBIS_HTTPS_FLAG_SNI;
pub const ORBIS_HTTPS_FLAG_PUBLIC_VALID: u32 = 0x0000_20ff;
pub const ORBIS_HTTPS_FLAG_PRIVATE_VALID: u32 = 0x0000_2dff;

/// `OrbisHttpMethod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisHttpMethod {
    Get = 0,
    Post = 1,
    Head = 2,
    Options = 3,
    Put = 4,
    Delete = 5,
    Trace = 6,
    Connect = 7,
    Custom = 8,
}

/// `OrbisHttpHeaderModes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisHttpHeaderModes {
    Overwrite = 0,
    Add = 1,
}

/// `OrbisUriBuild`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisUriBuild {
    Scheme = 0x01,
    Hostname = 0x02,
    Port = 0x04,
    Path = 0x08,
    Username = 0x10,
    Password = 0x20,
    Query = 0x40,
    Fragment = 0x80,
}

/// `OrbisHttpUriElement`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisHttpUriElement {
    pub opaque: bool,
    pub scheme: *mut u8,
    pub username: *mut u8,
    pub password: *mut u8,
    pub hostname: *mut u8,
    pub path: *mut u8,
    pub query: *mut u8,
    pub fragment: *mut u8,
    pub port: u16,
    pub reserved: [u8; 10],
}

/// `HttpRequestInternal`.
#[repr(C)]
#[derive(Debug)]
pub struct HttpRequestInternal {
    pub state: i32,
    pub error_code: i32,
    pub http_status_code: i32,
    pub mutex: std::sync::Mutex<()>,
}

/// `OrbisHttpMemoryPoolStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisHttpMemoryPoolStats {
    pub pool_size: u64,
    pub max_inuse_size: u64,
    pub current_inuse_size: u64,
    pub reserved: i32,
}

/// `OrbisHttpCookieStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisHttpCookieStats {
    pub current_inuse_size: u64,
    pub current_inuse_num: u32,
    pub max_inuse_size: u64,
    pub max_inuse_num: u32,
    pub removed_num: u32,
    pub reserved: i32,
}

/// `OrbisHttpEpollHandle`.
pub type OrbisHttpEpollHandle = *mut core::ffi::c_void;

/// `OrbisHttpNBEvent`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisHttpNBEvent {
    pub events: u32,
    pub event_detail: u32,
    pub id: i32,
    pub user_arg: *mut core::ffi::c_void,
}

pub type OrbisHttpAuthInfoCallback = unsafe extern "C" fn(
    request: i32,
    auth_type: i32,
    realm: *const u8,
    username: *mut u8,
    password: *mut u8,
    is_need_entity: i32,
    entity_body: *mut *mut u8,
    entity_size: *mut u64,
    is_save: *mut i32,
    user_arg: *mut core::ffi::c_void,
) -> i32;

pub type OrbisHttpRedirectCallback = unsafe extern "C" fn(
    request: i32,
    status_code: i32,
    method: *mut i32,
    location: *const u8,
    user_arg: *mut core::ffi::c_void,
) -> i32;

pub type OrbisHttpRequestStatusCallback =
    unsafe extern "C" fn(request: i32, request_status: i32, user_arg: *mut core::ffi::c_void);

pub type OrbisHttpCookieRecvCallback = unsafe extern "C" fn(
    request: i32,
    url: *const u8,
    cookie_header: *const u8,
    header_len: u64,
    user_arg: *mut core::ffi::c_void,
) -> i32;

pub type OrbisHttpCookieSendCallback = unsafe extern "C" fn(
    request: i32,
    url: *const u8,
    cookie_header: *const u8,
    user_arg: *mut core::ffi::c_void,
) -> i32;

pub type OrbisHttpsCallback = unsafe extern "C" fn(
    libssl_ctx_id: i32,
    verify_err: u32,
    ssl_cert: *mut *mut core::ffi::c_void,
    cert_num: i32,
    user_arg: *mut core::ffi::c_void,
) -> i32;

/// `OrbisHttpsCaList`.
pub type OrbisHttpsCaList = OrbisSslCaList;

/// The HTTP entry points; the HLE loader implements them.
pub trait HttpApi {
    fn init(&mut self) -> i32;
    fn term(&mut self) -> i32;
    fn create_template(
        &mut self,
        ctx_id: i32,
        user_agent: &str,
        http_ver: i32,
        auto_proxy: i32,
    ) -> i32;
    fn delete_template(&mut self, template_id: i32) -> i32;
    fn create_connection(
        &mut self,
        template_id: i32,
        server_name: &str,
        scheme: &str,
        port: u16,
        keep_alive: bool,
    ) -> i32;
    fn delete_connection(&mut self, connection_id: i32) -> i32;
    fn create_request(
        &mut self,
        connection_id: i32,
        method: i32,
        path: &str,
        content_length: u64,
    ) -> i32;
    fn delete_request(&mut self, request_id: i32) -> i32;
    fn send_request(&mut self, request_id: i32, data: &[u8]) -> i32;
    fn get_status_code(&mut self, request_id: i32, status_code: &mut i32) -> i32;
    fn read_data(&mut self, request_id: i32, data: &mut [u8]) -> i32;
    fn abort_request(&mut self, request_id: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_http_types_match_the_header() {
        assert_eq!(ORBIS_HTTP_NB_EVENT_RESOLVED, 0x10000);
        assert_eq!(ORBIS_HTTPS_FLAG_SDK_DEFAULT, 0xa5);
        assert_eq!(OrbisHttpMethod::Custom as i32, 8);
        assert_eq!(OrbisHttpHeaderModes::Add as i32, 1);
        assert_eq!(OrbisUriBuild::Fragment as i32, 0x80);
        assert_eq!(core::mem::size_of::<OrbisHttpMemoryPoolStats>(), 32);
        assert_eq!(core::mem::size_of::<OrbisHttpCookieStats>(), 40);
        assert_eq!(core::mem::size_of::<OrbisHttpNBEvent>(), 24);
        assert_eq!(core::mem::size_of::<HttpRequestInternal>(), 20);
    }
}

/// `HttpState`: templates, connections and requests (`http.cpp`).
#[derive(Debug, Clone, Default)]
pub struct HttpTemplate {
    pub user_agent: String,
    pub http_version: i32,
    pub auto_proxy: i32,
    pub ssl_flags: u32,
}

#[derive(Debug, Clone, Default)]
pub struct HttpConnection {
    pub template_id: i32,
    pub server_name: String,
    pub scheme: String,
    pub port: u16,
    pub keep_alive: bool,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum HttpRequestState {
    #[default]
    Created = 0,
    Sending = 1,
    Sent = 2,
    Reading = 3,
    Done = 4,
    Aborted = 5,
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub connection_id: i32,
    pub method: i32,
    pub path: String,
    pub url: String,
    pub content_length: u64,
    pub post_data: Vec<u8>,
    pub state: HttpRequestState,
    pub status_code: i32,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Vec<u8>,
    pub read_offset: usize,
    pub last_errno: i32,
}

#[derive(Debug, Default)]
pub struct HttpState {
    pub next_template: i32,
    pub next_connection: i32,
    pub next_request: i32,
    pub templates: std::collections::HashMap<i32, HttpTemplate>,
    pub connections: std::collections::HashMap<i32, HttpConnection>,
    pub requests: std::collections::HashMap<i32, HttpRequest>,
}

impl HttpState {
    pub fn create_template(&mut self, user_agent: &str, http_ver: i32, auto_proxy: i32) -> i32 {
        self.next_template += 1;
        self.templates.insert(
            self.next_template,
            HttpTemplate {
                user_agent: user_agent.to_string(),
                http_version: http_ver,
                auto_proxy,
                ssl_flags: ORBIS_HTTPS_FLAG_SDK_DEFAULT,
            },
        );
        self.next_template
    }

    pub fn delete_template(&mut self, id: i32) -> i32 {
        if self.templates.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn create_connection(
        &mut self,
        template_id: i32,
        server_name: &str,
        scheme: &str,
        port: u16,
        keep_alive: bool,
    ) -> i32 {
        if !self.templates.contains_key(&template_id) {
            return -1;
        }
        self.next_connection += 1;
        self.connections.insert(
            self.next_connection,
            HttpConnection {
                template_id,
                server_name: server_name.to_string(),
                scheme: scheme.to_string(),
                port,
                keep_alive,
                ..HttpConnection::default()
            },
        );
        self.next_connection
    }

    pub fn delete_connection(&mut self, id: i32) -> i32 {
        if self.connections.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn create_request(
        &mut self,
        connection_id: i32,
        method: i32,
        path: &str,
        content_length: u64,
    ) -> i32 {
        if !self.connections.contains_key(&connection_id) {
            return -1;
        }
        self.next_request += 1;
        self.requests.insert(
            self.next_request,
            HttpRequest {
                connection_id,
                method,
                path: path.to_string(),
                content_length,
                ..HttpRequest::default()
            },
        );
        self.next_request
    }

    pub fn delete_request(&mut self, id: i32) -> i32 {
        if self.requests.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn send_request(&mut self, id: i32, data: &[u8]) -> i32 {
        let Some(request) = self.requests.get_mut(&id) else {
            return -1;
        };
        if request.content_length != 0 && data.len() as u64 > request.content_length {
            return -1;
        }
        request.post_data = data.to_vec();
        request.state = HttpRequestState::Sent;
        0
    }

    pub fn read_data(&mut self, id: i32, size: u64) -> Option<Vec<u8>> {
        let request = self.requests.get_mut(&id)?;
        if request.read_offset >= request.response_body.len() {
            return Some(Vec::new());
        }
        let end = (request.read_offset + size as usize).min(request.response_body.len());
        let data = request.response_body[request.read_offset..end].to_vec();
        request.read_offset = end;
        Some(data)
    }

    pub fn get_status_code(&self, id: i32) -> Option<i32> {
        self.requests.get(&id).map(|request| request.status_code)
    }

    pub fn all_response_headers(&self, id: i32) -> Option<String> {
        let request = self.requests.get(&id)?;
        Some(
            request
                .response_headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}\r\n"))
                .collect(),
        )
    }

    pub fn abort_request(&mut self, id: i32) -> i32 {
        match self.requests.get_mut(&id) {
            Some(request) => {
                request.state = HttpRequestState::Aborted;
                0
            }
            None => -1,
        }
    }

    pub fn request_count(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn http_state_creates_sends_reads_and_aborts() {
        let mut state = HttpState::default();
        assert_eq!(state.create_connection(1, "host", "http", 80, true), -1);
        let template = state.create_template("agent", 1, 0);
        let connection = state.create_connection(template, "host", "http", 80, true);
        let request = state.create_request(connection, OrbisHttpMethod::Post as i32, "/x", 4);
        assert_eq!(state.send_request(request, b"data"), 0);
        assert_eq!(state.requests[&request].state, HttpRequestState::Sent);
        state.requests.get_mut(&request).unwrap().response_body = b"body".to_vec();
        state.requests.get_mut(&request).unwrap().status_code = 200;
        assert_eq!(state.get_status_code(request), Some(200));
        assert_eq!(state.read_data(request, 2), Some(b"bo".to_vec()));
        assert_eq!(state.read_data(request, 8), Some(b"dy".to_vec()));
        assert!(state.all_response_headers(request).is_some());
        assert_eq!(state.abort_request(request), 0);
        assert_eq!(state.delete_request(request), 0);
    }
}
