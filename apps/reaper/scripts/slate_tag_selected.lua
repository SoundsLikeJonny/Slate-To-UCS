-- Slate-To-UCS: tag the selected media items.
--
-- One undo block for the whole selection, so a 20-item tag is a single Ctrl+Z. The
-- description for each item is its take name, which is what a recordist already has on
-- the item after a slate pass.
--
-- Works with the extension loaded and without it: slate_api picks the code path.

local function script_directory()
  local source = debug.getinfo(1, "S").source
  local path = source:match("^@(.*)$") or source
  return path:match("^(.*)[/\\][^/\\]*$") or "."
end

local slate_api = dofile(script_directory() .. "/lib/slate_api.lua")
local api = slate_api.default()

local function selected_items()
  local items = {}
  local count = reaper.CountSelectedMediaItems(0)
  for index = 0, count - 1 do
    items[#items + 1] = reaper.GetSelectedMediaItem(0, index)
  end
  return items
end

local function description_for(item)
  local take = reaper.GetActiveTake(item)
  if not take then
    return nil
  end
  local _, name = reaper.GetSetMediaItemTakeInfo_String(take, "P_NAME", "", false)
  return name
end

local items = selected_items()
if #items == 0 then
  reaper.ShowMessageBox("Select at least one media item first.", "Slate-To-UCS", 0)
  return
end

local tagged, message = api.tag_items(items, description_for)
reaper.UpdateArrange()

if message then
  reaper.ShowMessageBox(
    string.format("Tagged %d of %d items (%s).", tagged, #items, message),
    "Slate-To-UCS", 0)
else
  reaper.ShowMessageBox(
    string.format("Tagged %d of %d items.", tagged, #items),
    "Slate-To-UCS", 0)
end
