//! Video decoder helpers, ported from
//! `legacy/gpu/shadps4/core/libraries/videodec/video_utils.h`. The C build copies from an
//! FFmpeg `AVFrame`; the Rust port takes the planes it would read.

/// The plane layout of a decoded NV12 frame (`AVFrame::data`/`linesize`).
#[derive(Debug, Clone, Copy)]
pub struct Nv12Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub y: &'a [u8],
    pub uv: &'a [u8],
    /// Padded source strides; the destination is tightly packed.
    pub y_linesize: usize,
    pub uv_linesize: usize,
}

/// Copies a row of `width` bytes from a padded source plane into a packed destination.
fn copy_rows(dst: &mut [u8], source: &[u8], linesize: usize, width: usize, rows: usize) -> usize {
    let mut written = 0;
    for row in 0..rows {
        let start = row * linesize;
        let end = start + width;
        if end > source.len() || written + width > dst.len() {
            break;
        }
        dst[written..written + width].copy_from_slice(&source[start..end]);
        written += width;
    }
    written
}

/// `Libraries::Videodec::CopyNV12Data`: the luma plane, then the interleaved chroma.
pub fn copy_nv12_data(dst: &mut [u8], max_size: u64, src: &Nv12Frame) -> usize {
    let limit = dst.len().min(max_size.min(usize::MAX as u64) as usize);
    let dst = &mut dst[..limit];
    let width = src.width as usize;
    let height = src.height as usize;
    if width == 0 || height == 0 {
        return 0;
    }
    let y_written = copy_rows(&mut dst[..limit], src.y, src.y_linesize, width, height);
    let chroma_rows = height / 2;
    let uv_written = copy_rows(
        &mut dst[y_written.min(limit)..limit],
        src.uv,
        src.uv_linesize,
        width,
        chroma_rows,
    );
    y_written + uv_written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planes_are_packed_after_their_padded_rows() {
        // Four 4-byte luma rows and two 4-byte chroma rows, each padded to a 5-byte stride.
        let y = [
            1u8, 2, 3, 4, 0xff, 5, 6, 7, 8, 0xff, 9, 10, 11, 12, 0xff, 13, 14, 15, 16, 0xff,
        ];
        let uv = [17u8, 18, 19, 20, 0xff, 21, 22, 23, 24, 0xff];
        let frame = Nv12Frame {
            width: 4,
            height: 4,
            y: &y,
            uv: &uv,
            y_linesize: 5,
            uv_linesize: 5,
        };
        let mut dst = vec![0u8; 24];
        assert_eq!(copy_nv12_data(&mut dst, 24, &frame), 24);
        assert_eq!(
            &dst[..16],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
        assert_eq!(
            &dst[16..24],
            &[17, 18, 19, 20, 21, 22, 23, 24],
            "the chroma follows the tightly packed luma"
        );
    }

    #[test]
    fn max_size_stops_the_copy_inside_the_luma_plane() {
        let y = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let frame = Nv12Frame {
            width: 4,
            height: 2,
            y: &y,
            uv: &[9, 10, 11, 12],
            y_linesize: 4,
            uv_linesize: 4,
        };
        let mut dst = vec![0u8; 12];
        // Six bytes of room fit one full luma row; the second one needs eight.
        assert_eq!(copy_nv12_data(&mut dst, 6, &frame), 4);
        assert_eq!(&dst[..4], &[1, 2, 3, 4]);
        assert_eq!(&dst[4..], &[0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
