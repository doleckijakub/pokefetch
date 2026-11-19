use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

const ASSETS: &[(&str, &str)] = &[
    (
        "heartgold-soulsilver.png",
        "https://veekun.com/static/pokedex/downloads/heartgold-soulsilver.png",
    ),
    (
        "pokedex.json",
        "https://raw.githubusercontent.com/Purukitto/pokemon-data.json/refs/heads/master/pokedex.json",
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=POKEFETCH_OFFLINE_DIR");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let offline = env::var_os("POKEFETCH_OFFLINE_DIR").map(PathBuf::from);

    for (name, url) in ASSETS {
        let dest = out.join(name);
        if dest.exists() {
            continue;
        }
        let bytes = match &offline {
            Some(dir) => {
                let src = dir.join(name);
                fs::read(&src).unwrap_or_else(|e| panic!("reading {}: {e}", src.display()))
            }
            None => download(url),
        };
        write_atomic(&dest, &bytes);
    }

    println!("cargo:rustc-env=POKEFETCH_ASSET_DIR={}", out.display());
}

fn download(url: &str) -> Vec<u8> {
    let resp = ureq::get(url)
        .set("User-Agent", "pokefetch-build")
        .call()
        .unwrap_or_else(|e| {
            panic!("downloading {url}: {e}\n(set POKEFETCH_OFFLINE_DIR to build offline)")
        });
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("reading response from {url}: {e}"));
    buf
}

fn write_atomic(dest: &Path, bytes: &[u8]) {
    let tmp = dest.with_extension("part");
    fs::write(&tmp, bytes).unwrap_or_else(|e| panic!("writing {}: {e}", tmp.display()));
    fs::rename(&tmp, dest).unwrap_or_else(|e| panic!("renaming to {}: {e}", dest.display()));
}
