use std::path::{Path, PathBuf};
use thiserror::Error;
use path_clean::PathClean;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path traversal attack detected")]
    PathTraversal,
    #[error("Path not contained in target directory")]
    NotInTargetDir,
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Ensures a path resolves within a target directory to prevent path traversal attacks
pub fn sanitize_path(base_dir: &Path, relative_path: &str) -> Result<PathBuf, PathError> {
    // Convert string to path and clean it (resolve ".." and ".")
    let rel_path = PathBuf::from(relative_path).clean();
    
    // Check if the path has components that could lead to path traversal
    if rel_path.components().any(|c| c.as_os_str() == "..") {
        return Err(PathError::PathTraversal);
    }
    
    // Strip leading slash if present
    let rel_path = if let Ok(stripped) = rel_path.strip_prefix("/") {
        stripped.to_path_buf()
    } else {
        rel_path
    };
    
    // Create the absolute path from the base directory
    let abs_path = base_dir.join(&rel_path);
    
    // Verify the path is still within the base directory
    if !abs_path.starts_with(base_dir) {
        return Err(PathError::NotInTargetDir);
    }
    
    Ok(abs_path)
}

/// Ensures the path is absolute and exists
pub fn validate_absolute_path(path: &Path) -> Result<PathBuf, PathError> {
    if !path.is_absolute() {
        return Err(PathError::InvalidPath("Path must be absolute".to_string()));
    }
    
    if !path.exists() {
        return Err(PathError::InvalidPath(format!("Path does not exist: {:?}", path)));
    }
    
    Ok(path.to_path_buf())
}
