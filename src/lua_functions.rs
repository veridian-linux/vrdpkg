use mlua::{Error as LuaError, Lua, Result as LuaResult, Value};
use std::{fs, path::{Path, PathBuf}, sync::Arc};
use serde_json::Value as JsonValue;
use regex::Regex;

use crate::file_operations::{copy_dir_all, download_file_blocking, sha256sum_file, extract_tarball};
use crate::path_utils::{sanitize_path, validate_absolute_path};

/// Convert JSON value to Lua value
pub fn json_to_lua_table<'lua>(lua: &'lua Lua, value: &JsonValue) -> LuaResult<Value> {
    match value {
        JsonValue::Object(map) => {
            let table = lua.create_table()?;
            for (key, val) in map {
                table.set(key.as_str(), json_to_lua_table(lua, val)?)?;
            }
            Ok(Value::Table(table))
        }
        JsonValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, val) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_table(lua, val)?)?;
            }
            Ok(Value::Table(table))
        }
        JsonValue::String(s) => Ok(Value::String(lua.create_string(s)?)),
        JsonValue::Number(n) => {
            if let Some(int) = n.as_i64() {
                Ok(Value::Integer(int))
            } else if let Some(float) = n.as_f64() {
                Ok(Value::Number(float))
            } else {
                Ok(Value::Nil) // Fallback case (shouldn't really happen)
            }
        }
        JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
        JsonValue::Null => Ok(Value::Nil),
    }
}


/// Regex match function for Lua
pub fn regex_match<'lua>(_: &'lua Lua, (text, pattern): (String, String)) -> LuaResult<(Option<String>, Option<String>, Option<String>, Option<String>)> {
    let re = Regex::new(&pattern).map_err(|e| LuaError::RuntimeError(e.to_string()))?;

    if let Some(caps) = re.captures(&text) {
        let major = caps.get(1).map(|m| m.as_str().to_string());
        let minor = caps.get(2).map(|m| m.as_str().to_string());
        let patch = caps.get(3).map(|m| m.as_str().to_string());
        let revision = caps.get(4).map(|m| m.as_str().to_string());

        Ok((major, minor, patch, revision))
    } else {
        Ok((None, None, None, None))
    }
}

