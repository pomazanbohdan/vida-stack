package main

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

type fakeGit struct {
	outputs map[string][]byte
	errors  map[string]error
}

func (f fakeGit) run(_ context.Context, args ...string) ([]byte, error) {
	key := strings.Join(args, "\x00")
	if err := f.errors[key]; err != nil {
		return nil, err
	}
	return f.outputs[key], nil
}

func TestRenderReleaseMatchesContract(t *testing.T) {
	root := t.TempDir()
	installDir := filepath.Join(root, "install")
	if err := os.MkdirAll(installDir, 0o755); err != nil {
		t.Fatal(err)
	}
	note, err := os.ReadFile(filepath.Join("testdata", "release-notes-v1.2.3.md"))
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(installDir, "release-notes-v1.2.3.md"), note, 0o644); err != nil {
		t.Fatal(err)
	}

	fake := fakeGit{outputs: map[string][]byte{
		strings.Join([]string{"rev-parse", "--verify", "v1.2.3^{commit}"}, "\x00"):                   []byte("commit-current\n"),
		strings.Join([]string{"tag", "--list", "v*", "--sort=-v:refname"}, "\x00"):                   []byte("v1.2.3\nv1.2.2\n"),
		strings.Join([]string{"rev-parse", "--verify", "v1.2.2^{commit}"}, "\x00"):                   []byte("commit-previous\n"),
		strings.Join([]string{"log", "--no-merges", "--format=- `%h` %s", "v1.2.2..v1.2.3"}, "\x00"): []byte("- `abc1234` First change\n"),
	}}

	actual, err := renderRelease(context.Background(), root, "v1.2.3", fake)
	if err != nil {
		t.Fatal(err)
	}
	want := "## Highlights\n\n- New behavior.\n\n\n## Commit Ledger\n\nCommits since `v1.2.2`:\n\n- `abc1234` First change\n"
	if string(actual) != want {
		t.Fatalf("rendered body mismatch:\n--- got ---\n%s--- want ---\n%s", actual, want)
	}
}

func TestResolveCurrentTagFromPath(t *testing.T) {
	got, err := resolveCurrentTag(filepath.Join("install", "release-notes-v0.9.7.md"), true)
	if err != nil {
		t.Fatal(err)
	}
	if got != "v0.9.7" {
		t.Fatalf("tag = %q, want v0.9.7", got)
	}

	_, err = resolveCurrentTag("notes.md", true)
	if err == nil || !strings.Contains(err.Error(), "cannot infer release tag") {
		t.Fatalf("expected path inference error, got %v", err)
	}
}

func TestResolvePreviousTag(t *testing.T) {
	fake := fakeGit{outputs: map[string][]byte{
		strings.Join([]string{"tag", "--list", "v*", "--sort=-v:refname"}, "\x00"): []byte("v0.9.7\r\nv0.9.6\r\n"),
	}}
	got, err := resolvePreviousTag(context.Background(), "v0.9.7", fake)
	if err != nil {
		t.Fatal(err)
	}
	if got != "v0.9.6" {
		t.Fatalf("previous tag = %q, want v0.9.6", got)
	}
}

func TestRenderBodyStopsAtFooterOrExistingLedger(t *testing.T) {
	cases := []struct {
		name string
		note string
		want string
	}{
		{
			name: "footer",
			note: "# Title\n\n## Body\n\n-----\nmetadata\n",
			want: "## Body\n\n",
		},
		{
			name: "existing ledger",
			note: "# Title\n\n## Body\n\n## Commit Ledger\nold\n",
			want: "## Body\n\n",
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := renderBody([]byte(tc.note))
			if err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(got, tc.want) {
				t.Fatalf("body = %q, want %q", got, tc.want)
			}
		})
	}
}

func TestRenderReleasePropagatesGitFailure(t *testing.T) {
	root := t.TempDir()
	installDir := filepath.Join(root, "install")
	if err := os.MkdirAll(installDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(installDir, "release-notes-v1.2.3.md"), []byte("# Title\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	fake := fakeGit{errors: map[string]error{
		strings.Join([]string{"rev-parse", "--verify", "v1.2.3^{commit}"}, "\x00"): errors.New("git failed"),
	}}
	_, err := renderRelease(context.Background(), root, "v1.2.3", fake)
	if err == nil || !strings.Contains(err.Error(), "release tag not found: v1.2.3") {
		t.Fatalf("expected release tag error, got %v", err)
	}
}
