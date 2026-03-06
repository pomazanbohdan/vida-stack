#!/usr/bin/env python3
import argparse
import datetime as dt
import fcntl
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BEADS_DIR = ROOT / ".beads"
ISSUES_JSONL = BEADS_DIR / "issues.jsonl"
LOCK_FILE = BEADS_DIR / "issues.jsonl.lock"
LAST_TOUCHED = BEADS_DIR / "last-touched"


def now_utc() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def load_issues():
    issues = []
    with ISSUES_JSONL.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            issues.append(json.loads(line))
    return issues


def write_issues(issues):
    tmp = ISSUES_JSONL.with_suffix(".jsonl.tmp")
    with tmp.open("w") as fh:
        for issue in issues:
            fh.write(json.dumps(issue, ensure_ascii=True, separators=(",", ":")))
            fh.write("\n")
    tmp.replace(ISSUES_JSONL)


def find_issue(issues, issue_id):
    for issue in issues:
        if issue.get("id") == issue_id:
            return issue
    raise SystemExit(f"Issue not found: {issue_id}")


def set_labels(issue, set_labels, add_labels, remove_labels):
    labels = list(issue.get("labels", []))
    if set_labels is not None:
      labels = []
      for label in set_labels:
          if label not in labels:
              labels.append(label)

    for label in add_labels or []:
        if label not in labels:
            labels.append(label)

    for label in remove_labels or []:
        labels = [existing for existing in labels if existing != label]

    if labels:
        issue["labels"] = labels
    else:
        issue.pop("labels", None)


def apply_update(issue, args):
    changed = False

    if args.status is not None:
        issue["status"] = args.status
        if args.status != "closed":
            issue.pop("closed_at", None)
            issue.pop("close_reason", None)
        changed = True

    if args.notes is not None:
        issue["notes"] = args.notes
        changed = True

    if args.description is not None:
        issue["description"] = args.description
        changed = True

    if args.set_labels is not None or args.add_label or args.remove_label:
        set_labels(issue, args.set_labels, args.add_label, args.remove_label)
        changed = True

    if changed:
        issue["updated_at"] = now_utc()


def apply_close(issue, args):
    issue["status"] = "closed"
    issue["updated_at"] = now_utc()
    issue["closed_at"] = issue["updated_at"]
    issue["close_reason"] = args.reason


def update_last_touched(issue_id: str):
    LAST_TOUCHED.write_text(issue_id)


def parse_args():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    update = sub.add_parser("update")
    update.add_argument("issue_id")
    update.add_argument("--status")
    update.add_argument("--notes")
    update.add_argument("--description")
    update.add_argument("--add-label", action="append")
    update.add_argument("--remove-label", action="append")
    update.add_argument("--set-labels", action="append")
    update.add_argument("--json", action="store_true")

    close = sub.add_parser("close")
    close.add_argument("issue_id")
    close.add_argument("--reason", required=True)
    close.add_argument("--json", action="store_true")

    return parser.parse_args()


def main():
    args = parse_args()

    BEADS_DIR.mkdir(exist_ok=True)
    LOCK_FILE.touch(exist_ok=True)

    with LOCK_FILE.open("r+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        issues = load_issues()
        issue = find_issue(issues, args.issue_id)

        if args.command == "update":
            apply_update(issue, args)
        elif args.command == "close":
            apply_close(issue, args)

        write_issues(issues)
        update_last_touched(args.issue_id)

    if getattr(args, "json", False):
        print(json.dumps(issue, ensure_ascii=True))


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        sys.exit(0)
