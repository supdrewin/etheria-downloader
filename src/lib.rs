use std::{collections::HashMap, fmt::Write, path::Path};

use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use serde::{Deserialize, Serialize};

use wuwa_dl::{
    helper::{ResourceHelperBase, ResourceHelperExt},
    utils::PROGRESS_STYLE,
};

pub const VERSION_FILES_JSON: &str = include_str!("../assets/version_files_309402.json");

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionFiles {
    pub version: String,
    pub display_version: String,
    pub command: Command,
    pub asset: Asset,
    pub files: HashMap<String, FileInner>,
}

#[derive(Serialize, Deserialize)]
pub struct Command {
    pub exe: String,
    pub params: String,
}

#[derive(Serialize, Deserialize)]
pub struct Asset {
    pub current: String,
    pub assets: Vec<Pair>,
}

#[derive(Serialize, Deserialize)]
pub struct Pair {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInner {
    pub hash: String,
    pub path: String,
    pub size: u64,
    pub url: String,
    pub is_downloaded: u8,
    pub downloaded_size: u64,
}

pub struct FileHelper {
    inner: FileInner,
    pb: Option<ProgressBar>,
}

impl ResourceHelperBase for FileHelper {
    fn md5(&self) -> &str {
        &self.inner.hash
    }

    fn size(&self) -> u64 {
        self.inner.size
    }

    fn download_src(&self) -> &str {
        &self.inner.url
    }

    fn download_dest(&self) -> &Path {
        Path::new(&self.inner.path)
    }

    fn pb(&self) -> &Option<ProgressBar> {
        &self.pb
    }
}

impl ResourceHelperExt for FileHelper {}

impl FileHelper {
    pub fn new(inner: FileInner) -> Self {
        Self { inner, pb: None }
    }

    pub fn with_progress_bar(self) -> Self {
        let Self { inner, .. } = self;

        let pb = ProgressBar::new(inner.size);
        let path = Path::new(&inner.path);

        let file_name = path.file_name().unwrap().to_str().unwrap();
        let file_name = match file_name.len() {
            0..40 => file_name.to_string(),
            _ => format!("{}...", &file_name[..36]),
        };

        let style = ProgressStyle::with_template(PROGRESS_STYLE)
            .unwrap()
            .with_key("file_name", move |_: &ProgressState, w: &mut dyn Write| {
                write!(w, "{file_name}").unwrap()
            })
            .progress_chars("##-");

        pb.set_style(style);

        Self {
            inner,
            pb: Some(pb),
        }
    }

    pub fn with_multi_progress(self, mp: MultiProgress) -> Self {
        let pb = self.pb.and_then(|pb| Some(mp.add(pb)));
        Self { pb, ..self }
    }
}

#[cfg(test)]
mod tests;
