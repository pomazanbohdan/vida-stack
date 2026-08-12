package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func repoRoot(t *testing.T) string {
	t.Helper()
	root, err := filepath.Abs(filepath.Join("..", ".."))
	if err != nil {
		t.Fatal(err)
	}
	return root
}

func TestFixturesMatchPowerShellContract(t *testing.T) {
	cases := map[string][]string{
		"pass.md":                             nil,
		"timezone-valid-z.md":                 nil,
		"timezone-valid-negative-fraction.md": nil,
		"timezone-valid-positive-boundary.md": nil,
		"missing-pr-processing.md":            {"missing_heading", "missing_heading"},
		"missing-task-ref.md":                 {"missing_heading", "missing_implementation_task_ref"},
		"proof-command-mismatch.md":           {"declared_executed_proof_mismatch"},
		"proof-command-multiline-mismatch.md": {"declared_executed_proof_mismatch"},
		"proof-count-shrinkage.md":            {"proof_count_shrinkage_without_rationale"},
		"stale-pending.md":                    {"missing_heading", "stale_placeholder"},
		"timezone-invalid-calendar.md":        {"updated_at_invalid_value"},
		"timezone-invalid-date-only.md":       {"updated_at_invalid_format"},
		"timezone-invalid-malformed.md":       {"updated_at_invalid_format"},
		"timezone-invalid-offset.md":          {"updated_at_invalid_offset"},
		"timezone-invalid-space.md":           {"updated_at_invalid_format"},
		"timezone-missing-updated-at.md":      {"missing_updated_at"},
		"unclosed-processed-issue.md":         {"missing_heading", "invalid_processed_issue_closure"},
		"zero-test-proof.md":                  {"zero_test_proof"},
	}
	root := repoRoot(t)
	for name, expected := range cases {
		t.Run(name, func(t *testing.T) {
			path := filepath.Join(root, "tests", "fixtures", "agent-evaluation-log", name)
			got, err := evaluate(path, false, root)
			if err != nil {
				t.Fatal(err)
			}
			if len(expected) == 0 && got.Status != "pass" {
				t.Fatalf("expected pass, got %+v", got)
			}
			if len(expected) != len(got.Issues) {
				t.Fatalf("issue count = %d, want %d (%+v)", len(got.Issues), len(expected), got.Issues)
			}
			for index, code := range expected {
				if got.Issues[index].Code != code {
					t.Fatalf("issue[%d] = %s, want %s", index, got.Issues[index].Code, code)
				}
			}
		})
	}
}

func TestMissingFileProducesBlockedEnvelope(t *testing.T) {
	got, err := evaluate("does-not-exist.md", false, t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	if got.Status != "blocked" || got.IssueCount != 1 || got.Issues[0].Code != "missing_file" {
		t.Fatalf("unexpected result: %+v", got)
	}
}

func TestJSONEnvelopeRetainsNullFields(t *testing.T) {
	root := repoRoot(t)
	got, err := evaluate(filepath.Join(root, "tests", "fixtures", "agent-evaluation-log", "pass.md"), false, root)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(got)
	if err != nil {
		t.Fatal(err)
	}
	if string(raw) == "" || !strings.Contains(string(raw), `"error_code":null`) || !strings.Contains(string(raw), `"source_field":null`) {
		t.Fatalf("nullable result fields missing: %s", raw)
	}
}

func TestSourceCalendarDateValidation(t *testing.T) {
	for _, value := range []string{"2026-08-12T12:00:00Z", "2026-08-12T12:00:00.1234567-05:30", "2026-08-12T12:00:00+14:00"} {
		if _, code, _ := sourceCalendarDate(value); code != "" {
			t.Fatalf("valid timestamp %q returned %s", value, code)
		}
	}
	for value, want := range map[string]string{
		"2026-08-12":                "updated_at_invalid_format",
		"2026-08-12T12:00:00+14:01": "updated_at_invalid_offset",
		"2026-02-30T12:00:00Z":      "updated_at_invalid_value",
	} {
		if _, code, _ := sourceCalendarDate(value); code != want {
			t.Fatalf("timestamp %q returned %s, want %s", value, code, want)
		}
	}
}

func TestResolvePathUsesCurrentDirectory(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "fixture.md")
	if err := os.WriteFile(path, []byte(""), 0o644); err != nil {
		t.Fatal(err)
	}
	resolved, err := resolvePath("fixture.md", dir)
	if err != nil || resolved != path {
		t.Fatalf("resolvePath() = %q, %v; want %q", resolved, err, path)
	}
}
