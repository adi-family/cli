use anyhow::{bail, Context, Result};

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("ID must not be empty");
    }
    let valid = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
    if !valid {
        bail!("Invalid ID '{id}': only alphanumeric, '.', '_', '-' allowed");
    }
    Ok(())
}

pub fn validate_version(version: &str) -> Result<()> {
    semver::Version::parse(version)
        .with_context(|| format!("Invalid semver version '{version}'"))?;
    Ok(())
}
