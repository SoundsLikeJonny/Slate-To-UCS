-- Slate-To-UCS: set the project render filename pattern from the UCS fields.
--
-- `$item` is the wildcard REAPER substitutes per rendered file, and the tag action has
-- already made each item name a UCS file name, so one pattern stays correct for a batch
-- render without naming any single item.

local function script_directory()
  local source = debug.getinfo(1, "S").source
  local path = source:match("^@(.*)$") or source
  return path:match("^(.*)[/\\][^/\\]*$") or "."
end

local slate_api = dofile(script_directory() .. "/lib/slate_api.lua")
local api = slate_api.default()

local ok, creator, source = reaper.GetUserInputs(
  "Slate-To-UCS render pattern", 2, "Creator,Source,extrawidth=120", "NONE,NONE")
if not ok then
  return
end

local pattern, message = api.render_pattern(creator, source)
if not pattern then
  reaper.ShowMessageBox(message, "Slate-To-UCS", 0)
  return
end

reaper.GetSetProjectInfo_String(0, "RENDER_PATTERN", pattern, true)
reaper.ShowMessageBox("Render pattern set to:\n\n" .. pattern, "Slate-To-UCS", 0)
