use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::{self, Cursor, Write};

const CELL: u32 = 80;

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub fn decode_sheet(png_bytes: &[u8]) -> Result<Image, String> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("sprite sheet is too large")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "unexpected sprite sheet format {:?}/{:?}",
            info.color_type, info.bit_depth
        ));
    }
    pixels.truncate(info.buffer_size());
    Ok(Image {
        width: info.width,
        height: info.height,
        pixels,
    })
}

const SPRITE_COUNT: u32 = 493;

pub fn capacity(sheet: &Image) -> u32 {
    SPRITE_COUNT.min((sheet.width / CELL) * (sheet.height / CELL))
}

pub fn extract(sheet: &Image, id: u32, scale: u32) -> Option<Image> {
    let cols = sheet.width / CELL;
    let idx = id.checked_sub(1)?;
    if idx >= capacity(sheet) {
        return None;
    }
    let (x0, y0) = ((idx % cols) * CELL, (idx / cols) * CELL);
    let alpha = |x: u32, y: u32| sheet.pixels[((y * sheet.width + x) * 4 + 3) as usize];

    let (mut min_x, mut min_y, mut max_x, mut max_y) = (CELL, CELL, 0, 0);
    for y in 0..CELL {
        for x in 0..CELL {
            if alpha(x0 + x, y0 + y) != 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x {
        return None;
    }

    let (w, h) = (max_x - min_x + 1, max_y - min_y + 1);
    let (width, height) = (w * scale, h * scale);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let sx = x0 + min_x + x / scale;
            let sy = y0 + min_y + y / scale;
            let i = ((sy * sheet.width + sx) * 4) as usize;
            pixels.extend_from_slice(&sheet.pixels[i..i + 4]);
        }
    }
    Some(Image {
        width,
        height,
        pixels,
    })
}

pub fn print_kitty(img: &Image, keys: &str, out: &mut impl Write) -> io::Result<()> {
    let data = STANDARD.encode(&img.pixels);
    let mut chunks = data.as_bytes().chunks(4096).peekable();
    let mut first = true;
    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        if first {
            write!(
                out,
                "\x1b_Ga=T,f=32,s={},v={},q=2{keys},m={more};",
                img.width, img.height
            )?;
            first = false;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.write_all(chunk)?;
        out.write_all(b"\x1b\\")?;
    }
    Ok(())
}
