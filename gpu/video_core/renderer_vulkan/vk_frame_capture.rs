//! Frame analyzer, ported from
//! `shadps4/video_core/renderer_vulkan/vk_frame_capture.{h,cpp}`.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub const IDLE: u32 = 0;
pub const ARMED: u32 = 1;
pub const RECORDING: u32 = 2;

static STATE: AtomicU32 = AtomicU32::new(IDLE);
static FLIPS: AtomicU64 = AtomicU64::new(0);
static LAST_PRESENTED: AtomicU64 = AtomicU64::new(0);

/// A render target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Target {
    pub address: u64,
    pub format: i32,
    pub width: u32,
    pub height: u32,
}

impl Target {
    pub fn describe(&self) -> String {
        if self.address == 0 {
            "-".to_string()
        } else {
            format!(
                "{:#x} fmt{} {}x{}",
                self.address, self.format, self.width, self.height
            )
        }
    }
}

/// One pass or dispatch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub compute: bool,
    pub colors: Vec<Target>,
    pub depth: Target,
    pub draws: u32,
    pub indices: u64,
    pub shaders: Vec<u64>,
    pub sampled: Vec<(Target, bool)>,
    pub note: String,
    pub buffers: Vec<String>,
}

/// GPU-thread capture state.
#[derive(Debug, Default)]
pub struct CaptureState {
    pub entries: Vec<Entry>,
    pub pending_sampled: Vec<(Target, bool)>,
    pub pending_buffers: Vec<String>,
    pub pass_open: bool,
}

fn capture() -> &'static Mutex<CaptureState> {
    static CAPTURE: OnceLock<Mutex<CaptureState>> = OnceLock::new();
    CAPTURE.get_or_init(|| Mutex::new(CaptureState::default()))
}

fn display_buffers() -> &'static Mutex<Vec<u64>> {
    static DISPLAY: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();
    DISPLAY.get_or_init(|| Mutex::new(Vec::new()))
}

/// `FrameCapture`.
pub struct FrameCapture;

impl FrameCapture {
    /// `OnFlip`; arms the capture when the trigger file exists.
    pub fn on_flip(presented_address: u64) {
        LAST_PRESENTED.store(presented_address, Ordering::Relaxed);
        FLIPS.fetch_add(1, Ordering::Release);
        if let Ok(trigger) = std::env::var("BB_CAPTURE_TRIGGER") {
            if STATE.load(Ordering::Relaxed) == IDLE && std::path::Path::new(&trigger).exists() {
                let _ = std::fs::remove_file(&trigger);
                STATE.store(ARMED, Ordering::Release);
            }
        }
    }

    pub fn add_display_buffer(address: u64) {
        let mut buffers = display_buffers()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !buffers.contains(&address) {
            buffers.push(address);
        }
    }

    pub fn is_display_buffer(address: u64) -> bool {
        display_buffers()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains(&address)
    }

    pub fn active() -> bool {
        STATE.load(Ordering::Relaxed) != IDLE
    }

    /// `Poll`; a recording ends at the next frame boundary.
    pub fn poll() {
        if STATE.load(Ordering::Acquire) == RECORDING {
            STATE.store(IDLE, Ordering::Release);
        }
    }

    /// `BeginPass`.
    pub fn begin_pass(colors: &[Target], depth: Option<Target>) {
        if STATE.load(Ordering::Acquire) == IDLE {
            return;
        }
        STATE.store(RECORDING, Ordering::Release);
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        capture.entries.push(Entry {
            colors: colors.to_vec(),
            depth: depth.unwrap_or_default(),
            ..Entry::default()
        });
        capture.pass_open = true;
    }

    /// `Draw`; appends to the current pass.
    pub fn draw(vs_hash: u64, ps_hash: u64, num_indices: u32, _num_instances: u32) {
        if STATE.load(Ordering::Acquire) == IDLE {
            return;
        }
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        let pending_buffers = std::mem::take(&mut capture.pending_buffers);
        let pending_sampled = std::mem::take(&mut capture.pending_sampled);
        let Some(entry) = capture.entries.last_mut() else {
            return;
        };
        entry.draws += 1;
        entry.indices += u64::from(num_indices);
        if entry.shaders.len() < 8 && !entry.shaders.contains(&vs_hash) {
            entry.shaders.push(vs_hash);
        }
        if entry.shaders.len() < 8 && !entry.shaders.contains(&ps_hash) {
            entry.shaders.push(ps_hash);
        }
        if entry.draws <= 4 {
            entry.buffers.extend(pending_buffers);
        }
        for sampled in pending_sampled {
            if entry.sampled.len() < 32 && !entry.sampled.contains(&sampled) {
                entry.sampled.push(sampled);
            }
        }
    }

