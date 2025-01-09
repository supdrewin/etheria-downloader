use std::{sync::Arc, time::Duration};

use console::Term;
use indicatif::MultiProgress;
use tokio::{sync::Mutex, time};

use etheria_downloader::{FileHelper, Result, VersionFiles, VERSION_FILES_JSON};

#[tokio::main]
async fn main() -> Result<()> {
    let VersionFiles { files, .. } = serde_json::from_str(VERSION_FILES_JSON)?;

    let threads = Arc::new(Mutex::new(num_cpus::get()));
    let mp = MultiProgress::new();

    let mut handles = vec![];

    for inner in files.into_values() {
        let threads = Arc::clone(&threads);
        let mut helper = FileHelper::new(inner);

        while {
            time::sleep(Duration::from_millis(1)).await;

            let mut threads = threads.lock().await;
            let status = threads.checked_sub(1);

            match status {
                Some(t) => *threads = t,
                None => (),
            }

            status.is_none()
        } {}

        helper.pb = mp.add(helper.pb);

        handles.push(tokio::spawn(async move {
            let mut result;

            while {
                result = helper.download().await;
                result.is_err()
            } {}

            *threads.lock().await += 1;
        }));
    }

    for handle in handles {
        handle.await?;
    }

    Ok({
        println!("All the resources are downloaded!");
        println!("Press any key to continue...");

        Term::read_key(&Term::stdout())?;
    })
}
