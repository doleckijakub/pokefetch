use crate::sprite::{self, Image};
use std::io::{self, Write};

const MARGIN: u32 = 1;
const GAP: u32 = 3;

pub struct Term {
    pub cols: u32,
    pub cell_w: u32,
    pub cell_h: u32,
}

pub fn term() -> Option<Term> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == 0;
    if !ok || ws.ws_col == 0 || ws.ws_row == 0 || ws.ws_xpixel == 0 || ws.ws_ypixel == 0 {
        return None;
    }
    Some(Term {
        cols: ws.ws_col.into(),
        cell_w: u32::from(ws.ws_xpixel) / u32::from(ws.ws_col),
        cell_h: u32::from(ws.ws_ypixel) / u32::from(ws.ws_row),
    })
}

pub fn side_by_side(
    img: &Image,
    lines: &[String],
    text_width: u32,
    term: &Term,
    out: &mut impl Write,
) -> io::Result<bool> {
    let img_cols = img.width.div_ceil(term.cell_w);
    let img_rows = img.height.div_ceil(term.cell_h);
    let text_col = MARGIN + img_cols + GAP;
    if text_col + text_width > term.cols {
        return Ok(false);
    }
    let rows = img_rows.max(lines.len() as u32);

    write!(out, "{}\x1b[{rows}A\r", "\n".repeat(rows as usize))?;

    let pad = (rows * term.cell_h - img.height) / 2;
    let (row_off, y_off) = (pad / term.cell_h, pad % term.cell_h);
    if row_off > 0 {
        write!(out, "\x1b[{row_off}B")?;
    }
    write!(out, "\x1b[{MARGIN}C")?;
    sprite::print_kitty(img, &format!(",C=1,Y={y_off}"), out)?;
    write!(out, "\r")?;
    if row_off > 0 {
        write!(out, "\x1b[{row_off}A")?;
    }

    for line in lines {
        writeln!(out, "\x1b[{text_col}C{line}")?;
    }
    for _ in lines.len() as u32..rows {
        writeln!(out)?;
    }
    Ok(true)
}
