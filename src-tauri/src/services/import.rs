use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use image::imageops::FilterType;
use thiserror::Error;

use crate::{
    domain::asset::{Asset, AssetSource},
    repository::assets::{AssetRepository, AssetRepositoryError},
};

static NEXT_ASSET_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("failed to access image files: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode or encode an image: {0}")]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Repository(#[from] AssetRepositoryError),
}

pub fn import_files(
    paths: &[PathBuf],
    data_directory: &Path,
    repository: &AssetRepository,
) -> Result<Vec<Asset>, ImportError> {
    paths
        .iter()
        .map(|path| persist_image(path, data_directory, repository, AssetSource::Import))
        .collect()
}

pub(crate) fn persist_image(
    source_path: &Path,
    data_directory: &Path,
    repository: &AssetRepository,
    source: AssetSource,
) -> Result<Asset, ImportError> {
    let id = new_asset_id();
    let originals_directory = data_directory.join("assets").join("originals");
    let previews_directory = data_directory.join("assets").join("previews");
    fs::create_dir_all(&originals_directory)?;
    fs::create_dir_all(&previews_directory)?;

    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .unwrap_or("img");
    let original_path = originals_directory.join(format!("{id}.{extension}"));
    let preview_path = previews_directory.join(format!("{id}.png"));

    let result = (|| {
        fs::copy(source_path, &original_path)?;
        let image = image::open(&original_path)?;
        image
            .resize(512, 512, FilterType::Triangle)
            .save(&preview_path)?;

        let now = Utc::now();
        let asset = Asset {
            id,
            created_at: now,
            imported_at: now,
            source,
            original_path: original_path.clone(),
            preview_path: preview_path.clone(),
            album_id: None,
            tags: Vec::new(),
            favorite: false,
            sync_version: 0,
        };
        repository.create(&asset)?;
        Ok(asset)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&original_path);
        let _ = fs::remove_file(&preview_path);
    }
    result
}

fn new_asset_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before the Unix epoch")
        .as_nanos();
    let sequence = NEXT_ASSET_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("asset-{timestamp}-{sequence}")
}
