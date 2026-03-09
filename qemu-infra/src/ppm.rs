use std::path::Path;

use anyhow::anyhow;
use image::{ImageBuffer, Rgb};

pub struct PpmImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGB, 3 bytes per pixel
}

impl PpmImage {
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        Self::parse(&data)
    }

    pub fn parse(data: &[u8]) -> anyhow::Result<Self> {
        let mut pos = 0;

        let magic = read_token(data, &mut pos)?;
        if magic != "P6" {
            return Err(anyhow!("not a binary PPM (expected P6, got {magic})"));
        }

        let width: u32 = read_token(data, &mut pos)?.parse()?;
        let height: u32 = read_token(data, &mut pos)?.parse()?;
        let maxval: u32 = read_token(data, &mut pos)?.parse()?;
        if maxval != 255 {
            return Err(anyhow!("unsupported maxval {maxval} (expected 255)"));
        }

        // After maxval there's exactly one whitespace byte, then pixel data
        let expected = (width * height * 3) as usize;
        let remaining = data.len() - pos;
        if remaining < expected {
            return Err(anyhow!(
                "truncated pixel data: need {expected} bytes, got {remaining}"
            ));
        }

        Ok(Self {
            width,
            height,
            pixels: data[pos..pos + expected].to_vec(),
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = ((y * self.width + x) * 3) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }

    pub fn has_non_black_pixels(&self) -> bool {
        self.pixels.iter().any(|&b| b != 0)
    }

    pub fn to_png_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let img: ImageBuffer<Rgb<u8>, _> =
            ImageBuffer::from_raw(self.width, self.height, self.pixels.clone())
                .ok_or_else(|| anyhow!("pixel data size mismatch"))?;
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)?;
        Ok(buf.into_inner())
    }
}

fn read_token(data: &[u8], pos: &mut usize) -> anyhow::Result<String> {
    // Skip whitespace and comments
    while *pos < data.len() {
        let b = data[*pos];
        if b == b'#' {
            // Skip comment line
            while *pos < data.len() && data[*pos] != b'\n' {
                *pos += 1;
            }
        } else if b.is_ascii_whitespace() {
            *pos += 1;
        } else {
            break;
        }
    }

    let start = *pos;
    while *pos < data.len() && !data[*pos].is_ascii_whitespace() {
        *pos += 1;
    }

    if start == *pos {
        return Err(anyhow!("unexpected end of PPM header"));
    }

    let token_end = *pos;

    // Consume exactly one trailing whitespace byte (required by PPM spec after maxval)
    if *pos < data.len() && data[*pos].is_ascii_whitespace() {
        *pos += 1;
    }

    Ok(String::from_utf8_lossy(&data[start..token_end]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_ppm() {
        let mut data = Vec::new();
        data.extend_from_slice(b"P6\n2 2\n255\n");
        // 4 pixels: red, green, blue, white
        data.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        let img = PpmImage::parse(&data).unwrap();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);
        assert_eq!(img.pixel(0, 0), (255, 0, 0));
        assert_eq!(img.pixel(1, 0), (0, 255, 0));
        assert_eq!(img.pixel(0, 1), (0, 0, 255));
        assert_eq!(img.pixel(1, 1), (255, 255, 255));
        assert!(img.has_non_black_pixels());
    }

    #[test]
    fn all_black() {
        let mut data = Vec::new();
        data.extend_from_slice(b"P6 1 1 255\n");
        data.extend_from_slice(&[0, 0, 0]);
        let img = PpmImage::parse(&data).unwrap();
        assert!(!img.has_non_black_pixels());
    }
}
