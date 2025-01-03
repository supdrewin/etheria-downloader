use std::{error::Error, fmt::Write, fs, io, ops::Deref, sync::Arc, thread, time::Duration};

use base16ct::lower;
use console::Term;
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt, sync::Mutex};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const LIVE_PATCH_JSON: &str = include_str!("../assets/live_patch_version_2846214.json");
const BASE_PATH: &str = r"http://etheria-static.xdcdn.com/cbt3/0.6/Android";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Pak {
    patch_version: u64,
    patch_pak: String,
    chunk_id: u64,
    pak_file_size: u64,
    md5_hash: String,
    material_updated: bool,
    necessary: bool,
    #[serde(rename = "PSOUpdated")]
    pso_updated: bool,
    device_profiles_updated: bool,
    game_user_settings_updated: bool,
    ini_settings_updated: bool,
    asset_registry_updated: bool,
    project_shader_updated: bool,
    global_shader_updated: bool,
    localization_opt: bool,
    streamed: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LivePatchJson {
    current_version: u64,
    patches: Vec<Pak>,
    base_paks: Vec<Pak>,
}

struct PakHelper {
    inner: Pak,
    pb: ProgressBar,
    path: String,
}

impl Deref for PakHelper {
    type Target = Pak;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PakHelper {
    const STYLE: &'static str = r"{spinner:.green} {file_name:40} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}";

    fn new(inner: Pak, prefix: &str) -> Result<Self> {
        let Pak {
            patch_pak,
            pak_file_size,
            ..
        } = &inner;

        let pb = ProgressBar::new(*pak_file_size);
        let path = format!("{prefix}/{patch_pak}");

        let file_name = patch_pak.clone();

        pb.set_style(
            ProgressStyle::with_template(Self::STYLE)?
                .with_key("file_name", move |_: &ProgressState, w: &mut dyn Write| {
                    write!(w, "{file_name}").unwrap()
                })
                .progress_chars("##-"),
        );

        Ok(Self { inner, pb, path })
    }

    async fn download(&mut self) -> Result<()> {
        while match self.verify() {
            Ok(downloaded) => !downloaded,
            Err(_) => true,
        } {
            let mut file = File::create(&self.path).await?;
            let mut stream = reqwest::get(&format!("{BASE_PATH}/{}", self.patch_pak))
                .await?
                .bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;

                file.write_all(&chunk).await?;
                self.pb.inc(chunk.len() as u64);
            }

            file.flush().await?;
        }

        Ok(self.pb.finish())
    }

    fn verify(&self) -> Result<bool> {
        let mut file = fs::File::open(&self.path)?;
        let mut hasher = Md5::new();

        io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        let hash = lower::encode_string(&hash);

        Ok(hash.eq(&self.md5_hash))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let LivePatchJson {
        patches, base_paks, ..
    } = serde_json::from_str(LIVE_PATCH_JSON)?;

    fs::create_dir_all("PatchPaks")?;
    fs::create_dir_all("Paks")?;

    let patches = patches
        .into_iter()
        .map(|inner| PakHelper::new(inner, "PatchPaks"));
    let base_paks = base_paks
        .into_iter()
        .map(|inner| PakHelper::new(inner, "Paks"));

    let threads = Arc::new(Mutex::new(num_cpus::get()));
    let multi_progress = MultiProgress::new();

    let mut handles = vec![];

    for pak in patches.chain(base_paks) {
        let threads = Arc::clone(&threads);
        let mut pak = pak?;

        while {
            let mut threads = threads.lock().await;

            threads.checked_sub(1).is_none_or(|t| {
                *threads = t;
                false
            })
        } {
            thread::sleep(Duration::from_millis(1));
        }

        pak.pb = multi_progress.add(pak.pb);

        handles.push(tokio::spawn(async move {
            pak.download().await?;
            Result::Ok(*threads.lock().await += 1)
        }));
    }

    for handle in handles {
        handle.await??;
    }

    Ok({
        println!("All resources downloaded!");
        Term::read_key(&Term::stdout())?;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_live_patch() -> Result<()> {
        let live_patch_json = serde_json::from_str::<LivePatchJson>(LIVE_PATCH_JSON)?;
        let live_patch_json = serde_json::to_string_pretty(&live_patch_json)?;

        Ok(println!("{live_patch_json}"))
    }
}
