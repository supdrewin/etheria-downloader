use super::{VersionFiles, VERSION_FILES_JSON};

use wuwa_dl::utils::Result;

#[test]
fn version_files() -> Result<()> {
    let json = serde_json::from_str::<VersionFiles>(VERSION_FILES_JSON)?;
    let json = serde_json::to_string_pretty(&json)?;

    Ok(println!("{json}"))
}
