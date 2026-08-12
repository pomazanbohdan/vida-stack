package main

import (
	"bufio"
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
	"unicode"
)

const usage = `Render the public GitHub release body from a canonical release-note artifact.

Usage:
  scripts/render-public-release-notes.sh <vX.Y.Z|path-to-release-note.md>
`

type gitRunner interface {
	run(context.Context, ...string) ([]byte, error)
}

type commandGit struct {
	path string
	dir  string
}

func (g commandGit) run(ctx context.Context, args ...string) ([]byte, error) {
	command := exec.CommandContext(ctx, g.path, args...)
	command.Dir = g.dir
	return command.Output()
}

func main() {
	if len(os.Args) < 2 {
		_, _ = fmt.Fprint(os.Stderr, usage)
		os.Exit(1)
	}

	root, err := discoverRepoRoot()
	if err != nil {
		fail(err)
	}

	gitPath, err := resolveGit()
	if err != nil {
		fail(errors.New("git is required to render the public commit ledger"))
	}

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	output, err := renderRelease(ctx, root, os.Args[1], commandGit{path: gitPath, dir: root})
	if err != nil {
		fail(err)
	}
	if _, err := os.Stdout.Write(output); err != nil {
		fail(fmt.Errorf("write rendered release body: %w", err))
	}
}

func fail(err error) {
	_, _ = fmt.Fprintf(os.Stderr, "[render-public-release-notes] ERROR: %s\n", err)
	os.Exit(1)
}

func resolveGit() (string, error) {
	name := strings.TrimSpace(os.Getenv("GIT"))
	if name == "" {
		name = "git"
	}
	return exec.LookPath(name)
}

func discoverRepoRoot() (string, error) {
	starts := make([]string, 0, 2)
	if configured := strings.TrimSpace(os.Getenv("VIDA_REPO_ROOT")); configured != "" {
		starts = append(starts, configured)
	}
	if cwd, err := os.Getwd(); err == nil {
		starts = append(starts, cwd)
	}

	for _, start := range starts {
		if root := findRepoRoot(start); root != "" {
			return root, nil
		}
	}
	return "", errors.New("repository root not found; set VIDA_REPO_ROOT")
}

func findRepoRoot(start string) string {
	dir, err := filepath.Abs(start)
	if err != nil {
		return ""
	}
	if info, err := os.Stat(dir); err == nil && !info.IsDir() {
		dir = filepath.Dir(dir)
	}

	for {
		installDir := filepath.Join(dir, "install")
		if isDirectory(installDir) && (pathExists(filepath.Join(dir, ".git")) || pathExists(filepath.Join(dir, "Cargo.toml"))) {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return ""
		}
		dir = parent
	}
}

func pathExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func isDirectory(path string) bool {
	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}

func renderRelease(ctx context.Context, root, input string, git gitRunner) ([]byte, error) {
	sourcePath, inputIsPath := resolveSourcePath(root, input)
	if !pathExists(sourcePath) {
		return nil, fmt.Errorf("release-note source not found: %s", sourcePath)
	}

	currentTag, err := resolveCurrentTag(input, inputIsPath)
	if err != nil {
		return nil, err
	}
	if _, err := git.run(ctx, "rev-parse", "--verify", currentTag+"^{commit}"); err != nil {
		return nil, fmt.Errorf("release tag not found: %s", currentTag)
	}

	previousTag, err := resolvePreviousTag(ctx, currentTag, git)
	if err != nil {
		return nil, err
	}
	if previousTag == "" {
		return nil, fmt.Errorf("previous release tag not found before %s", currentTag)
	}
	if _, err := git.run(ctx, "rev-parse", "--verify", previousTag+"^{commit}"); err != nil {
		return nil, fmt.Errorf("previous release tag not found: %s", previousTag)
	}

	note, err := os.ReadFile(sourcePath)
	if err != nil {
		return nil, fmt.Errorf("read release-note source: %w", err)
	}
	body, err := renderBody(note)
	if err != nil {
		return nil, err
	}
	ledger, err := git.run(ctx, "log", "--no-merges", "--format=- `%h` %s", previousTag+".."+currentTag)
	if err != nil {
		return nil, fmt.Errorf("render commit ledger: %w", err)
	}

	var output bytes.Buffer
	output.WriteString(body)
	output.WriteString("\n## Commit Ledger\n\n")
	fmt.Fprintf(&output, "Commits since `%s`:\n\n", previousTag)
	output.Write(ledger)
	return output.Bytes(), nil
}

func resolveSourcePath(root, input string) (string, bool) {
	if info, err := os.Stat(input); err == nil && !info.IsDir() {
		return input, true
	}
	return filepath.Join(root, "install", "release-notes-"+input+".md"), false
}

func resolveCurrentTag(input string, inputIsPath bool) (string, error) {
	if !inputIsPath {
		return input, nil
	}
	base := filepath.Base(input)
	const prefix = "release-notes-"
	if !strings.HasPrefix(base, prefix) || !strings.HasSuffix(base, ".md") {
		return "", fmt.Errorf("cannot infer release tag from note path: %s", input)
	}
	return strings.TrimSuffix(strings.TrimPrefix(base, prefix), ".md"), nil
}

func resolvePreviousTag(ctx context.Context, currentTag string, git gitRunner) (string, error) {
	output, err := git.run(ctx, "tag", "--list", "v*", "--sort=-v:refname")
	if err != nil {
		return "", fmt.Errorf("resolve previous release tag: %w", err)
	}
	seenCurrent := false
	for _, line := range strings.Split(strings.ReplaceAll(string(output), "\r\n", "\n"), "\n") {
		line = strings.TrimSuffix(line, "\r")
		if !seenCurrent {
			if line == currentTag {
				seenCurrent = true
			}
			continue
		}
		if line != "" {
			return line, nil
		}
	}
	return "", nil
}

func renderBody(note []byte) (string, error) {
	scanner := bufio.NewScanner(bytes.NewReader(note))
	scanner.Buffer(make([]byte, 64*1024), 1024*1024)
	var body strings.Builder
	droppedTitle := false
	droppedBlankAfterTitle := false

	for scanner.Scan() {
		line := strings.TrimSuffix(scanner.Text(), "\r")
		if line == "-----" || strings.TrimRightFunc(line, unicode.IsSpace) == "## Commit Ledger" {
			break
		}
		if !droppedTitle && strings.HasPrefix(line, "# ") {
			droppedTitle = true
			continue
		}
		if droppedTitle && !droppedBlankAfterTitle && strings.TrimSpace(line) == "" {
			droppedBlankAfterTitle = true
			continue
		}
		body.WriteString(line)
		body.WriteByte('\n')
	}
	if err := scanner.Err(); err != nil {
		return "", fmt.Errorf("read release-note body: %w", err)
	}
	return body.String(), nil
}
