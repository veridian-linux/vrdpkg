INFO = {
  name = "vrdpkg",
  description = "A package manager for Veridian",
  url = "https://github.com/veridian-linux/vrdpkg",
  maintainers = {"Iris Junckes <me@junckes.dev>"},
  license = "MIT",
  dev = true,
  provides = {"vrdpkg"},
  arch = {"x86_64", "aarch64"},
}

--- Download sources
--- 
--- @return nil
function SOURCES()
  local repo = git.clone(INFO.url .. ".git", "/vrdpkg")
  print(repo.path)
end

--- Returns the version of the package in the format "major.minor.patch-revision"
--- 
--- @return string
function VERSION()
  local src = git.load("/vrdpkg")
  local tag = src:get_tags()[0]

  if tag == nil then
    error("No tags found")
  end

  local revision = src:get_revision(tag)

  return tag .. "-" .. revision
end

--- Prepare the sources
--- 
--- @return nil
function PREPARE()

end

--- Build the package
--- 
--- @return nil
function BUILD()

end

--- Package the built files
--- 
--- @return nil
function PACKAGE()

end
