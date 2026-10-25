//! Helpers the readers use beside the vocabularies: the elapsed time of a
//! timing report, raw-deflate payloads and the file name of a picture
//! payload.

use std::io::Read;
use std::time::Instant;

use anyhow::{Context, Result};
use flate2::read::DeflateDecoder;

pub fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

pub fn inflate_raw_deflate(input: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(input);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .context("failed to inflate raw deflate blob")?;
    Ok(output)
}

pub fn ext_picture_file_name(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "Picture.png"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "Picture.gif"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "Picture.jpg"
    } else if bytes.starts_with(b"BM") {
        "Picture.bmp"
    } else if bytes.starts_with(b"\x00\x00\x01\x00") {
        "Picture.ico"
    } else if bytes.starts_with(b"PK\x03\x04") {
        "Picture.zip"
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        let trimmed = text.trim_start_matches('\u{feff}').trim_start();
        if is_svg_text(text) {
            "Picture.svg"
        } else if trimmed.starts_with('<') {
            // A markup payload the platform does not recognise as an image is
            // written with no extension at all: ERP WE English
            // `CommonPictures/PictureBack` (an EDT metadata document stored as
            // the picture) is published `<xr:Abs>Picture</xr:Abs>`, and no
            // corpus publishes `Picture.xml`.
            "Picture"
        } else {
            "Picture.txt"
        }
    } else {
        "Picture.bin"
    }
}

pub fn is_svg_content(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    is_svg_text(text)
}

fn is_svg_text(text: &str) -> bool {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    text.starts_with("<svg") || text.starts_with("<?xml") && text.contains("<svg")
}
