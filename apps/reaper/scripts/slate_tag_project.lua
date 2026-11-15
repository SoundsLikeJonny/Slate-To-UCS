-- Slate-To-UCS: tag every media item in the project.
--
-- Shows a progress window that can be cancelled with Escape. A cancelled run keeps the
-- items it already tagged, in one undo block, so cancelling is not the same as undoing.

local function script_directory()
  local source = debug.getinfo(1, "S").source
  local path = source:match("^@(.*)$") or source
  return path:match("^(.*)[/\\][^/\\]*$") or "."
end

local slate_api = dofile(script_directory() .. "/lib/slate_api.lua")
local api = slate_api.default()

local function project_items()
  local items = {}
  for track = 0, reaper.CountTracks(0) - 1 do
    local media_track = reaper.GetTrack(0, track)
    for index = 0, reaper.CountTrackMediaItems(media_track) - 1 do
      items[#items + 1] = reaper.GetTrackMediaItem(media_track, index)
    end
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

-- A small gfx window rather than a message box: a modal dialog would block the main
-- thread, which is the one thing this extension must never do.
local function progress_window(total)
  gfx.init("Slate-To-UCS: tagging", 340, 84, 0, 140, 140)
  local state = { cancelled = false }

  function state.step(done)
    gfx.set(0.10, 0.10, 0.12, 1.0)
    gfx.rect(0, 0, gfx.w, gfx.h, true)
    gfx.set(0.92, 0.92, 0.92, 1.0)
    gfx.x, gfx.y = 12, 16
    gfx.drawstr(string.format("Tagging %d of %d", done, total))
    gfx.set(0.62, 0.62, 0.66, 1.0)
    gfx.x, gfx.y = 12, 42
    gfx.drawstr("Press Escape to cancel")
    gfx.update()
    if gfx.getchar() == 27 then
      state.cancelled = true
    end
    return not state.cancelled
  end

  function state.close()
    gfx.quit()
  end

  return state
end

local items = project_items()
if #items == 0 then
  reaper.ShowMessageBox("This project has no media items.", "Slate-To-UCS", 0)
  return
end

local progress = progress_window(#items)
reaper.PreventUIRefresh(1)
local tagged, message = api.tag_items(items, description_for, function(done)
  return progress.step(done)
end)
reaper.PreventUIRefresh(-1)
progress.close()
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
