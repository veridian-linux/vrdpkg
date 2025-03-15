---@meta

--- Download a file from a URL to a local file.
--- Returns the path to the downloaded file.
--- 
--- @param source string
--- @param destination string
--- @return string
function download(source, destination) end

--- Reads a file into a string.
---
--- @nodiscard
--- @param file string
--- @return string
function file_load(file) end

--- Write a string to a file.
--- 
--- @param file string
--- @param content string
--- @return nil
function file_save(file, content) end

--- Returns all containing matches of the pattern in the string.
--- 
--- @nodiscard
--- @param to_match string
--- @param pattern string
--- @return string[]
function regex_match(to_match, pattern) end

--- Decode a JSON string into a Lua table.
--- 
--- @nodiscard
--- @param json string
--- @return table
function json_decode(json) end

--- Calculate the SHA256 sum of a file.
--- 
--- @nodiscard
--- @param file string
--- @return string
function sha256sum_file(file) end

--- Calculate the SHA256 sum of a string.
--- 
--- @nodiscard
--- @param str string
--- @return string
function sha256sum_string(str) end

--- Unpack a tarball.
--- 
--- @param tarball string
--- @param dest string
--- @return nil
function unpack_tarball(tarball, dest) end

--- Copy a file or directory.
--- 
--- @param source string
--- @param destination string
--- @return nil
function copy(source, destination) end

--- Create a symbolic link.
--- 
--- @param source string
--- @param destination string
--- @return nil
function link(source, destination) end

--- @diagnostic disable-next-line: doc-field-no-class
--- @field arch string
--- Current architecture. (e.g. "x86_64", "aarch64")
ARCH = ""

--- @diagnostic disable-next-line: doc-field-no-class
--- @field SRC_DIR string
--- Source directory.
SRC_DIR = ""

--- @diagnostic disable-next-line: doc-field-no-class
--- @field PKG_DIR string
--- Package directory.
PKG_DIR = ""

--- @class git_repo
--- @field path string
git_repo = {
    --- Get the tag of the repository.
    --- 
    --- @return string
    get_tag = function() end,

    --- Get the revision of the repository.
    --- 
    --- @return string
    get_revision = function() end
}

--- @class git
git = {
    --- Clone a git repository.
    --- 
    --- @param url string
    --- @param destination string?
    --- @return nil
    clone = function(url, destination) end,

    --- Load a git repository.
    --- 
    --- @param path string
    --- @return git_repo 
    load = function(path) end
}


