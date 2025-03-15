use lua_functions::{register_git_object, register_lua_functions};
use mlua::{FromLuaMulti, Function, IntoLuaMulti, Lua, Table, Value};
use std::{fs, path::{Path, PathBuf}, process};
use clap::{command, value_parser, Arg};
use serde::{Deserialize, Serialize};
mod lua_functions;
mod file_operations;
mod path_utils;

#[derive(Serialize, Deserialize)]
struct PackageInfo {
    name: String,
    description: String,
    version: Option<String>,
    license: String,
    dev: bool,
    dependencies: Vec<String>,
    build_dependencies: Vec<String>,
    optional_dependencies: Vec<String>,
    conflicts: Vec<String>,
    provides: Vec<String>,
    replaces: Vec<String>,
    arch: Vec<String>,
    url: String,
    maintainers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct FinalPackageInfo {
    name: String,
    description: String,
    version: String,
    license: String,
    dev: bool,
    dependencies: Vec<String>,
    conflicts: Vec<String>,
    provides: Vec<String>,
    replaces: Vec<String>,
    arch: Vec<String>,
    url: String,
    maintainers: Vec<String>,
}

impl Into<FinalPackageInfo> for PackageInfo {
    fn into(self) -> FinalPackageInfo {
        FinalPackageInfo {
            name: self.name,
            description: self.description,
            version: self.version.unwrap_or_else(|| "0.0.0".to_string()),
            license: self.license,
            dev: self.dev,
            dependencies: self.dependencies,
            conflicts: self.conflicts,
            provides: self.provides,
            replaces: self.replaces,
            arch: self.arch,
            url: self.url,
            maintainers: self.maintainers,
        }
    }
}

fn lua_get_package_info(lua: &Lua) -> Result<PackageInfo, mlua::Error> {
    let globals = lua.globals();

    // Get the INFO table
    let info_table: Table = globals.get("INFO").map_err(|_| {
        mlua::Error::RuntimeError("INFO table missing".to_string())
    })?;

    // Required fields
    let name = info_table.get("name").map_err(|_| {
        mlua::Error::RuntimeError("name field missing".to_string())
    })?;
    
    let description = info_table.get("description").map_err(|_| {
        mlua::Error::RuntimeError("description field missing".to_string())
    })?;
    
    let url = info_table.get("url").map_err(|_| {
        mlua::Error::RuntimeError("url field missing".to_string())
    })?;
    
    let license = info_table.get("license").map_err(|_| {
        mlua::Error::RuntimeError("license field missing".to_string())
    })?;
    
    let dev = info_table.get("dev").map_err(|_| {
        mlua::Error::RuntimeError("dev field missing".to_string())
    })?;

    // Required array fields
    let provides_table: Table = info_table.get("provides").map_err(|_| {
        mlua::Error::RuntimeError("provides field missing".to_string())
    })?;
    
    let arch_table: Table = info_table.get("arch").map_err(|_| {
        mlua::Error::RuntimeError("arch field missing".to_string())
    })?;

    // Optional version field (can be None)
    let version: Option<String> = info_table.get("version").ok();
    
    // Get maintainers
    let maintainers_table: Table = info_table.get("maintainers").map_err(|_| {
        mlua::Error::RuntimeError("maintainers field missing".to_string())
    })?;

    // Optional array fields - initialize as empty arrays if missing
    let dependencies = match info_table.get::<Table>("dependencies") {
        Ok(table) => table.sequence_values::<String>().map(|v| v.unwrap_or_default()).collect(),
        Err(_) => Vec::new(),
    };

    let build_dependencies = match info_table.get::<Table>("build_dependencies") {
        Ok(table) => table.sequence_values::<String>().map(|v| v.unwrap_or_default()).collect(),
        Err(_) => Vec::new(),
    };

    let optional_dependencies = match info_table.get::<Table>("optional_dependencies") {
        Ok(table) => table.sequence_values::<String>().map(|v| v.unwrap_or_default()).collect(),
        Err(_) => Vec::new(),
    };

    let conflicts = match info_table.get::<Table>("conflicts") {
        Ok(table) => table.sequence_values::<String>().map(|v| v.unwrap_or_default()).collect(),
        Err(_) => Vec::new(),
    };

    let provides = provides_table.sequence_values::<String>()
        .map(|v| v.unwrap_or_default())
        .collect();

    let replaces = match info_table.get::<Table>("replaces") {
        Ok(table) => table.sequence_values::<String>().map(|v| v.unwrap_or_default()).collect(),
        Err(_) => Vec::new(),
    };

    let arch = arch_table.sequence_values::<String>()
        .map(|v| v.unwrap_or_default())
        .collect();

    let maintainers = maintainers_table.sequence_values::<String>()
        .map(|v| v.unwrap_or_default())
        .collect();

    Ok(PackageInfo {
        name,
        description,
        version,
        license,
        dev,
        dependencies,
        build_dependencies,
        optional_dependencies,
        conflicts,
        provides,
        replaces,
        arch,
        url,
        maintainers,
    })
}

fn main() {
    let matches = command!()
        .arg(Arg::new("project")
            .required(true)
            .help("The project to build")
            .value_parser(value_parser!(PathBuf))
        )
        .arg(Arg::new("clean")
            .short('C')
            .long("clean")
            .required(false)
            .num_args(0)
            .help("Clean the project after building")
        )
        .get_matches();

    let project = matches.get_one::<PathBuf>("project").unwrap();
    let clean_project_after = matches.contains_id("clean");

    // check if the project is either a directory containing a buildpkg.lua file or a buildpkg.lua file
    let buildpkg_lua = if project.is_dir() {
        fs::canonicalize(project.join("buildpkg.lua")).unwrap()
    } else {
        fs::canonicalize(project.clone()).unwrap()
    };

    if !buildpkg_lua.exists() {
        eprintln!("Error: buildpkg.lua not found");
        std::process::exit(1);
    }

    let working_dir = if project.is_dir() {
        fs::canonicalize(project).unwrap()
    } else {
        fs::canonicalize(project.parent().unwrap().to_path_buf()).unwrap()
    };

    let lua = Lua::new();

    if !working_dir.join("src").exists() {
        fs::create_dir(working_dir.join("src")).unwrap();
    }

    let src_dir_value = working_dir.join("src");

    if !working_dir.join("pkg").exists() {
        fs::create_dir(working_dir.join("pkg")).unwrap();
    }

    let pkg_dir_value = working_dir.join("pkg");

    register_lua_functions(&lua, src_dir_value.clone(), pkg_dir_value.clone()).unwrap();
    register_git_object(&lua, src_dir_value.clone(), pkg_dir_value.clone()).unwrap();

    let lua_code = fs::read_to_string(buildpkg_lua).unwrap();

    let chunk = lua.load(lua_code);

    chunk.exec().unwrap();

    let mut package_info = lua_get_package_info(&lua).unwrap();

    if package_info.dev {
        println!("Building package in dev mode");
    }

    let version_function_exists = lua.globals().get::<Value>("VERSION").is_ok();

    if package_info.version.is_none() && !version_function_exists {
        eprintln!("Error: version field missing and VERSION function not found");
        std::process::exit(1);
    }

    if package_info.version.is_some() && version_function_exists {
        eprintln!("Error: version field and VERSION function both found");
        std::process::exit(1);
    }

    println!("Getting sources...");
    run_function::<()>(&lua, "SOURCES", ());

    if package_info.version.is_none() {
        package_info.version = run_function(&lua, "VERSION", ());
    }

    println!("\n- {} {} ({}) maintained by {}\n", package_info.name, package_info.version.clone().unwrap(), package_info.license, package_info.maintainers.join(", "));

    if !package_info.arch.contains(&std::env::consts::ARCH.to_string()) {
        eprintln!("Error: package not available for host architecture");
        std::process::exit(1);
    }

    println!("Preparing...");
    run_function::<()>(&lua, "PREPARE", ());

    println!("Packaging...");
    run_function::<()>(&lua, "PACKAGE", ());

    let mut find_result = Vec::new();

    visit_dirs(&pkg_dir_value, &mut find_result).unwrap();

    fs::write(working_dir.join("pkg").join(".pkgfiles"), find_result.iter().map(|file| {String::from("/") + &String::from(PathBuf::from(file).strip_prefix(&pkg_dir_value).unwrap().to_str().unwrap())}).collect::<Vec<String>>().join("\n")).unwrap();

    let final_package_info: FinalPackageInfo = package_info.into();

    let final_package_info_json = serde_json::to_string(&final_package_info).unwrap();
    fs::write(working_dir.join("pkg").join("package.json"), final_package_info_json).unwrap();

    // creates a tarball of the pkg directory named after the project version like project-version-arch.tar.gz
    let tarball_name = format!("{}-{}-{}.tar.gz", final_package_info.name, final_package_info.version.clone(), std::env::consts::ARCH);
    let tarball_path = working_dir.join(tarball_name);
    let tar = std::process::Command::new("tar")
        .arg("--owner=root")
        .arg("--group=root")
        .arg("--preserve-permissions")
        .arg("-czf")
        .arg(&tarball_path)
        .arg("-C")
        .arg("pkg")
        .arg(".")
        .current_dir(working_dir.clone().to_str().unwrap())
        .output()
        .unwrap();
    if !tar.status.success() {
        eprintln!("Error: {}", String::from_utf8_lossy(&tar.stderr));
        std::process::exit(1);
    }

    if clean_project_after {
        fs::remove_dir_all(working_dir.join("src")).unwrap();
        fs::remove_dir_all(working_dir.join("pkg")).unwrap();
    }
}

fn visit_dirs(dir: &Path, paths: &mut Vec<String>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() || path.is_symlink() {
                // Get relative path from pkg_dir
                let base_path = Path::new("pkg");
                if let Some(rel_path) = path.strip_prefix(base_path.parent().unwrap()).ok() {
                    paths.push(format!("{}", rel_path.display()));
                }
            }
            
            if path.is_dir() {
                visit_dirs(&path, paths)?;
            }
        }
    }
    
    Ok(())
}

fn run_function_if_exists<R: FromLuaMulti>(lua: &Lua, function_name: &str, args: impl IntoLuaMulti) -> Option<R> {
    let globals = lua.globals();
    let function_res = globals.get(function_name);
    if let Err(_) = function_res {
        return None;
    }
    let function: Function = function_res.unwrap();

    let res = function.call::<R>(args);
    match res {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

fn run_function<R: FromLuaMulti>(lua: &Lua, function_name: &str, args: impl IntoLuaMulti) -> R {
    let globals = lua.globals();
    let function_res = globals.get(function_name);
    if let Err(_) = function_res {
        eprintln!("Function {} not found", function_name);
        process::exit(1);
    }
    let function: Function = function_res.unwrap();

    let res = function.call::<R>(args);
    match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
