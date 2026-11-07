//! HTTP/2 types and request state, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/http2.{h,cpp}`.

use std::collections::HashMap;

/// `OrbisHttp2HttpVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisHttp2HttpVersion {
    Version1_0 = 1,
    Version1_1 = 2,
    Version2_0 = 3,
}

/// `OrbisHttp2PreSendCallbackData`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisHttp2PreSendCallbackData {
    pub unk: *mut core::ffi::c_void,
    pub cert: *mut core::ffi::c_void,
}

/// `OrbisHttp2PreSendCallback`.
pub type OrbisHttp2PreSendCallback = unsafe extern "C" fn(
    request_id: i32,
    ssl_id: i32,
    data: *mut OrbisHttp2PreSendCallbackData,
    user_arg: *mut core::ffi::c_void,
) -> i32;

#[derive(Debug, Clone, Default)]
pub struct Http2Template {
    pub user_agent: String,
    pub http_version: u32,
    pub auto_proxy: i32,
    pub pre_send_callback: Option<OrbisHttp2PreSendCallback>,
    pub pre_send_arg: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Http2Request {
    pub template_id: i32,
    pub method: String,
    pub url: String,
    pub content_length: u64,
    pub status_code: i32,
    pub headers: Vec<(String, String)>,
    pub sent: bool,
}

/// `Http2State`; request-to-connection mapping in the cpp.
#[derive(Debug, Default)]
pub struct Http2State {
    pub initialized: bool,
    pub next_template: i32,
    pub next_request: i32,
    pub templates: HashMap<i32, Http2Template>,
    pub requests: HashMap<i32, Http2Request>,
}

impl Http2State {
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        self.next_template = 1;
        self.next_request = 1;
        0
    }

    pub fn term(&mut self) -> i32 {
        self.initialized = false;
        self.templates.clear();
        self.requests.clear();
        0
    }

    pub fn create_template(&mut self, user_agent: &str, http_ver: i32, auto_proxy: i32) -> i32 {
        let id = self.next_template;
        self.next_template += 1;
        self.templates.insert(
            id,
            Http2Template {
                user_agent: user_agent.to_string(),
                http_version: http_ver as u32,
                auto_proxy,
                ..Http2Template::default()
            },
        );
        id
    }

    pub fn delete_template(&mut self, id: i32) -> i32 {
        if self.templates.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn create_request_with_url(
        &mut self,
        template_id: i32,
        method: &str,
        url: &str,
        content_length: u64,
    ) -> Option<i32> {
        if !self.templates.contains_key(&template_id) {
            return None;
        }
        let id = self.next_request;
        self.next_request += 1;
        self.requests.insert(
            id,
            Http2Request {
                template_id,
                method: method.to_string(),
                url: url.to_string(),
                content_length,
                ..Http2Request::default()
            },
        );
        Some(id)
    }

    pub fn send_request(&mut self, id: i32, data: &[u8]) -> i32 {
        let Some(request) = self.requests.get_mut(&id) else {
            return -1;
        };
        if data.len() as u64 > request.content_length && request.content_length != 0 {
            return -1;
        }
        request.sent = true;
        0
    }

    pub fn delete_request(&mut self, id: i32) -> i32 {
        if self.requests.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn default_version() -> OrbisHttp2HttpVersion {
        OrbisHttp2HttpVersion::Version1_1
    }

    pub fn set_pre_send_callback(
        &mut self,
        template_id: i32,
        callback: Option<OrbisHttp2PreSendCallback>,
        user_arg: usize,
    ) -> i32 {
        match self.templates.get_mut(&template_id) {
            Some(template) => {
                template.pre_send_callback = callback;
                template.pre_send_arg = user_arg;
                0
            }
            None => -1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http2_types_and_request_state_follow_the_cpp() {
        assert_eq!(OrbisHttp2HttpVersion::Version1_1 as u32, 2);
        assert_eq!(core::mem::size_of::<OrbisHttp2PreSendCallbackData>(), 16);
        assert_eq!(Http2State::default_version() as u32, 2);
        let mut state = Http2State::default();
        assert_eq!(state.init(), 0);
        let template = state.create_template("agent", 2, 0);
        let request = state
            .create_request_with_url(template, "GET", "https://x", 4)
            .unwrap();
        assert_eq!(state.send_request(request, b"data"), 0);
        assert_eq!(state.send_request(request, b"toolong"), -1);
        assert_eq!(state.delete_request(request), 0);
        assert_eq!(state.delete_template(template), 0);
    }
}
