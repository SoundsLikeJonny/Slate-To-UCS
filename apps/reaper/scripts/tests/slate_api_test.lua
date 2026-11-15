-- Unit tests for lib/slate_api.lua.
--
-- Plain Lua with no test framework, because ReaScript ships none and a contributor must
-- be able to run this with nothing but `lua`. Run with:
--
--   lua apps/reaper/scripts/tests/slate_api_test.lua
--
-- Exits 0 when every check passes and 1 otherwise, so CI can gate on it.

local function this_directory()
  local source = debug.getinfo(1, "S").source
  local path = source:match("^@(.*)$") or source
  return path:match("^(.*)[/\\][^/\\]*$") or "."
end

local slate_api = dofile(this_directory() .. "/../lib/slate_api.lua")

local passed, failed = 0, 0
local failures = {}

local function test(name, body)
  local ok, err = pcall(body)
  if ok then
    passed = passed + 1
  else
    failed = failed + 1
    failures[#failures + 1] = name .. ": " .. tostring(err)
  end
end

local function eq(actual, expected, what)
  if actual ~= expected then
    error(string.format("%s: expected %s, got %s", what or "value",
      tostring(expected), tostring(actual)), 2)
  end
end

local function is_nil(actual, what)
  if actual ~= nil then
    error(string.format("%s: expected nil, got %s", what or "value", tostring(actual)), 2)
  end
end

-- A REAPER stand-in that records what was written.
local function fake_reaper()
  local state = {
    notes = {},
    take_names = {},
    undo_begin = 0,
    undo_end = 0,
    undo_labels = {},
    launched = nil,
  }
  local api = {}

  function api.GetSetMediaItemInfo_String(item, key, value, set)
    if key ~= "P_NOTES" then
      return false, ""
    end
    if set then
      state.notes[item] = value
      return true, value
    end
    return true, state.notes[item] or ""
  end

  function api.GetActiveTake(item)
    return item * 10
  end

  function api.GetSetMediaItemTakeInfo_String(take, key, value, set)
    if key ~= "P_NAME" then
      return false, ""
    end
    if set then
      state.take_names[take] = value
      return true, value
    end
    return true, state.take_names[take] or ""
  end

  function api.Undo_BeginBlock()
    state.undo_begin = state.undo_begin + 1
  end

  function api.Undo_EndBlock(label, flags)
    state.undo_end = state.undo_end + 1
    state.undo_labels[#state.undo_labels + 1] = label
    state.undo_flags = flags
  end

  function api.ExecProcess(path, flags)
    state.launched = path
    state.launch_flags = flags
    return 0
  end

  return api, state
end

local function api_with(overrides)
  local reaper, state = fake_reaper()
  local deps = { reaper = reaper, extension = nil, exec = function() return "" end }
  for key, value in pairs(overrides or {}) do
    deps[key] = value
  end
  return slate_api.new(deps), state, reaper
end

local function sample_prediction()
  return {
    cat_id = "DOORCreak",
    confidence = 0.712,
    decision = "confident",
    filename = "DOORCreak_Door Creak_NONE_NONE",
    fx_name = "Door Creak",
    keywords = { "Door", "Creak" },
  }
end

-- ---------------------------------------------------------------- decode_json

test("decode_json reads an object", function()
  local value = slate_api.decode_json('{"a":1,"b":"two"}')
  eq(value.a, 1, "a")
  eq(value.b, "two", "b")
end)

test("decode_json reads a nested array of objects", function()
  local value = slate_api.decode_json('[{"cat_id":"A","confidence":0.5},{"cat_id":"B"}]')
  eq(#value, 2, "length")
  eq(value[1].cat_id, "A", "first cat_id")
  eq(value[2].cat_id, "B", "second cat_id")
end)

test("decode_json reads every scalar form", function()
  local value = slate_api.decode_json('[true,false,null,-1.5e2,0]')
  eq(value[1], true, "true")
  eq(value[2], false, "false")
  eq(value[3], slate_api.NULL, "null")
  eq(value[4], -150, "exponent")
  eq(value[5], 0, "zero")
end)

test("decode_json keeps a null from truncating an array", function()
  local value = slate_api.decode_json('[1,null,3]')
  eq(#value, 3, "length")
  eq(value[3], 3, "third element")
end)

test("decode_json decodes the escapes JSON requires", function()
  local value = slate_api.decode_json('"a\\"b\\\\c\\/d\\ne\\tf"')
  eq(value, 'a"b\\c/d\ne\tf', "escapes")
end)

test("decode_json decodes a surrogate pair", function()
  local value = slate_api.decode_json('"\\ud83d\\ude00"')
  eq(value, utf8.char(0x1F600), "emoji")
end)

test("decode_json rejects a truncated document", function()
  local value, err = slate_api.decode_json('{"a":')
  is_nil(value, "value")
  if not err then
    error("expected an error message")
  end
end)

test("decode_json rejects trailing data", function()
  local value, err = slate_api.decode_json("{} extra")
  is_nil(value, "value")
  if not err or not err:find("trailing") then
    error("expected a trailing-data message, got " .. tostring(err))
  end
end)

test("decode_json rejects an unterminated string", function()
  local value, err = slate_api.decode_json('"abc')
  is_nil(value, "value")
  if not err then
    error("expected an error message")
  end
end)

test("decode_json rejects a non-string input", function()
  local value, err = slate_api.decode_json(nil)
  is_nil(value, "value")
  if not err or not err:find("expected a string") then
    error("expected a type message, got " .. tostring(err))
  end
end)

test("decode_json rejects a malformed number", function()
  local value, err = slate_api.decode_json("[1.]")
  is_nil(value, "value")
  if not err then
    error("expected an error message")
  end
end)

test("decode_json rejects a bad unicode escape", function()
  local value, err = slate_api.decode_json('"\\uZZZZ"')
  is_nil(value, "value")
  if not err or not err:find("u escape") then
    error("expected a unicode message, got " .. tostring(err))
  end
end)

test("decode_json reads an empty object and array", function()
  eq(#slate_api.decode_json("[]"), 0, "empty array")
  local object = slate_api.decode_json("{}")
  eq(next(object), nil, "empty object")
end)

-- ---------------------------------------------------------------- identifiers

test("is_valid_identifier accepts a plain name", function()
  eq(slate_api.is_valid_identifier("Jon Evans"), true, "plain")
  eq(slate_api.is_valid_identifier("NONE"), true, "NONE")
end)

test("is_valid_identifier rejects the empty string and non-strings", function()
  eq(slate_api.is_valid_identifier(""), false, "empty")
  eq(slate_api.is_valid_identifier(nil), false, "nil")
  eq(slate_api.is_valid_identifier(7), false, "number")
end)

test("is_valid_identifier rejects path separators", function()
  eq(slate_api.is_valid_identifier("a/b"), false, "slash")
  eq(slate_api.is_valid_identifier("a\\b"), false, "backslash")
end)

test("is_valid_identifier rejects the characters UCS forbids", function()
  for _, value in ipairs({ "a*b", "a?b", 'a"b', "a<b", "a>b", "a|b", "a:b" }) do
    eq(slate_api.is_valid_identifier(value), false, value)
  end
end)

test("is_valid_identifier rejects a leading or trailing space", function()
  eq(slate_api.is_valid_identifier(" a"), false, "leading")
  eq(slate_api.is_valid_identifier("a "), false, "trailing")
end)

-- ---------------------------------------------------------------- merge_notes

test("merge_notes appends when no slate line exists", function()
  local merged = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
    .merge_notes("take 3", "slate: DOORCreak 0.712 x")
  eq(merged, "take 3\nslate: DOORCreak 0.712 x", "merged")
end)

test("merge_notes replaces the slate line in place", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local merged = api.merge_notes("first\nslate: OLD 0.1 y\nlast", "slate: NEW 0.9 z")
  eq(merged, "first\nslate: NEW 0.9 z\nlast", "merged")
end)

test("merge_notes replaces only the first slate line", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local merged = api.merge_notes("slate: A\nslate: B", "slate: C")
  eq(merged, "slate: C\nslate: B", "merged")
end)

test("merge_notes returns the line for empty notes", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  eq(api.merge_notes("", "slate: A"), "slate: A", "empty")
  eq(api.merge_notes(nil, "slate: A"), "slate: A", "nil")
end)

test("merge_notes refuses an empty slate line", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local merged, err = api.merge_notes("x", "")
  is_nil(merged, "merged")
  if not err then
    error("expected an error message")
  end
end)

-- ---------------------------------------------------------------- notes_line

test("notes_line formats cat_id, confidence and filename", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  eq(api.notes_line(sample_prediction()), "slate: DOORCreak 0.712 DOORCreak_Door Creak_NONE_NONE",
    "line")
end)

test("notes_line refuses a prediction without a cat_id", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local line, err = api.notes_line({ confidence = 0.5 })
  is_nil(line, "line")
  if not err then
    error("expected an error message")
  end
end)

test("notes_line refuses a non-table", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local line, err = api.notes_line("nope")
  is_nil(line, "line")
  if not err then
    error("expected an error message")
  end
end)

-- ---------------------------------------------------------------- render_pattern

test("render_pattern builds the UCS pattern", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  eq(api.render_pattern("Jon Evans", "Zoom H6"), "$item_Jon Evans_Zoom H6", "pattern")
end)

test("render_pattern refuses an invalid creator or source", function()
  local api = slate_api.new({ reaper = fake_reaper(), exec = function() return "" end })
  local pattern, err = api.render_pattern("", "Zoom H6")
  is_nil(pattern, "pattern")
  if not err then
    error("expected an error message")
  end
  local other, other_err = api.render_pattern("Jon", "a/b")
  is_nil(other, "pattern")
  if not other_err then
    error("expected an error message")
  end
end)

-- ---------------------------------------------------------------- has_extension

test("has_extension is false without the extension", function()
  local api = api_with()
  eq(api.has_extension(), false, "has_extension")
end)

test("has_extension is true when health is exported", function()
  local api = api_with({ extension = { health = function() return 1 end } })
  eq(api.has_extension(), true, "has_extension")
end)

test("has_extension is false when the table lacks health", function()
  local api = api_with({ extension = { schema_version = function() return 1 end } })
  eq(api.has_extension(), false, "has_extension")
end)

-- ---------------------------------------------------------------- health

test("health returns the extension value when present", function()
  local api = api_with({ extension = { health = function() return 0 end } })
  eq(api.health(), 0, "health")
end)

test("health reads the CLI fallback", function()
  local api = api_with({ exec = function() return '{"reachable":true,"warm":true}' end })
  eq(api.health(), 1, "warm")
end)

test("health reports cold when reachable but not warm", function()
  local api = api_with({ exec = function() return '{"reachable":true,"warm":false}' end })
  eq(api.health(), 0, "cold")
end)

test("health reports unreachable when the CLI fails", function()
  local api = api_with({ exec = function() error("no binary") end })
  eq(api.health(), -1, "unreachable")
end)

test("health reports unreachable on malformed JSON", function()
  local api = api_with({ exec = function() return "not json" end })
  eq(api.health(), -1, "unreachable")
end)

-- ---------------------------------------------------------------- predict

test("predict uses the extension when present", function()
  local calls = 0
  local api = api_with({
    extension = {
      health = function() return 1 end,
      predict = function(description, top_k)
        calls = calls + 1
        eq(description, "door creak", "description")
        eq(top_k, 3, "top_k")
        return sample_prediction()
      end,
    },
  })
  local prediction = api.predict("door creak")
  eq(prediction.cat_id, "DOORCreak", "cat_id")
  eq(calls, 1, "calls")
end)

test("predict falls back to the CLI", function()
  local command = nil
  local api = api_with({
    exec = function(line)
      command = line
      return '[{"cat_id":"DOORCreak","confidence":0.7,"filename":"DOORCreak_Door Creak_NONE_NONE"}]'
    end,
  })
  local prediction = api.predict("door creak")
  eq(prediction.cat_id, "DOORCreak", "cat_id")
  if not command:find("slate predict") then
    error("expected a slate predict command, got " .. tostring(command))
  end
  if not command:find("--json") then
    error("expected --json in " .. tostring(command))
  end
end)

test("predict refuses an empty description", function()
  local api = api_with()
  local prediction, err = api.predict("")
  is_nil(prediction, "prediction")
  if not err then
    error("expected an error message")
  end
end)

test("predict refuses a non-positive top_k", function()
  local api = api_with()
  local prediction, err = api.predict("door", 0)
  is_nil(prediction, "prediction")
  if not err then
    error("expected an error message")
  end
end)

test("predict reports an extension that throws", function()
  local api = api_with({
    extension = { health = function() return 1 end, predict = function() error("boom") end },
  })
  local prediction, err = api.predict("door")
  is_nil(prediction, "prediction")
  if not err or not err:find("boom") then
    error("expected the thrown message, got " .. tostring(err))
  end
end)

test("predict reports an empty CLI result", function()
  local api = api_with({ exec = function() return "[]" end })
  local prediction, err = api.predict("door")
  is_nil(prediction, "prediction")
  if not err then
    error("expected an error message")
  end
end)

test("predict reports malformed CLI JSON", function()
  local api = api_with({ exec = function() return "{" end })
  local prediction, err = api.predict("door")
  is_nil(prediction, "prediction")
  if not err then
    error("expected an error message")
  end
end)

-- ---------------------------------------------------------------- tag_item

test("tag_item writes notes and the take name", function()
  local api, state = api_with()
  local ok = api.tag_item(1, sample_prediction())
  eq(ok, true, "ok")
  eq(state.notes[1], "slate: DOORCreak 0.712 DOORCreak_Door Creak_NONE_NONE", "notes")
  eq(state.take_names[10], "DOORCreak_Door Creak_NONE_NONE", "take name")
end)

test("tag_item preserves existing notes", function()
  local api, state = api_with()
  state.notes[2] = "take 3\nslate: OLD 0.1 x"
  api.tag_item(2, sample_prediction())
  eq(state.notes[2], "take 3\nslate: DOORCreak 0.712 DOORCreak_Door Creak_NONE_NONE", "notes")
end)

test("tag_item refuses a missing item", function()
  local api = api_with()
  local ok, err = api.tag_item(0, sample_prediction())
  eq(ok, false, "ok")
  if not err then
    error("expected an error message")
  end
end)

test("tag_item refuses a prediction without a cat_id", function()
  local api = api_with()
  local ok, err = api.tag_item(1, {})
  eq(ok, false, "ok")
  if not err then
    error("expected an error message")
  end
end)

test("tag_item skips the take name when the filename is empty", function()
  local api, state = api_with()
  local prediction = sample_prediction()
  prediction.filename = ""
  eq(api.tag_item(3, prediction), true, "ok")
  is_nil(state.take_names[30], "take name")
end)

-- ---------------------------------------------------------------- tag_items

test("tag_items wraps the batch in one undo block", function()
  local api, state = api_with({
    extension = {
      health = function() return 1 end,
      predict = function() return sample_prediction() end,
    },
  })
  local items = { 1, 2, 3 }
  local tagged = api.tag_items(items, function(item) return "desc " .. item end)
  eq(tagged, 3, "tagged")
  eq(state.undo_begin, 1, "undo_begin")
  eq(state.undo_end, 1, "undo_end")
  eq(state.undo_labels[1], "Slate-To-UCS: tag items", "label")
  eq(state.undo_flags, -1, "flags")
end)

test("tag_items skips an item with no description", function()
  local api, state = api_with({
    extension = {
      health = function() return 1 end,
      predict = function() return sample_prediction() end,
    },
  })
  local tagged = api.tag_items({ 1, 2 }, function(item)
    if item == 2 then
      return nil
    end
    return "desc"
  end)
  eq(tagged, 1, "tagged")
  eq(state.notes[2], nil, "second item untouched")
end)

test("tag_items stops when the progress callback cancels", function()
  local api = api_with({
    extension = {
      health = function() return 1 end,
      predict = function() return sample_prediction() end,
    },
  })
  local seen = 0
  local tagged, message = api.tag_items({ 1, 2, 3, 4 }, function() return "desc" end,
    function(done)
      seen = done
      return done < 2
    end)
  eq(tagged, 2, "tagged")
  eq(seen, 2, "seen")
  eq(message, "cancelled", "message")
end)

test("tag_items refuses a non-table", function()
  local api = api_with()
  local tagged, err = api.tag_items(nil, function() return "x" end)
  eq(tagged, 0, "tagged")
  if not err then
    error("expected an error message")
  end
end)

test("tag_items refuses a missing description function", function()
  local api = api_with()
  local tagged, err = api.tag_items({ 1 }, nil)
  eq(tagged, 0, "tagged")
  if not err then
    error("expected an error message")
  end
end)

test("tag_items closes the undo block even when every prediction fails", function()
  local api, state = api_with({ exec = function() return "[]" end })
  local tagged = api.tag_items({ 1, 2 }, function() return "desc" end)
  eq(tagged, 0, "tagged")
  eq(state.undo_begin, 1, "undo_begin")
  eq(state.undo_end, 1, "undo_end")
end)

-- ---------------------------------------------------------------- open_app

test("open_app launches the path", function()
  local api, state = api_with()
  eq(api.open_app("/opt/slate"), true, "ok")
  eq(state.launched, "/opt/slate", "launched")
  eq(state.launch_flags, 0, "flags")
end)

test("open_app refuses an empty path", function()
  local api = api_with()
  local ok, err = api.open_app("")
  eq(ok, false, "ok")
  if not err then
    error("expected an error message")
  end
end)

test("open_app reports a launch that throws", function()
  local reaper = fake_reaper()
  reaper.ExecProcess = function() error("no such file") end
  local api = slate_api.new({ reaper = reaper, exec = function() return "" end })
  local ok, err = api.open_app("/nope")
  eq(ok, false, "ok")
  if not err or not err:find("/nope") then
    error("expected the path in the message, got " .. tostring(err))
  end
end)

-- ---------------------------------------------------------------- construction

test("new refuses a missing dependency table", function()
  local ok = pcall(slate_api.new, nil)
  eq(ok, false, "pcall")
end)

test("new refuses a missing reaper table", function()
  local ok = pcall(slate_api.new, { exec = function() return "" end })
  eq(ok, false, "pcall")
end)

test("new refuses a missing exec function", function()
  local ok = pcall(slate_api.new, { reaper = fake_reaper() })
  eq(ok, false, "pcall")
end)

-- ---------------------------------------------------------------- report

for _, failure in ipairs(failures) do
  io.stderr:write("FAIL " .. failure .. "\n")
end
io.write(string.format("%d checks, %d failures\n", passed + failed, failed))
os.exit(failed == 0 and 0 or 1)
