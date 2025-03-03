use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use serde::Deserialize;

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
