#!/usr/bin/env bash
# Migration script: tasks.md -> beads JSONL-first runtime.
# Usage: ./_vida/scripts/migrate-tasks-to-beads.sh <path/to/tasks.md>
#
# To make executable: chmod +x _vida/scripts/migrate-tasks-to-beads.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$ROOT_DIR/_vida/scripts/beads-runtime.sh"

TASKS_FILE="${1:-}"

if [[ -z "$TASKS_FILE" ]]; then
    echo "Usage: ./_vida/scripts/migrate-tasks-to-beads.sh <path/to/tasks.md>" >&2
    exit 1
fi

if [ ! -f "$TASKS_FILE" ]; then
    echo "Error: $TASKS_FILE not found"
    exit 1
fi

if [ ! -d ".beads" ]; then
    echo "Initializing beads..."
    beads_br init
fi

echo "Parsing $TASKS_FILE..."

# Extract milestone from path
MILESTONE=$(echo "$TASKS_FILE" | grep -oP '\d+\.\d+-[^/]+' || echo "unknown")

# Parse table rows (skip header and separator lines)
# Format: | T-XX | Task | Files | AC | ∥ | Status |
grep -E '^\| T-[0-9]+' "$TASKS_FILE" | while IFS='|' read -r _ id task files ac parallel status _; do
    # Trim whitespace
    id=$(echo "$id" | xargs)
    task=$(echo "$task" | xargs)
    status=$(echo "$status" | xargs)
    
    # Skip if already exists
    if beads_br show "$id" &>/dev/null; then
        echo "Skip: $id already exists"
        continue
    fi
    
    # Determine priority from task content
    priority=2  # default: medium
    if [[ "$task" == *"Security"* ]] || [[ "$task" == *"Auth"* ]]; then
        priority=1  # high for security tasks
    fi
    if [[ "$task" == *"Test"* ]] || [[ "$task" == *"Config"* ]]; then
        priority=3  # low for tests and config
    fi
    
    # Determine beads status from checkbox
    beads_status="open"
    if [[ "$status" == *"[x]"* ]]; then
        beads_status="closed"
    elif [[ "$status" == *"[/]"* ]]; then
        beads_status="in_progress"
    fi
    
    # Determine type from task content
    task_type="task"  # default
    if [[ "$task" == *"Test"* ]]; then
        task_type="task"  # tests are still tasks
    fi
    
    echo "Creating: $id - $task (status: $beads_status, priority: $priority)"
    
    # Create issue (title format: "T-XX: Description")
    beads_br create "$id: $task" --type="$task_type" --priority="$priority"
    
    # Update status if not open
    if [ "$beads_status" != "open" ]; then
        # Get the issue ID by searching for the title
        ISSUE_ID=$(beads_br list --json 2>/dev/null | jq -r ".[] | select(.title | startswith(\"$id:\")) | .id" | head -1)
        
        if [ -n "$ISSUE_ID" ]; then
            if [ "$beads_status" == "closed" ]; then
                beads_br close "$ISSUE_ID" --reason="Migrated from tasks.md (already complete)"
            else
                beads_br update "$ISSUE_ID" --status="$beads_status"
            fi
        fi
    fi
done

# Sync to JSONL
echo "Syncing changes..."
beads_br sync --flush-only

echo ""
echo "✅ Migration complete!"
echo ""
echo "Next steps:"
echo "  1. Run 'bash _vida/scripts/br-safe.sh list' to see imported tasks"
echo "  2. Run 'bash _vida/scripts/br-safe.sh ready' to see tasks ready to work"
echo "  3. Run 'bv' for interactive view (TUI)"
echo "  4. Run 'bash _vida/scripts/br-safe.sh sync --flush-only' to persist changes"
