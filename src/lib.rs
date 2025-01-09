use std::{collections::HashMap, error::Error, fmt::Write, fs, io, ops::Deref, path::Path};

use base16ct::lower;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

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
    pub pb: ProgressBar,
    inner: FileInner,
}

impl Deref for FileHelper {
    type Target = FileInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl FileHelper {
    const STYLE: &str = r"{spinner:.green} {file_name:40} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}";

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

    pub async fn download(&mut self) -> Result<()> {
        while match self.verify().await {
            Ok(downloaded) => !downloaded,
            Err(_) => true,
        } {
            let path = Path::new(&self.path);

            fs::create_dir_all(path.parent().unwrap())?;

            let mut file = File::create(path).await?;
            let mut stream = reqwest::get(&self.url).await?.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;

                file.write_all(&chunk).await?;
                self.pb.inc(chunk.len() as u64);
            }

            file.flush().await?;
        }

        Ok(self.pb.finish())
    }

    async fn verify(&self) -> Result<bool> {
        let mut file = File::open(&self.path).await?.into_std().await;
        let mut hasher = Md5::new();

        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        let hash = lower::encode_string(&hash);

        Ok(hash.eq(&self.hash))
    }
}

#[cfg(test)]
mod tests;
