use console::Term;
use indicatif::MultiProgress;

use etheria_downloader::{FileHelper, Pool, Result, VersionFiles, VERSION_FILES_JSON};

#[tokio::main]
async fn main() -> Result<()> {
    let VersionFiles { files, .. } = serde_json::from_str(VERSION_FILES_JSON)?;

    let pool = Pool::new(num_cpus::get());
    let mp = MultiProgress::new();

    let mut handles = vec![];

    for inner in files.into_values() {
        let pool = pool.attach().await;
        let mp = mp.clone();

        handles.push(tokio::spawn(async move {
            let mut helper = FileHelper::new(inner);
            helper.pb = mp.add(helper.pb);

            while helper.download().await.is_err() {}
            pool.dettach().await;
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
