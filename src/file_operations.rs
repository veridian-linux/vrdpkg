use std::{fs, io, path::{Path, PathBuf}};
use sha2::Digest;
use tar::Archive;
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;
use xz2::read::XzDecoder;
use zstd::stream::Decoder as ZstdDecoder;

use crate::path_utils::sanitize_path;

// Function to detect compression type and extract tarball
pub fn extract_tarball<P: AsRef<Path>, Q: AsRef<Path>>(src_path: P, dest_path: Q) -> io::Result<()> {
    let file = fs::File::open(&src_path)?;
    let path_str = src_path.as_ref().to_string_lossy();
    
    // Create destination directory if it doesn't exist
    fs::create_dir_all(&dest_path)?;
    
    // Detect compression format based on extension and extract accordingly
    if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
        let mut tar = Archive::new(GzDecoder::new(file));
        tar.unpack(&dest_path)
    } else if path_str.ends_with(".tar.bz2") || path_str.ends_with(".tbz2") {
        let mut tar = Archive::new(BzDecoder::new(file));
        tar.unpack(&dest_path)
    } else if path_str.ends_with(".tar.xz") || path_str.ends_with(".txz") {
        let mut tar = Archive::new(XzDecoder::new(file));
        tar.unpack(&dest_path)
    } else if path_str.ends_with(".tar.zst") || path_str.ends_with(".tzst") {
        let decoder = ZstdDecoder::new(file)?;
        let mut tar = Archive::new(decoder);
        tar.unpack(&dest_path)
    } else if path_str.ends_with(".tar") {
        // No compression
        let mut tar = Archive::new(file);
        tar.unpack(&dest_path)
    } else {
        // Try to detect by content if extension isn't recognized
        // This is more complex and would require reading magic bytes
        // For simplicity, let's fall back to treating it as uncompressed
        let mut tar = Archive::new(file);
        tar.unpack(&dest_path)
    }
}

// Function to create a gzipped tarball
fn create_gzip_tarball<P: AsRef<Path>, Q: AsRef<Path>>(src_path: P, dest_path: Q) -> io::Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::ffi::OsStr;
    
    let dest_file = fs::File::create(&dest_path)?;
    let gz_encoder = GzEncoder::new(dest_file, Compression::default());
    let mut tar_builder = tar::Builder::new(gz_encoder);
    
    let src_path = src_path.as_ref();
    
    if src_path.is_dir() {
        // For directories, add all contents
        let base_path = src_path;
        for entry in walkdir::WalkDir::new(src_path) {
            let entry = entry?;
            let path = entry.path();
            
            if path == base_path {
                continue; // Skip the root directory itself
            }
            
            let relative_path = path.strip_prefix(base_path)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                
            if path.is_file() {
                tar_builder.append_file(relative_path, &mut fs::File::open(path)?)?;
            } else if path.is_dir() {
                tar_builder.append_dir(relative_path, path)?;
            }
        }
    } else if src_path.is_file() {
        // For a single file, just add that file
        let file_name = src_path.file_name().unwrap_or(OsStr::new("file"));
        tar_builder.append_file(file_name, &mut fs::File::open(src_path)?)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Source path does not exist or is neither a file nor directory",
        ));
    }
    
    // Finish writing archive
    tar_builder.into_inner()?.finish()?;
    
    Ok(())
}

/// Copy a directory recursively
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Download a file to a specific directory
pub async fn download_file(url: &str, dest_dir: &Path, filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest_dir)?;

    // Normalize the filename (remove any path traversal)
    let dest_path = match sanitize_path(dest_dir, filename) {
        Ok(path) => path,
        Err(e) => return Err(Box::new(e)),
    };

    println!("Downloading {} to {:?}", url, dest_path);

    // Download the file
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;
    
    // Save the file
    fs::write(&dest_path, content)?;
    
    Ok(dest_path)
}

/// Download a file to a specific directory (blocking version)
pub fn download_file_blocking(url: &str, dest_dir: &Path, filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest_dir)?;

    // Normalize the filename (remove any path traversal)
    let dest_path = match sanitize_path(dest_dir, filename) {
        Ok(path) => path,
        Err(e) => return Err(Box::new(e)),
    };

    println!("Downloading {} to {:?}", url, dest_path);

    // Download the file
    let mut response = reqwest::blocking::get(url)?;
    let mut file = fs::File::create(&dest_path)?;
    response.copy_to(&mut file)?;
    
    Ok(dest_path)
}

/// Calculate SHA256 hash of a file
pub fn sha256sum_file(path: impl AsRef<Path>) -> Result<String, io::Error> {
    let content = fs::read(path)?;
    let hash = sha2::Sha256::digest(&content);
    Ok(format!("{:x}", hash))
}
