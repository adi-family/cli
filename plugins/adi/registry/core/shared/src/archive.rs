use std::io::Read;

use flate2::read::GzDecoder;

/// Extract a specific file from a tar.gz archive by filename.
pub fn extract_file_from_tar_gz(data: &[u8], target_filename: &str) -> Option<Vec<u8>> {
    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().ok()?;

    for entry in entries {
        let mut entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let filename = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_os_string()))
            .unwrap_or_default();

        if filename == target_filename {
            let mut buf = Vec::new();
            if entry.read_to_end(&mut buf).is_ok() {
                return Some(buf);
            }
        }
    }
    None
}

/// Extract multiple files from a tar.gz archive. Returns a vec of (filename, contents) pairs.
pub fn extract_files_from_tar_gz(
    data: &[u8],
    target_filenames: &[&str],
) -> Vec<(String, Vec<u8>)> {
    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let entries = match archive.entries() {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for entry in entries {
        let mut entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let filename = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_default();

        if target_filenames.contains(&filename.as_str()) {
            let mut buf = Vec::new();
            if entry.read_to_end(&mut buf).is_ok() {
                results.push((filename, buf));
            }
        }

        if results.len() == target_filenames.len() {
            break;
        }
    }
    results
}

/// Extract and parse manifest.json from a tar.gz archive.
pub fn extract_manifest(data: &[u8]) -> Option<crate::manifest::Manifest> {
    let json_bytes = extract_file_from_tar_gz(data, "manifest.json")?;
    serde_json::from_slice(&json_bytes).ok()
}
