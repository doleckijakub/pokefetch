use std::borrow::Cow;

pub struct Assets {
    pub sheet: Cow<'static, [u8]>,
    pub dex: Cow<'static, [u8]>,
}

#[cfg(feature = "embed")]
pub fn load() -> Result<Assets, String> {
    Ok(Assets {
        sheet: Cow::Borrowed(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/heartgold-soulsilver.png"
        ))),
        dex: Cow::Borrowed(include_bytes!(concat!(env!("OUT_DIR"), "/pokedex.json"))),
    })
}

#[cfg(not(feature = "embed"))]
pub fn load() -> Result<Assets, String> {
    use std::{env, fs, path::PathBuf};

    let dir = env::var_os("POKEFETCH_ASSET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("POKEFETCH_ASSET_DIR")));
    let read = |name: &str| {
        let path = dir.join(name);
        fs::read(&path).map(Cow::Owned).map_err(|e| {
            format!(
                "cannot read {}: {e}\nRebuild, or point POKEFETCH_ASSET_DIR at a directory containing the assets.",
                path.display()
            )
        })
    };
    Ok(Assets {
        sheet: read("heartgold-soulsilver.png")?,
        dex: read("pokedex.json")?,
    })
}
