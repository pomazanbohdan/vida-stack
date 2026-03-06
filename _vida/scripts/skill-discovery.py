#!/usr/bin/env python3
import argparse
import json
import os
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent.parent
SKILLS_ROOT = ROOT / ".agents" / "skills"


def tokenize(text: str):
    return [t for t in re.split(r"[^a-z0-9_+-]+", text.lower()) if len(t) >= 2]


def parse_skill(path: Path):
    name = ""
    description = ""
    content = path.read_text(encoding="utf-8", errors="ignore")
    lines = content.splitlines()

    in_frontmatter = False
    fm_seen = 0
    for ln in lines[:80]:
        if ln.strip() == "---":
            fm_seen += 1
            in_frontmatter = fm_seen == 1
            if fm_seen >= 2:
                break
            continue
        if in_frontmatter:
            if ln.startswith("name:"):
                name = ln.split(":", 1)[1].strip()
            elif ln.startswith("description:"):
                description = ln.split(":", 1)[1].strip()

    if not name:
        name = path.parent.name

    preview = "\n".join(lines[:200]).lower()
    return {
        "path": str(path),
        "name": name,
        "description": description,
        "preview": preview,
    }


def score_skill(skill, query_tokens):
    hay_name = skill["name"].lower()
    hay_desc = skill["description"].lower()
    hay_path = skill["path"].lower()
    hay_prev = skill["preview"]

    score = 0
    matched = []
    for t in query_tokens:
        local = 0
        if t in hay_name:
            local += 5
        if t in hay_desc:
            local += 4
        if t in hay_path:
            local += 2
        if t in hay_prev:
            local += 1
        if local > 0:
            matched.append(t)
            score += local

    # tiny boost for focused skills in project/global/system namespaces
    if "/global/" in hay_path:
        score += 1
    if "/system/" in hay_path:
        score += 1
    if "/project/" in hay_path:
        score += 2

    return score, sorted(set(matched))


def cmd_suggest(args):
    if not SKILLS_ROOT.exists():
        raise SystemExit(f"Skills root not found: {SKILLS_ROOT}")

    query_tokens = tokenize(args.query)
    if not query_tokens:
        raise SystemExit("Query is empty after tokenization")

    skills = []
    for p in SKILLS_ROOT.rglob("SKILL.md"):
        try:
            s = parse_skill(p)
            s_score, matched = score_skill(s, query_tokens)
            if s_score > 0:
                skills.append({
                    "score": s_score,
                    "matched": matched,
                    "name": s["name"],
                    "description": s["description"],
                    "path": s["path"],
                })
        except Exception:
            continue

    skills.sort(key=lambda x: (-x["score"], x["name"]))
    top = skills[: args.top]

    if args.json:
        print(json.dumps(top, ensure_ascii=False, indent=2))
        return

    if not top:
        print("No matching skills")
        return

    for s in top:
        print(f"- {s['name']} (score={s['score']})")
        print(f"  path: {s['path']}")
        print(f"  desc: {s['description']}")
        print(f"  matched: {', '.join(s['matched'])}")


def cmd_scaffold(args):
    # Create project-specific skill skeleton only if absent.
    skill_name = re.sub(r"[^a-z0-9-]", "-", args.name.lower()).strip("-")
    if not skill_name:
        raise SystemExit("Invalid skill name")

    out_dir = SKILLS_ROOT / "project" / skill_name
    out_dir.mkdir(parents=True, exist_ok=True)
    skill_md = out_dir / "SKILL.md"

    if skill_md.exists() and not args.force:
        print(f"exists: {skill_md}")
        return

    body = f"""---
name: {skill_name}
description: {args.description}
---

# {skill_name}

Use this project-specific skill when request matches this domain.

## Trigger

- Keywords: TODO
- Scope: TODO

## Workflow

1. Read relevant project docs/specs.
2. Execute deterministic steps.
3. Log evidence and constraints.
4. Return concise result + risks.
"""
    skill_md.write_text(body, encoding="utf-8")
    print(f"created: {skill_md}")


def main():
    parser = argparse.ArgumentParser(description="Dynamic skill discovery and project skill scaffolding")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_suggest = sub.add_parser("suggest", help="Suggest best matching skills for a request")
    p_suggest.add_argument("query", help="User request or scope text")
    p_suggest.add_argument("--top", type=int, default=8)
    p_suggest.add_argument("--json", action="store_true")
    p_suggest.set_defaults(func=cmd_suggest)

    p_scaffold = sub.add_parser("scaffold", help="Create project skill skeleton")
    p_scaffold.add_argument("name", help="Skill short name")
    p_scaffold.add_argument("description", help="Skill description")
    p_scaffold.add_argument("--force", action="store_true")
    p_scaffold.set_defaults(func=cmd_scaffold)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
