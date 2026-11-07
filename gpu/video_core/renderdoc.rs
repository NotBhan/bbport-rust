//! Optional RenderDoc capture integration, ported from
//! `shadps4/video_core/renderdoc.{h,cpp}`.
//!
//! The external `renderdoc_app.h` is not vendored in the port tree; this module keeps the
//! integration surface so the renderer can call it when a RenderDoc API is supplied.

/// `RenderDocStatus`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderDocStatus {
    #[default]
    Unavailable,
    Available,
    Capturing,
}

/// `RenderDoc`.
#[derive(Debug, Default)]
pub struct RenderDoc {
    pub status: RenderDocStatus,
    pub capture_path: String,
    pub frame_count: u64,
    pub capture_requested: bool,
}

impl RenderDoc {
    pub fn is_available(&self) -> bool {
        self.status == RenderDocStatus::Available
    }

    pub fn start_frame_capture(&mut self, path: &str) -> bool {
        if self.status != RenderDocStatus::Available {
            return false;
        }
        self.capture_path = path.to_string();
        self.capture_requested = true;
        self.status = RenderDocStatus::Capturing;
        true
    }

    pub fn end_frame_capture(&mut self) -> bool {
        if self.status != RenderDocStatus::Capturing {
            return false;
        }
        self.status = RenderDocStatus::Available;
        self.capture_requested = false;
        self.frame_count += 1;
        true
    }

    pub fn end_frame(&mut self) {
        self.frame_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_lifecycle_requires_available_status() {
        let mut renderdoc = RenderDoc::default();
        assert!(!renderdoc.is_available());
        assert!(!renderdoc.start_frame_capture("out.rdc"));
        renderdoc.status = RenderDocStatus::Available;
        assert!(renderdoc.start_frame_capture("out.rdc"));
        assert_eq!(renderdoc.status, RenderDocStatus::Capturing);
        assert!(renderdoc.end_frame_capture());
        assert_eq!(renderdoc.frame_count, 1);
        renderdoc.end_frame();
        assert_eq!(renderdoc.frame_count, 2);
    }
}
