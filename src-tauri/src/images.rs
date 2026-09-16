//! Image clipboard helpers: format sniffing, thumbnails, and data URIs.
//!
//! Kept out of `clipboard` because `storage::retention` needs the thumbnail
//! path convention too — a deleted clip has to take its sidecar with it.

use std::path::{Path, PathBuf};

use image::ImageEncoder;

/// Longest edge of a generated thumbnail, in pixels.
const THUMBNAIL_MAX_EDGE: u32 = 256;

/// An image format we recognise on the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageKind {
    /// File extension to store the original under, without a dot.
    pub extension: &'static str,
    /// MIME type, for metadata and data URIs.
    pub mime: &'static str,
}

/// Identify an image by its magic bytes.
///
/// Sniffing the content rather than trusting the clipboard's advertised target
/// matters because `xclip -o` serves whatever the selection owner offers
/// regardless of the target asked for. Without this check any non-UTF8 payload
/// — a latin-1 text selection, say — would be stored as if it were an image.
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageKind> {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    if bytes.starts_with(PNG) {
        return Some(ImageKind {
            extension: "png",
            mime: "image/png",
        });
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageKind {
            extension: "jpg",
            mime: "image/jpeg",
        });
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageKind {
            extension: "gif",
            mime: "image/gif",
        });
    }
    // RIFF....WEBP
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(ImageKind {
            extension: "webp",
            mime: "image/webp",
        });
    }
    if bytes.starts_with(b"BM") {
        return Some(ImageKind {
            extension: "bmp",
            mime: "image/bmp",
        });
    }
    None
}

/// The thumbnail that belongs to a stored original.
///
/// By convention it sits beside the original as `{stem}_thumb.webp`. Deriving
/// it rather than storing a second path keeps retention able to clean up a
/// sidecar for a clip written by an older build.
pub fn thumbnail_path_for(image_path: &Path) -> PathBuf {
    let stem = image_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    image_path.with_file_name(format!("{stem}_thumb.webp"))
}

/// Decode `bytes`, scale the longest edge down to `THUMBNAIL_MAX_EDGE`, and
/// write a lossless WebP thumbnail to `dest`.
///
/// Returns the *original* dimensions, which the card footer displays.
pub fn write_thumbnail(bytes: &[u8], dest: &Path) -> Result<(u32, u32), String> {
    let decoded = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let (width, height) = (decoded.width(), decoded.height());

    // `thumbnail` preserves aspect ratio and only ever shrinks, so a small
    // image is stored at its own size rather than being blown up.
    let thumb = decoded.thumbnail(THUMBNAIL_MAX_EDGE, THUMBNAIL_MAX_EDGE);
    let rgba = thumb.to_rgba8();

    let mut out = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut out)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;

    std::fs::write(dest, &out).map_err(|e| e.to_string())?;
    Ok((width, height))
}

/// Delete a stored original and the thumbnail beside it.
///
/// Every path that removes an image clip has to go through here: the thumbnail
/// is a second file the database never names, so deleting only `image_path`
/// leaks it forever.
pub fn remove_image_and_thumbnail(image_path: &Path) {
    for path in [image_path.to_path_buf(), thumbnail_path_for(image_path)] {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("Failed to remove {}: {e}", path.display());
            }
        }
    }
}

/// Wrap bytes as a `data:` URI.
///
/// Thumbnails reach the frontend this way rather than over Tauri's asset
/// protocol: it keeps the "no direct file access from the frontend" boundary
/// from CLAUDE.md, and avoids widening the CSP for the whole app so that one
/// card can show a picture.
pub fn to_data_uri(bytes: &[u8], mime: &str) -> String {
    format!("data:{mime};base64,{}", base64_encode(bytes))
}

/// Minimal standard-alphabet base64 encoder with padding.
///
/// Hand-rolled because the only caller needs a few KB of thumbnail turned into
/// a data URI, and adding a dependency for that is a decision CLAUDE.md asks to
/// escalate.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[(triple >> 18) as usize & 0x3F] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 0x3F] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3F] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3F] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest valid PNG the `image` crate will decode: an 8x8 RGB image.
    fn tiny_png() -> Vec<u8> {
        let mut buf = Vec::new();
        let img = image::RgbImage::from_fn(8, 8, |x, y| {
            image::Rgb([(x * 30) as u8, (y * 30) as u8, 120])
        });
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn test_detect_png() {
        let kind = detect_image_format(&tiny_png()).unwrap();
        assert_eq!(kind.extension, "png");
        assert_eq!(kind.mime, "image/png");
    }

    #[test]
    fn test_detect_other_formats_by_magic() {
        assert_eq!(
            detect_image_format(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap().mime,
            "image/jpeg"
        );
        assert_eq!(detect_image_format(b"GIF89a....").unwrap().extension, "gif");
        assert_eq!(
            detect_image_format(b"RIFF\x00\x00\x00\x00WEBPVP8 ")
                .unwrap()
                .extension,
            "webp"
        );
        assert_eq!(detect_image_format(b"BM\x00\x00").unwrap().extension, "bmp");
    }

    #[test]
    fn test_non_image_bytes_are_not_images() {
        // Latin-1 text is non-UTF8 but must not be stored as an image.
        assert!(detect_image_format(&[0xE9, 0xE8, 0xFC]).is_none());
        assert!(detect_image_format(b"").is_none());
        assert!(detect_image_format(b"hello").is_none());
        // Truncated RIFF header must not panic or match.
        assert!(detect_image_format(b"RIFF").is_none());
    }

    #[test]
    fn test_thumbnail_path_sits_beside_the_original() {
        let p = thumbnail_path_for(Path::new("/data/images/abc-123.png"));
        assert_eq!(p, PathBuf::from("/data/images/abc-123_thumb.webp"));
        // Extension of the original does not leak into the thumbnail name.
        let p = thumbnail_path_for(Path::new("/data/images/abc-123.jpg"));
        assert_eq!(p, PathBuf::from("/data/images/abc-123_thumb.webp"));
    }

    #[test]
    fn test_write_thumbnail_reports_original_dimensions() {
        let dir = std::env::temp_dir().join(format!("paste-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("t.webp");

        let (w, h) = write_thumbnail(&tiny_png(), &dest).unwrap();
        assert_eq!((w, h), (8, 8), "reports the original size, not the thumb");
        assert!(dest.exists());
        assert!(std::fs::metadata(&dest).unwrap().len() > 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_thumbnail_rejects_non_image() {
        let dest = std::env::temp_dir().join("paste-thumb-never.webp");
        assert!(write_thumbnail(b"not an image at all", &dest).is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn test_base64_known_vectors() {
        // RFC 4648 test vectors — these pin the padding behaviour.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // Bytes above 0x7F must survive.
        assert_eq!(base64_encode(&[0xFF, 0xFE, 0xFD]), "//79");
    }

    #[test]
    fn test_data_uri_shape() {
        assert_eq!(
            to_data_uri(b"foo", "image/webp"),
            "data:image/webp;base64,Zm9v"
        );
    }
}
