mod assets;
mod layout;
mod sprite;

use serde::Deserialize;
use std::{
    hash::{BuildHasher, RandomState},
    io::{self, IsTerminal, Write},
    process::ExitCode,
};

#[derive(Deserialize)]
struct Pokemon {
    id: u32,
    name: Names,
    #[serde(rename = "type")]
    types: Vec<String>,
    base: Option<Base>,
    species: String,
    description: String,
    profile: Profile,
}

#[derive(Deserialize)]
struct Names {
    english: String,
}

#[derive(Deserialize)]
struct Base {
    #[serde(rename = "HP")]
    hp: u32,
    #[serde(rename = "Attack")]
    attack: u32,
    #[serde(rename = "Defense")]
    defense: u32,
    #[serde(rename = "Sp. Attack")]
    sp_attack: u32,
    #[serde(rename = "Sp. Defense")]
    sp_defense: u32,
    #[serde(rename = "Speed")]
    speed: u32,
}

#[derive(Deserialize)]
struct Profile {
    height: String,
    weight: String,
    ability: Vec<(String, String)>,
}

const USAGE: &str = "usage: pokefetch [NUMBER|NAME] [--scale N] [--no-image]";

const TEXT_WIDTH: usize = 44;
const BAR_WIDTH: u32 = 22;

struct Args {
    query: Option<String>,
    scale: Option<u32>,
    image: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        query: None,
        scale: None,
        image: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(USAGE.into()),
            "--no-image" => args.image = false,
            "--scale" => {
                args.scale = Some(
                    it.next()
                        .and_then(|s| s.parse().ok())
                        .filter(|&n| (1..=8).contains(&n))
                        .ok_or("--scale needs a number from 1 to 8")?,
                );
            }
            _ if args.query.is_none() && !arg.starts_with('-') => args.query = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'\n{USAGE}")),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pokefetch: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let assets = assets::load()?;
    let sheet = sprite::decode_sheet(&assets.sheet).map_err(|e| format!("sprite sheet: {e}"))?;
    let dex: Vec<Pokemon> =
        serde_json::from_slice(&assets.dex).map_err(|e| format!("pokedex.json: {e}"))?;

    let available: Vec<&Pokemon> = dex
        .iter()
        .filter(|p| p.id >= 1 && p.id <= sprite::capacity(&sheet))
        .collect();
    if available.is_empty() {
        return Err("no Pokémon found in the data".into());
    }

    let mon = match &args.query {
        Some(q) => available
            .iter()
            .find(|p| q.parse() == Ok(p.id) || p.name.english.eq_ignore_ascii_case(q))
            .copied()
            .ok_or_else(|| format!("no Pokémon '{q}' in #1–#{}", available.len()))?,
        None => {
            let r = RandomState::new().hash_one(std::time::SystemTime::now());
            available[(r % available.len() as u64) as usize]
        }
    };

    let stdout = io::stdout();
    let tty = stdout.is_terminal();
    let mut out = io::BufWriter::new(stdout.lock());
    let io_err = |e: io::Error| e.to_string();
    let lines = info_lines(mon, tty);

    if args.image && tty {
        let term = layout::term();
        let scale = args.scale.unwrap_or_else(|| match &term {
            Some(t) => (lines.len() as u32 * t.cell_h / 80).clamp(1, 5),
            None => 2,
        });
        if let Some(img) = sprite::extract(&sheet, mon.id, scale) {
            let placed = match &term {
                Some(t) => layout::side_by_side(&img, &lines, TEXT_WIDTH as u32, t, &mut out)
                    .map_err(io_err)?,
                None => false,
            };
            if placed {
                return out.flush().map_err(io_err);
            }
            sprite::print_kitty(&img, "", &mut out).map_err(io_err)?;
            writeln!(out).map_err(io_err)?;
        }
    }
    for line in &lines {
        writeln!(out, "{line}").map_err(io_err)?;
    }
    out.flush().map_err(io_err)
}

struct Style {
    color: bool,
}

