use console::Term;
use indicatif::MultiProgress;
use tokio::runtime::Builder;
use wuwa_dl::{
    helper::ResourceHelperExt,
    pool::{Pool, PoolOp},
    utils::Result,
};

use etheria_downloader::{FileHelper, VersionFiles, VERSION_FILES_JSON};

fn main() -> Result<()> {
    let VersionFiles { files, .. } = serde_json::from_str(VERSION_FILES_JSON)?;

    let rt = Builder::new_multi_thread().enable_all().build()?;
    let mp = MultiProgress::new();

    rt.block_on(async {
        let mut pool = Pool::new()?;
        let mut tasks = vec![];

        for (_, inner) in files {
            let sender = pool.sender.clone();
            let mp = mp.clone();

            pool.watcher.changed().await?;
            sender.send(PoolOp::Attach).await?;

            tasks.push(rt.spawn(async move {
                let helper = FileHelper::new(inner)
                    .with_progress_bar()
                    .with_multi_progress(mp);

                wuwa_dl::while_err! { helper.download().await }
                sender.send(PoolOp::Dettach).await
            }));
        }

        wuwa_dl::wait_all!(tasks, 2);

        println!("All the resources are downloaded!");
        println!("Press any key to continue...");

        Ok(Term::stdout().read_key().map(|_| ())?)
    })
}
