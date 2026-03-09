## Tests for core/utils module

import std/[json, options, os, strutils, times, unittest]
import ../src/core/utils

suite "policyValue":
  test "returns default for nil":
    check policyValue(newJNull(), "default") == "default"

  test "returns default for empty string":
    check policyValue(newJString("  "), "fallback") == "fallback"

  test "returns trimmed value":
    check policyValue(newJString("  hello  "), "x") == "hello"

  test "converts int to string":
    check policyValue(newJInt(42), "x") == "42"

suite "policyInt":
  test "returns default for nil":
    check policyInt(newJNull(), 99) == 99

  test "returns int value":
    check policyInt(newJInt(42), 0) == 42

  test "parses string to int":
    check policyInt(newJString("7"), 0) == 7

  test "returns default for non-numeric string":
    check policyInt(newJString("abc"), -1) == -1

suite "splitCsv":
  test "splits comma-separated string":
    check splitCsv(newJString("a, b, c")) == @["a", "b", "c"]

  test "handles JSON array":
    check splitCsv(parseJson("""["x", "y"]""")) == @["x", "y"]

  test "returns empty for null":
    check splitCsv(newJNull()) == newSeq[string]()

  test "skips empty items":
    check splitCsv(newJString("a,,b, ,c")) == @["a", "b", "c"]

suite "safeName":
  test "passes valid names through":
    check safeName("hello-world", "fallback") == "hello-world"

  test "replaces invalid chars":
    check safeName("hello world!", "fb") == "hello-world-"

  test "uses fallback for empty":
    check safeName("", "fallback") == "fallback"

suite "cleanText":
  test "collapses whitespace":
    check cleanText("hello   world") == "hello world"

  test "replaces newlines":
    check cleanText("line1\nline2") == "line1 line2"

  test "returns dash for empty":
    check cleanText("") == "-"

suite "dottedGet":
  test "navigates nested object":
    let obj = parseJson("""{"a": {"b": {"c": 42}}}""")
    check dottedGet(obj, "a.b.c") == newJInt(42)

  test "returns default for missing path":
    let obj = parseJson("""{"a": 1}""")
    check dottedGet(obj, "a.b.c", newJString("miss")) == newJString("miss")

  test "returns default for nil":
    check dottedGet(newJNull(), "anything", newJString("d")) == newJString("d")

suite "dottedGetStr":
  test "extracts string value":
    let obj = parseJson("""{"key": "value"}""")
    check dottedGetStr(obj, "key") == "value"

  test "returns default for missing":
    let obj = newJObject()
    check dottedGetStr(obj, "missing", "default") == "default"

suite "time functions":
  test "nowUtc returns ISO format with Z":
    let ts = nowUtc()
    check ts.endsWith("Z")
    check ts.len == 20  # "2024-01-01T00:00:00Z"

  test "parseUtcTimestamp handles valid timestamp":
    let dt = parseUtcTimestamp("2024-06-15T10:30:00Z")
    check dt.isSome

  test "parseUtcTimestamp returns none for empty":
    check parseUtcTimestamp("").isNone

  test "parseUtcTimestamp returns none for garbage":
    check parseUtcTimestamp("not-a-date").isNone

suite "normalizeDomainTag":
  test "applies alias":
    check normalizeDomainTag("odoo_api") == "api_contract"
    check normalizeDomainTag("flutter_ui") == "frontend_ui"

  test "lowercases unknown tags":
    check normalizeDomainTag("CUSTOM_TAG") == "custom_tag"

suite "JSON I/O":
  test "loadJson returns default for missing file":
    let result = loadJson("/tmp/_vida_test_nonexistent_file.json")
    check result.kind == JObject

  test "saveJson and loadJson roundtrip":
    let path = "/tmp/_vida_test_roundtrip.json"
    let data = %*{"key": "value", "num": 42}
    saveJson(path, data)
    let loaded = loadJson(path)
    check loaded["key"].getStr() == "value"
    check loaded["num"].getInt() == 42
    removeFile(path)
