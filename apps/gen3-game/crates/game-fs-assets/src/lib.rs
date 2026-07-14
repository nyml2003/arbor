//! Filesystem adapter for deterministic asset plans.

#![forbid(unsafe_code)]

use std::{fs, io, path::Path};

use game_asset_plan::{AssetBytes, AssetRequest};
use map_assets::TileSource;

pub fn read_tile_sources(root: &Path) -> io::Result<Vec<TileSource>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| at_path(root, error))? {
        let path = entry.map_err(|error| at_path(root, error))?.path();
        if path.extension().is_some_and(|extension| extension == "png") {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("tile filename is not UTF-8: {}", path.display()),
                    )
                })?
                .to_owned();
            let bytes = fs::read(&path).map_err(|error| at_path(&path, error))?;
            Ok(TileSource { name, bytes })
        })
        .collect()
}

pub fn read_asset_requests(
    root: &Path,
    requests: Vec<AssetRequest>,
) -> io::Result<Vec<AssetBytes>> {
    requests
        .into_iter()
        .map(|request| {
            let path = root.join(&request.relative_path);
            let bytes = fs::read(&path).map_err(|error| at_path(&path, error))?;
            Ok(AssetBytes { request, bytes })
        })
        .collect()
}

pub fn read_optional_text(path: &Path) -> io::Result<Option<String>> {
    path.is_file()
        .then(|| fs::read_to_string(path).map_err(|error| at_path(path, error)))
        .transpose()
}

fn at_path(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}
