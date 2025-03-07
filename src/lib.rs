#![feature(int_roundings)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod animation;
pub mod collision;
pub mod editor;
pub mod ldtk;

pub fn deserialize_files<T>(
    file_paths: impl IntoIterator<Item = impl Into<PathBuf>>,
) -> HashMap<PathBuf, T>
where
    T: for<'a> Deserialize<'a>,
{
    let mut deserialized_items = HashMap::default();
    let mut successes: u32 = 0;
    let mut failures: u32 = 0;
    for path in file_paths.into_iter().map(Into::into) {
        let tile_depth_map = match deserialize_file(&path) {
            Ok(value) => value,
            Err(_) => {
                failures += 1;
                continue;
            }
        };
        successes += 1;
        deserialized_items.insert(path, tile_depth_map);
    }
    log::info!(
        "Deserialized {}/{} files successfully.",
        successes,
        failures
    );
    deserialized_items
}

pub fn deserialize_file<T>(path: impl Into<PathBuf>) -> Result<T, ()>
where
    T: for<'a> Deserialize<'a>,
{
    let path: PathBuf = path.into();

    if !path.is_file() {
        log::error!("{} is not a file!", path.display());
        return Err(());
    }

    let mut file = File::open(&path).map_err(|err| {
        log::error!("Could not open {}! [{}]", path.display(), err);
    })?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).map_err(|err| {
        log::error!("Could not read {}! [{}]", path.display(), err);
    })?;

    let tile_depth_map: T = ron::from_str(&buf).map_err(|err| {
        log::error!("Could not deserialize {}! [{}]", path.display(), err);
    })?;

    Ok(tile_depth_map)
}

pub fn serialize_to_file(serializable: impl Serialize, path: &str) -> bool {
    let buf = match ron::to_string(&serializable) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Could not serialize [{}]", &e);
            return false;
        }
    };
    let mut file = match File::options().create(true).write(true).open(path) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Could not open file [{}]", &e);
            return false;
        }
    };
    file.set_len(0);
    if file
        .write_all(buf.as_bytes())
        .map_err(|err| log::error!("Could not write to file [{}]", err))
        .is_err()
    {
        return false;
    };
    true
}
