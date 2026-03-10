## VIDA Guard Engine — boolean guard evaluation for root config machines and policies.

import std/[json, strutils]
import ./utils

proc truthyValue(node: JsonNode): bool =
  if node.isNil or node.kind == JNull:
    return false
  case node.kind
  of JNull:
    false
  of JBool:
    node.getBool()
  of JInt:
    node.getInt() != 0
  of JFloat:
    node.getFloat() != 0.0
  of JString:
    let value = node.getStr().strip().toLowerAscii()
    value in ["true", "yes", "on", "1", "passed", "ready", "completed", "approved"]
  of JObject:
    node.len > 0
  of JArray:
    node.len > 0

proc resolveGuardFlag*(ctx: JsonNode, name: string): bool =
  if name.len == 0:
    return false
  let direct = dottedGet(ctx, name)
  if not direct.isNil and direct.kind != JNull:
    return truthyValue(direct)
  if ctx.kind == JObject and ctx.hasKey(name):
    return truthyValue(ctx[name])
  false

proc evalGuardExpr*(expr, ctx: JsonNode): bool =
  if expr.isNil or expr.kind == JNull:
    return true
  case expr.kind
  of JString:
    resolveGuardFlag(ctx, expr.getStr())
  of JArray:
    for item in expr:
      if not evalGuardExpr(item, ctx):
        return false
    true
  of JObject:
    if expr.hasKey("all_of"):
      for item in expr["all_of"]:
        if not evalGuardExpr(item, ctx):
          return false
      return true
    if expr.hasKey("any_of"):
      for item in expr["any_of"]:
        if evalGuardExpr(item, ctx):
          return true
      return false
    if expr.hasKey("not"):
      return not evalGuardExpr(expr["not"], ctx)
    false
  else:
    truthyValue(expr)
