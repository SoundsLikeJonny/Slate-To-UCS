-- Slate-To-UCS: open the desktop app.
--
-- The app is the review surface; the extension never opens a window of its own. The path
-- is read from the SLATE_APP environment variable first, then from the install location
-- the installer writes, so a portable install works without editing the script.

local function script_directory()
  local source = debug.getinfo(1, "S").source
  local path = source:match("^@(.*)$") or source
  return path:match("^(.*)[/\\][^/\\]*$") or "."
end

local slate_api = dofile(script_directory() .. "/lib/slate_api.lua")
local api = slate_api.default()

local function app_path()
  local from_env = os.getenv("SLATE_APP")
  if from_env and from_env ~= "" then
    return from_env
  end
  local resource = reaper.GetResourcePath()
  local candidates = {
    resource .. "/Slate-To-UCS/Slate-To-UCS.exe",
    resource .. "/Slate-To-UCS/slate-to-ucs",
    resource .. "/Slate-To-UCS/Slate-To-UCS.app/Contents/MacOS/Slate-To-UCS",
  }
  for _, candidate in ipairs(candidates) do
    local handle = io.open(candidate, "rb")
    if handle then
      handle:close()
      return candidate
    end
  end
  return nil
end

local path = app_path()
if not path then
  reaper.ShowMessageBox(
    "Slate-To-UCS was not found.\n\nSet SLATE_APP to its full path, or install it from the app's "
      .. "Install REAPER integration action.",
    "Slate-To-UCS", 0)
  return
end

local ok, message = api.open_app(path)
if not ok then
  reaper.ShowMessageBox(message, "Slate-To-UCS", 0)
end
