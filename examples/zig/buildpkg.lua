INFO = {
  name = "zig",
  description = "A general-purpose programming language and toolchain for maintaining robust, optimal, and reusable software",
  url = "https://ziglang.org",
  maintainers = {"Iris Junckes <me@junckes.dev>"},
  license = "MIT",
  dev = true,
  provides = {"zig"},
  arch = {"x86_64", "aarch64"},
}

ZIG_VERSION_FILE_URL="https://ziglang.org/download/index.json"

FULL_VERSION = ""

local function parse_version(version)
  local major, minor, patch, revision = regex_match(version, "^([0-9]+)\\.([0-9]+)\\.([0-9]+)(?:-dev\\.([0-9]+)\\+(?:[A-z]|[0-9])+)?$")
  FULL_VERSION = version
  return major .. "." .. minor .. "." .. patch .. "-" .. revision
end

ZIG_INDEX = {}

function VERSION()
  download(ZIG_VERSION_FILE_URL, "/index.json")
  ZIG_INDEX = json_decode(file_load("/index.json"))

  return parse_version(ZIG_INDEX.master.version)
end

function PREPARE()
  local op = nil

  if ARCH == "x86_64" then
    op = ZIG_INDEX.master["x86_64-linux"]
  elseif ARCH == "aarch64" then
    op = ZIG_INDEX.master["aarch64-linux"]
  else
    error("Unsupported architecture: " .. ARCH)
  end

  download(op.tarball, "/zig.tar.xz")

  if sha256sum_file("/zig.tar.xz") ~= op.shasum then
    error("SHA256 sum mismatch")
  end

  unpack_tarball("/zig.tar.xz", SRC_DIR)
end

function PACKAGE()
  local zig_dir = "/zig-linux-" .. ARCH .. "-" .. FULL_VERSION

  copy(zig_dir .. "/zig", "/usr/lib/zig/zig")
  link("/usr/lib/zig/zig", "/usr/bin/zig")
  copy(zig_dir .. "/lib", "/usr/lib/zig/lib")
  copy(zig_dir .. "/LICENSE", "/usr/share/licenses/zig/LICENSE")
end