/// Register all Lua functions
pub fn register_lua_functions(lua: &Lua, src_dir: PathBuf, pkg_dir: PathBuf) -> LuaResult<()> {
    let globals = lua.globals();

    // Set global constants
    globals.set("ARCH", std::env::consts::ARCH)?;
    globals.set("SRC_DIR", src_dir.clone())?;
    globals.set("PKG_DIR", pkg_dir.clone())?;

    // Register download function (only downloads to src_dir)
    let download_src_dir = src_dir.clone();
    let download_function = lua.create_function(move |_, (url, dest): (String, String)| {
        match download_file_blocking(&url, &download_src_dir, &dest) {
            Ok(_) => Ok(()),
            Err(e) => Err(LuaError::RuntimeError(format!("Download error: {}", e))),
        }
    })?;
    globals.set("download", download_function)?;

    // Register JSON decode function
    let json_decode_function = lua.create_function(|lua, json_str: String| {
        let json_value: JsonValue = serde_json::from_str(&json_str)
            .map_err(|e| LuaError::ExternalError(Arc::new(e)))?;
        json_to_lua_table(lua, &json_value)
    })?;
    globals.set("json_decode", json_decode_function)?;

    // Register file_load function (only reads from src_dir)
    let file_load_src_dir = src_dir.clone();
    let file_load_function = lua.create_function(move |_, path: String| {
        match sanitize_path(&file_load_src_dir, &path) {
            Ok(abs_path) => {
                fs::read_to_string(&abs_path)
                    .map_err(|e| LuaError::ExternalError(Arc::new(e)))
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("file_load", file_load_function)?;

    // Register file_save function (only writes to src_dir)
    let file_save_src_dir = src_dir.clone();
    let file_save_function = lua.create_function(move |_, (path, content): (String, String)| {
        match sanitize_path(&file_save_src_dir, &path) {
            Ok(abs_path) => {
                // Ensure parent directory exists
                if let Some(parent) = abs_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| LuaError::ExternalError(Arc::new(e)))?;
                }
                
                fs::write(&abs_path, content)
                    .map_err(|e| LuaError::ExternalError(Arc::new(e)))?;
                Ok(())
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("file_save", file_save_function)?;

    // Register regex_match function
    let regex_match_function = lua.create_function(regex_match)?;
    globals.set("regex_match", regex_match_function)?;

    // Register sha256sum_file function (only works on src_dir)
    let sha256_src_dir = src_dir.clone();
    let sha256sum_file_function = lua.create_function(move |_, path: String| {
        match sanitize_path(&sha256_src_dir, &path) {
            Ok(abs_path) => {
                match sha256sum_file(&abs_path) {
                    Ok(hash) => Ok(hash),
                    Err(e) => Err(LuaError::ExternalError(Arc::new(e))),
                }
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("sha256sum_file", sha256sum_file_function)?;

    // Register unpack_tarball function (works within src_dir)
    let unpack_src_dir = src_dir.clone();
    let unpack_tarball_function = lua.create_function(move |_, (path, dest): (String, String)| {
        match sanitize_path(&unpack_src_dir, &path) {
            Ok(abs_path) => {
                match validate_absolute_path(Path::new(&dest)) {
                    Ok(abs_dest) => {
                        match extract_tarball(&abs_path, &abs_dest) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(LuaError::ExternalError(Arc::new(e))),
                        }
                    },
                    Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
                }
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("unpack_tarball", unpack_tarball_function)?;

    // Register copy function (works for both files and directories, within src_dir to pkg_dir)
    let copy_src_dir = src_dir.clone();
    let copy_pkg_dir = pkg_dir.clone(); // Add this line to clone pkg_dir
    let copy_function = lua.create_function(move |_, (src, dest): (String, String)| {
        match sanitize_path(&copy_src_dir, &src) {
            Ok(abs_src) => {
                // Use pkg_dir as base for destination path
                match sanitize_path(&copy_pkg_dir, &dest) {
                    Ok(abs_dest) => {
                        // Check if source is a file or directory and use appropriate copy function
                        if abs_src.is_file() {
                            // Create parent directories if they don't exist
                            if let Some(parent) = abs_dest.parent() {
                                if !parent.exists() {
                                    if let Err(e) = std::fs::create_dir_all(parent) {
                                        return Err(LuaError::ExternalError(Arc::new(e)));
                                    }
                                }
                            }
                            
                            println!("Copying file {:?} to {:?}", abs_src, &abs_dest);

                            match std::fs::copy(&abs_src, &abs_dest) {
                                Ok(_) => Ok(()),
                                Err(e) => Err(LuaError::ExternalError(Arc::new(e))),
                            }
                        } else if abs_src.is_dir() {
                            println!("Copying directory {:?} to {:?}", abs_src, &abs_dest);

                            match copy_dir_all(&abs_src, &abs_dest) {
                                Ok(_) => Ok(()),
                                Err(e) => Err(LuaError::ExternalError(Arc::new(e))),
                            }
                        } else {
                            Err(LuaError::RuntimeError(format!("Source path is neither a file nor directory: {:?}", abs_src)))
                        }
                    },
                    Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
                }
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("copy", copy_function)?;

    // Register the link function (src is absolute path destination, dest is within pkg_dir as the symlink)
    let link_pkg_dir = pkg_dir.clone();
    let link_function = lua.create_function(move |_, (target, link_path): (String, String)| {
        // Validate that the target path exists
        match validate_absolute_path(Path::new(&target)) {
            Ok(abs_target) => {
                // Sanitize the link_path to be within pkg_dir
                match sanitize_path(&link_pkg_dir, &link_path) {
                    Ok(abs_link) => {
                        // Create parent directories for the symlink if they don't exist
                        if let Some(parent) = abs_link.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                return Err(LuaError::ExternalError(Arc::new(e)));
                            }
                        }
                        
                        println!("Creating symlink at {:?} pointing to {:?}", abs_link, abs_target);
                        
                        // Create the symlink: first param is target (where it points to), second is link (where symlink is created)
                        match std::os::unix::fs::symlink(&abs_target, &abs_link) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(LuaError::ExternalError(Arc::new(e))),
                        }
                    },
                    Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
                }
            },
            Err(e) => Err(LuaError::RuntimeError(format!("Path error: {}", e))),
        }
    })?;
    globals.set("link", link_function)?;

    Ok(())
}