    /// `Dispatch`.
    pub fn dispatch(cs_hash: u64, _x: u32, _y: u32, _z: u32) {
        if STATE.load(Ordering::Acquire) == IDLE {
            return;
        }
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        capture.entries.push(Entry {
            compute: true,
            draws: 1,
            shaders: vec![cs_hash],
            ..Entry::default()
        });
    }

    pub fn sampled(target: Target, storage: bool) {
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        if capture.pending_sampled.len() < 32 {
            capture.pending_sampled.push((target, storage));
        }
    }

    pub fn buffer(_stage_hash: u64, _slot: u32, address: u64, data: &[u8]) {
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        let preview: Vec<String> = data
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        capture.pending_buffers.push(format!(
            "  buffer {:#x} {} bytes {}",
            address,
            data.len(),
            preview.join(" ")
        ));
    }

    pub fn note(text: &str) {
        let mut capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        if let Some(entry) = capture.entries.last_mut() {
            entry.note = text.to_string();
        }
    }

    pub fn flips() -> u64 {
        FLIPS.load(Ordering::Acquire)
    }

    pub fn last_presented() -> u64 {
        LAST_PRESENTED.load(Ordering::Acquire)
    }

    pub fn set_state(state: u32) {
        STATE.store(state, Ordering::Release);
    }

    /// `Write`; returns the text and optionally writes `frame_<time>.txt` under `dir`.
    pub fn write(dir: Option<&std::path::Path>, presented: u64) -> String {
        let capture = capture().lock().unwrap_or_else(|error| error.into_inner());
        let mut text = format!("presented buffer {presented:#x}\n");
        for (index, entry) in capture.entries.iter().enumerate() {
            if entry.compute {
                text.push_str(&format!(
                    "\n#{index} COMPUTE dispatches {}, cs",
                    entry.draws
                ));
            } else {
                text.push_str(&format!(
                    "\n#{index} PASS draws {} (indices {}), vs/ps",
                    entry.draws, entry.indices
                ));
            }
            for shader in &entry.shaders {
                text.push_str(&format!(" {shader:016x}"));
            }
            text.push('\n');
            for (index, color) in entry.colors.iter().enumerate() {
                text.push_str(&format!("  color{index} {}\n", color.describe()));
            }
            if !entry.compute {
                text.push_str(&format!("  depth  {}\n", entry.depth.describe()));
            }
            for (target, storage) in &entry.sampled {
                text.push_str(&format!(
                    "  {} {}\n",
                    if *storage { "writes " } else { "samples" },
                    target.describe()
                ));
            }
            if !entry.note.is_empty() {
                text.push_str(&format!("  note: {}\n", entry.note));
            }
            for buffer in &entry.buffers {
                text.push_str(buffer);
                text.push('\n');
            }
        }
        if let Some(dir) = dir {
            let path = dir.join(format!(
                "frame_{}.txt",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |time| time.as_secs())
            ));
            let _ = std::fs::write(path, &text);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_arm_and_finish_a_capture() {
        FrameCapture::set_state(ARMED);
        assert!(FrameCapture::active());
        FrameCapture::on_flip(0x1000);
        assert_eq!(FrameCapture::last_presented(), 0x1000);
        assert!(FrameCapture::flips() >= 1);
        FrameCapture::begin_pass(
            &[Target {
                address: 0x2000,
                format: 37,
                width: 1920,
                height: 1080,
            }],
            None,
        );
        FrameCapture::draw(1, 2, 300, 1);
        FrameCapture::note("scene");
        let text = FrameCapture::write(None, 0x1000);
        assert!(text.contains("PASS draws 1 (indices 300)"));
        assert!(text.contains("color0 0x2000"));
        assert!(text.contains("note: scene"));
        FrameCapture::poll();
        assert!(!FrameCapture::active());
        FrameCapture::add_display_buffer(0x2000);
        assert!(FrameCapture::is_display_buffer(0x2000));
        assert!(!FrameCapture::is_display_buffer(0x3000));
    }
}
