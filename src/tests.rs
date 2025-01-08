use super::{Result, VersionFiles, VERSION_FILES_JSON};

#[test]
fn version_files() -> Result<()> {
    let json = serde_json::from_str::<VersionFiles>(VERSION_FILES_JSON)?;
    let json = serde_json::to_string_pretty(&json)?;

    Ok(println!("{json}"))
}