impl Style {
    fn sgr(&self, codes: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{codes}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn fg(&self, (r, g, b): (u8, u8, u8), text: &str) -> String {
        self.sgr(&format!("38;2;{r};{g};{b}"), text)
    }

    fn bold(&self, text: &str) -> String {
        self.sgr("1", text)
    }

    fn dim(&self, text: &str) -> String {
        self.fg((130, 130, 140), text)
    }

    fn label(&self, text: &str) -> String {
        self.dim(&format!("{text:<9}"))
    }

    fn badge(&self, (r, g, b): (u8, u8, u8), text: &str) -> String {
        if self.color {
            format!("\x1b[1;38;2;20;20;20;48;2;{r};{g};{b}m {text} \x1b[0m")
        } else {
            format!("[{text}]")
        }
    }
}

fn info_lines(p: &Pokemon, color: bool) -> Vec<String> {
    let s = Style { color };
    let mut lines = Vec::new();

    let number = format!("#{:03}", p.id);
    lines.push(format!(
        "{}{}{}",
        s.bold(&p.name.english),
        " ".repeat(TEXT_WIDTH - p.name.english.chars().count() - number.len()),
        s.dim(&number),
    ));
    lines.push(s.dim(&"─".repeat(TEXT_WIDTH)));
    lines.push(s.sgr("3", &p.species));
    lines.push(String::new());

    let badges: Vec<String> = p.types.iter().map(|t| s.badge(type_color(t), t)).collect();
    lines.push(format!("{}{}", s.label("Type"), badges.join(" ")));
    lines.push(format!("{}{}", s.label("Height"), p.profile.height));
    lines.push(format!("{}{}", s.label("Weight"), p.profile.weight));
    let (hidden, normal): (Vec<_>, Vec<_>) =
        p.profile.ability.iter().partition(|(_, h)| h == "true");
    let names = |v: &[&(String, String)]| {
        v.iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !normal.is_empty() {
        lines.push(format!("{}{}", s.label("Ability"), names(&normal)));
    }
    if !hidden.is_empty() {
        lines.push(format!("{}{}", s.label("Hidden"), names(&hidden)));
    }

    if let Some(b) = &p.base {
        lines.push(String::new());
        let stats = [
            ("HP", b.hp),
            ("Attack", b.attack),
            ("Defense", b.defense),
            ("Sp. Atk", b.sp_attack),
            ("Sp. Def", b.sp_defense),
            ("Speed", b.speed),
        ];
        for (label, v) in stats {
            lines.push(format!("{}{v:>3}  {}", s.label(label), gauge(&s, v)));
        }
        let total: u32 = stats.iter().map(|st| st.1).sum();
        lines.push(format!(
            "{}{}",
            s.label("Total"),
            s.bold(&format!("{total:>3}"))
        ));
    }

    lines.push(String::new());
    lines.extend(wrap(&p.description, TEXT_WIDTH).iter().map(|l| s.dim(l)));
    lines
}

fn gauge(s: &Style, v: u32) -> String {
    const PARTIAL: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let eighths = (v.min(255) * BAR_WIDTH * 8).div_ceil(255);
    let (full, part) = (eighths / 8, eighths % 8);
    let bar = format!("{}{}", "█".repeat(full as usize), PARTIAL[part as usize]);
    let used = full + u32::from(part > 0);
    let track = "·".repeat((BAR_WIDTH - used) as usize);
    format!(
        "{}{}",
        s.fg(stat_color(v), &bar),
        s.sgr("38;2;70;70;80", &track)
    )
}

fn stat_color(v: u32) -> (u8, u8, u8) {
    match v {
        0..60 => (240, 110, 100),
        60..90 => (240, 200, 90),
        90..120 => (140, 210, 110),
        _ => (100, 200, 230),
    }
}

fn type_color(t: &str) -> (u8, u8, u8) {
    match t {
        "Normal" => (168, 168, 120),
        "Fire" => (240, 128, 48),
        "Water" => (104, 144, 240),
        "Electric" => (248, 208, 48),
        "Grass" => (120, 200, 80),
        "Ice" => (152, 216, 216),
        "Fighting" => (192, 48, 40),
        "Poison" => (160, 64, 160),
        "Ground" => (224, 192, 104),
        "Flying" => (168, 144, 240),
        "Psychic" => (248, 88, 136),
        "Bug" => (168, 184, 32),
        "Rock" => (184, 160, 56),
        "Ghost" => (112, 88, 152),
        "Dragon" => (112, 56, 248),
        "Dark" => (112, 88, 72),
        "Steel" => (184, 184, 208),
        "Fairy" => (238, 153, 172),
        _ => (200, 200, 200),
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
