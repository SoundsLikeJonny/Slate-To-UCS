-- Shared helper for the Slate-To-UCS ReaScripts.
--
-- Every script goes through this module so there is exactly one code path per action:
-- the extension's API_Slate_* exports when the extension is loaded, and
-- `slate` on the command line when it is not. A script never branches on which is
-- present; it calls a function here and gets the same shape back either way.
--
-- The module takes its dependencies as a table so it can be tested without REAPER.
-- `M.default()` builds that table from the real globals.

local M = {}

-- The contract version these scripts speak. It must equal the integer the engine
-- reports on /health, or the scripts refuse to talk to it.
M.SCHEMA_VERSION = 1

-- The default number of alternatives to ask for.
M.DEFAULT_TOP_K = 3

-- The value JSON's `null` decodes to.
--
-- A sentinel rather than nil, because a nil in a Lua array truncates it: `[1,null,3]`
-- would silently become two elements and every index after the null would shift.
M.NULL = setmetatable({}, { __tostring = function() return "null" end })

-- ---------------------------------------------------------------- JSON

-- Decode one JSON document. Returns the value, or nil and a message naming the byte.
--
-- Written out rather than pulled from a library because ReaScript ships no JSON module
-- and a script must not depend on luarocks being installed on the recordist's machine.
function M.decode_json(text)
  if type(text) ~= "string" then
    return nil, "expected a string, got " .. type(text)
  end

  local pos = 1
  local length = #text

  local function fail(message)
    error({ json = message, at = pos }, 0)
  end

  local function skip_space()
    while pos <= length do
      local byte = text:sub(pos, pos)
      if byte == " " or byte == "\t" or byte == "\n" or byte == "\r" then
        pos = pos + 1
      else
        return
      end
    end
  end

  local parse_value

  local function parse_string()
    if text:sub(pos, pos) ~= '"' then
      fail("expected a string")
    end
    pos = pos + 1
    local parts = {}
    while true do
      if pos > length then
        fail("unterminated string")
      end
      local byte = text:sub(pos, pos)
      if byte == '"' then
        pos = pos + 1
        return table.concat(parts)
      elseif byte == "\\" then
        local escape = text:sub(pos + 1, pos + 1)
        local simple = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f",
                         n = "\n", r = "\r", t = "\t" }
        if simple[escape] then
          parts[#parts + 1] = simple[escape]
          pos = pos + 2
        elseif escape == "u" then
          local hex = text:sub(pos + 2, pos + 5)
          if not hex:match("^%x%x%x%x$") then
            fail("malformed \\u escape")
          end
          local code = tonumber(hex, 16)
          -- Surrogate pairs are decoded so an emoji in a description survives.
          if code >= 0xD800 and code <= 0xDBFF and text:sub(pos + 6, pos + 7) == "\\u" then
            local low = tonumber(text:sub(pos + 8, pos + 11), 16)
            if low and low >= 0xDC00 and low <= 0xDFFF then
              code = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00)
              pos = pos + 6
            end
          end
          parts[#parts + 1] = utf8.char(code)
          pos = pos + 6
        else
          fail("unknown escape \\" .. escape)
        end
      else
        parts[#parts + 1] = byte
        pos = pos + 1
      end
    end
  end

  local function parse_number()
    local start = pos
    if text:sub(pos, pos) == "-" then
      pos = pos + 1
    end
    while pos <= length and text:sub(pos, pos):match("%d") do
      pos = pos + 1
    end
    if text:sub(pos, pos) == "." then
      pos = pos + 1
      local digits = 0
      while pos <= length and text:sub(pos, pos):match("%d") do
        pos = pos + 1
        digits = digits + 1
      end
      if digits == 0 then
        fail("a decimal point needs at least one digit after it")
      end
    end
    local exponent = text:sub(pos, pos)
    if exponent == "e" or exponent == "E" then
      pos = pos + 1
      local sign = text:sub(pos, pos)
      if sign == "+" or sign == "-" then
        pos = pos + 1
      end
      while pos <= length and text:sub(pos, pos):match("%d") do
        pos = pos + 1
      end
    end
    local literal = text:sub(start, pos - 1)
    local value = tonumber(literal)
    if value == nil then
      fail("malformed number " .. literal)
    end
    return value
  end

  local function parse_array()
    pos = pos + 1
    local out = {}
    skip_space()
    if text:sub(pos, pos) == "]" then
      pos = pos + 1
      return out
    end
    while true do
      out[#out + 1] = parse_value()
      skip_space()
      local byte = text:sub(pos, pos)
      if byte == "," then
        pos = pos + 1
        skip_space()
      elseif byte == "]" then
        pos = pos + 1
        return out
      else
        fail("expected , or ] in array")
      end
    end
  end

  local function parse_object()
    pos = pos + 1
    local out = {}
    skip_space()
    if text:sub(pos, pos) == "}" then
      pos = pos + 1
      return out
    end
    while true do
      skip_space()
      local key = parse_string()
      skip_space()
      if text:sub(pos, pos) ~= ":" then
        fail("expected : after a key")
      end
      pos = pos + 1
      out[key] = parse_value()
      skip_space()
      local byte = text:sub(pos, pos)
      if byte == "," then
        pos = pos + 1
      elseif byte == "}" then
        pos = pos + 1
        return out
      else
        fail("expected , or } in object")
      end
    end
  end

  parse_value = function()
    skip_space()
    local byte = text:sub(pos, pos)
    if byte == '"' then
      return parse_string()
    elseif byte == "{" then
      return parse_object()
    elseif byte == "[" then
      return parse_array()
    elseif byte == "-" or byte:match("%d") then
      return parse_number()
    elseif text:sub(pos, pos + 3) == "true" then
      pos = pos + 4
      return true
    elseif text:sub(pos, pos + 4) == "false" then
      pos = pos + 5
      return false
    elseif text:sub(pos, pos + 3) == "null" then
      pos = pos + 4
      return M.NULL
    end
    fail("unexpected byte " .. string.format("%q", byte))
  end

  local ok, value = pcall(function()
    local parsed = parse_value()
    skip_space()
    if pos <= length then
      fail("trailing data")
    end
    return parsed
  end)

  if not ok then
    if type(value) == "table" and value.json then
      return nil, value.json .. " at byte " .. tostring(value.at)
    end
    return nil, tostring(value)
  end
  return value
end

-- ---------------------------------------------------------------- validation

-- Whether a creator or source identifier is usable in a file name.
--
-- Rejects the empty string, anything with a path separator, and the characters the UCS
-- builder forbids, so a pattern built here cannot produce an unopenable path.
function M.is_valid_identifier(value)
  if type(value) ~= "string" or value == "" then
    return false
  end
  if value:find("[/\\]") then
    return false
  end
  if value:find('[%*%?"<>|:]') then
    return false
  end
  if value:match("^%s") or value:match("%s$") then
    return false
  end
  return true
end

-- ---------------------------------------------------------------- construction

-- Build an API over an explicit dependency table.
--
-- `deps.reaper` is the REAPER global table, `deps.extension` the table of API_Slate_*
-- functions when the extension is loaded (nil otherwise), and `deps.exec` a function
-- taking a command line and returning its stdout.
function M.new(deps)
  assert(type(deps) == "table", "slate_api.new needs a dependency table")
  assert(type(deps.reaper) == "table", "slate_api.new needs deps.reaper")
  assert(type(deps.exec) == "function", "slate_api.new needs deps.exec")

  local api = {}
  local reaper = deps.reaper
  local extension = deps.extension

  -- Whether the extension is loaded, which decides the code path for every action.
  function api.has_extension()
    return type(extension) == "table" and type(extension.health) == "function"
  end

  -- The engine's health: 1 warm, 0 cold, -1 unreachable.
  function api.health()
    if api.has_extension() then
      return extension.health()
    end
    local ok, code = pcall(deps.exec, "slate doctor --json")
    if not ok or type(code) ~= "string" then
      return -1
    end
    local decoded = M.decode_json(code)
    if type(decoded) ~= "table" then
      return -1
    end
    if decoded.warm == true then
      return 1
    end
    return decoded.reachable == true and 0 or -1
  end

  -- Predict one description. Returns a prediction table, or nil and a message.
  function api.predict(description, top_k)
    if type(description) ~= "string" or description == "" then
      return nil, "a description is required"
    end
    local limit = top_k or M.DEFAULT_TOP_K
    if type(limit) ~= "number" or limit < 1 then
      return nil, "top_k must be a positive number"
    end

    if api.has_extension() then
      local ok, result = pcall(extension.predict, description, limit)
      if not ok then
        return nil, tostring(result)
      end
      if type(result) ~= "table" then
        return nil, "the extension returned no prediction"
      end
      return result
    end

    local ok, stdout = pcall(deps.exec, "slate predict --json --top-k " .. limit ..
                               " " .. string.format("%q", description))
    if not ok or type(stdout) ~= "string" then
      return nil, "slate predict failed"
    end
    local decoded, err = M.decode_json(stdout)
    if type(decoded) ~= "table" then
      return nil, err or "slate predict returned no JSON"
    end
    if #decoded == 0 then
      return nil, "slate predict returned no rows"
    end
    return decoded[1]
  end

  -- The UCS render pattern for a creator and source pair.
  --
  -- `$item` is the wildcard REAPER substitutes per rendered file, and the item name is
  -- already the UCS file name once the tag action has run, so the pattern stays correct
  -- for a batch render without naming any single item.
  function api.render_pattern(creator, source)
    if not M.is_valid_identifier(creator) then
      return nil, "creator must be a non-empty name without path separators"
    end
    if not M.is_valid_identifier(source) then
      return nil, "source must be a non-empty name without path separators"
    end
    return "$item_" .. creator .. "_" .. source
  end

  -- The notes line for a prediction, in the same shape the extension writes.
  function api.notes_line(prediction)
    if type(prediction) ~= "table" then
      return nil, "a prediction is required"
    end
    local cat_id = prediction.cat_id
    if type(cat_id) ~= "string" or cat_id == "" then
      return nil, "the prediction has no cat_id"
    end
    local confidence = prediction.confidence or 0
    local filename = prediction.filename or ""
    return string.format("slate: %s %.3f %s", cat_id, confidence, filename)
  end

  -- Merge a slate line into existing notes, preserving every other line.
  --
  -- Mirrors merge_notes in the extension so the fallback path writes identical notes.
  function api.merge_notes(existing, slate_line)
    if type(slate_line) ~= "string" or slate_line == "" then
      return nil, "a slate line is required"
    end
    if type(existing) ~= "string" or existing == "" then
      return slate_line
    end

    local lines = {}
    for line in (existing .. "\n"):gmatch("([^\n]*)\n") do
      lines[#lines + 1] = line
    end
    -- gmatch yields a trailing empty field for a trailing newline; drop it.
    if lines[#lines] == "" then
      lines[#lines] = nil
    end

    local replaced = false
    for index, line in ipairs(lines) do
      if not replaced and line:sub(1, 6) == "slate:" then
        lines[index] = slate_line
        replaced = true
      end
    end
    if not replaced then
      lines[#lines + 1] = slate_line
    end
    return table.concat(lines, "\n")
  end

  -- Tag one media item with a prediction. Returns true, or false and a message.
  function api.tag_item(item, prediction)
    if type(item) ~= "number" or item == 0 then
      return false, "a media item is required"
    end
    local slate_line, line_err = api.notes_line(prediction)
    if not slate_line then
      return false, line_err
    end

    local _, notes = reaper.GetSetMediaItemInfo_String(item, "P_NOTES", "", false)
    local merged, merge_err = api.merge_notes(notes or "", slate_line)
    if not merged then
      return false, merge_err
    end
    reaper.GetSetMediaItemInfo_String(item, "P_NOTES", merged, true)

    local take = reaper.GetActiveTake(item)
    if take and prediction.filename and prediction.filename ~= "" then
      reaper.GetSetMediaItemTakeInfo_String(take, "P_NAME", prediction.filename, true)
    end
    return true
  end

  -- Tag a list of items in one undo block, so a batch is a single Ctrl+Z.
  --
  -- `on_progress(done, total)` is called after each item and may return false to cancel.
  function api.tag_items(items, predict_for, on_progress)
    if type(items) ~= "table" then
      return 0, "a list of items is required"
    end
    if type(predict_for) ~= "function" then
      return 0, "a description function is required"
    end

    reaper.Undo_BeginBlock()
    local tagged = 0
    local cancelled = false
    for index, item in ipairs(items) do
      local description = predict_for(item)
      if description and description ~= "" then
        local prediction = api.predict(description)
        if prediction then
          local ok = api.tag_item(item, prediction)
          if ok then
            tagged = tagged + 1
          end
        end
      end
      if on_progress and on_progress(index, #items) == false then
        cancelled = true
        break
      end
    end
    reaper.Undo_EndBlock("Slate-To-UCS: tag items", -1)

    if cancelled then
      return tagged, "cancelled"
    end
    return tagged
  end

  -- Launch the desktop app.
  function api.open_app(path)
    if type(path) ~= "string" or path == "" then
      return false, "a path is required"
    end
    local ok = pcall(reaper.ExecProcess, path, 0)
    if not ok then
      return false, "could not launch " .. path
    end
    return true
  end

  return api
end

-- Build an API over the real REAPER globals.
function M.default()
  local extension = nil
  if type(API_Slate_Health) == "function" then
    extension = {
      health = API_Slate_Health,
      last_drain_ms = API_Slate_LastDrainMs,
      schema_version = API_Slate_SchemaVersion,
      predict = API_Slate_Predict,
    }
  end

  local function exec(command)
    local handle = io.popen(command)
    if not handle then
      return nil
    end
    local output = handle:read("*a")
    handle:close()
    return output
  end

  return M.new({ reaper = reaper, extension = extension, exec = exec })
end

return M
