use std::{collections::HashMap, fmt::Write, io, ops::Deref, path::Path, time::Duration};

use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use wuwa_dl::utils::Result;

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
    pb: ProgressBar,
}

impl Deref for FileHelper {
    type Target = FileInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl FileHelper {
    pub fn new(inner: FileInner) -> Self {
        let FileInner { path, size, .. } = &inner;

        let pb = ProgressBar::new(*size);
        let path = Path::new(path);

        let file_name = path.file_name().unwrap().to_str().unwrap();
        let file_name = match file_name.len() {
            0..40 => file_name.to_string(),
            _ => format!("{}...", &file_name[..36]),
        };

        let style = ProgressStyle::with_template(Self::STYLE)
            .unwrap()
            .with_key("file_name", move |_: &ProgressState, w: &mut dyn Write| {
                write!(w, "{file_name}").unwrap()
            })
            .progress_chars("##-");

        pb.set_style(style);
        Self { inner, pb }
    }

    pub fn with_multi_progress(self, mp: MultiProgress) -> Self {
        let Self { inner, pb } = self;

        let pb = mp.add(pb);
        Self { inner, pb }
    }

    pub async fn download(&self) -> Result<()> {
        let path = Path::new(&self.path);

        fs::create_dir_all(path.parent().unwrap()).await?;

        while match self.verify().await {
            Ok(downloaded) => !downloaded,
            Err(_) => true,
        } {
            self.pb.set_position(0);

            let mut file = File::create(path).await?;
            let mut stream = reqwest::get(&self.url).await?.bytes_stream();

            while let Some(chunk) = stream.next().await {
                self.write_bytes(&mut file, &chunk?).await?;
            }

            file.flush().await?;
        }

        Ok(self.pb.finish())
    }
}

impl FileHelper {
    const STYLE: &str = r"{spinner:.green} {file_name:40} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}";

    async fn verify(&self) -> Result<bool> {
        let mut file = File::open(&self.path).await?.into_std().await;
        let mut hasher = Md5::new();

        self.pb.set_position(self.size);
        self.pb.enable_steady_tick(Duration::from_millis(20));

        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        self.pb.disable_steady_tick();

        Ok(format!("{hash:02x}").eq(&self.hash))
    }

    async fn write_bytes(&self, file: &mut File, chunk: &[u8]) -> Result<()> {
        file.write_all(&chunk).await?;
        Ok(self.pb.inc(chunk.len() as u64))
    }
}

#[cfg(test)]
mod tests;
