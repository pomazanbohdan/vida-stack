package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
)

type check struct {
	Name    string   `json:"name"`
	Status  string   `json:"status"`
	Matches []string `json:"matches"`
}

type result struct {
	Surface string  `json:"surface"`
	Status  string  `json:"status"`
	Checks  []check `json:"checks"`
}

type options struct {
	root string
	json bool
}

const boundarySurface = "tools/check-runtime-boundaries"

func main() {
	if err := run(os.Args[1:], os.Stdout, mustWorkingDirectory()); err != nil {
		if errors.Is(err, errBlocked) {
			os.Exit(1)
		}
		_, _ = fmt.Fprintf(os.Stderr, "[check-runtime-boundaries] ERROR: %s\n", err)
		os.Exit(1)
	}
}

var errBlocked = errors.New("validation blocked")

func mustWorkingDirectory() string {
	cwd, err := os.Getwd()
	if err != nil {
		return "."
	}
	return cwd
}

func run(args []string, output io.Writer, cwd string) error {
	opts, err := parseOptions(args)
	if err != nil {
		return err
	}
	root, err := resolveRoot(opts.root, cwd)
	if err != nil {
		return err
	}
	checked, err := evaluate(root, findRipgrep())
	if err != nil {
		return err
	}
	if opts.json {
		if err := json.NewEncoder(output).Encode(checked); err != nil {
			return err
		}
		if checked.Status != "pass" {
			return errBlocked
		}
		return nil
	}
	if _, err := fmt.Fprintf(output, "runtime boundary checks: %s\n", checked.Status); err != nil {
		return err
	}
	for _, item := range checked.Checks {
		if _, err := fmt.Fprintf(output, "- %s: %s\n", item.Name, item.Status); err != nil {
			return err
		}
		for _, match := range item.Matches {
			if _, err := fmt.Fprintf(output, "  %s\n", match); err != nil {
				return err
			}
		}
	}
	if checked.Status != "pass" {
		return errBlocked
	}
	return nil
}

func parseOptions(args []string) (options, error) {
	set := flag.NewFlagSet("check-runtime-boundaries", flag.ContinueOnError)
	set.SetOutput(io.Discard)
	opts := options{}
	set.StringVar(&opts.root, "root", "", "repository root")
	set.BoolVar(&opts.json, "json", false, "emit JSON")
	if err := set.Parse(args); err != nil {
		return options{}, err
	}
	if set.NArg() != 0 {
		return options{}, fmt.Errorf("unexpected argument: %s", set.Arg(0))
	}
	return opts, nil
}

func resolveRoot(configured, cwd string) (string, error) {
	start := strings.TrimSpace(configured)
	if start == "" {
		start = cwd
	}
	root, err := filepath.Abs(start)
	if err != nil {
		return "", err
	}
	return root, nil
}

func findRipgrep() string {
	if configured := strings.TrimSpace(os.Getenv("RG")); configured != "" {
		if info, err := os.Stat(configured); err == nil && !info.IsDir() {
			return configured
		}
	}
	path, err := exec.LookPath("rg")
	if err != nil {
		return ""
	}
	return path
}

func evaluate(root, rg string) (result, error) {
	vidaPaths := existingPaths([]string{filepath.Join(root, "crates", "vida", "src")})
	checks := []check{
		pathAbsentCheck("legacy vida operator facade files removed", []string{
			filepath.Join(root, "crates", "vida", "src", "operator_command_text.rs"),
			filepath.Join(root, "crates", "vida", "src", "operator_contracts.rs"),
			filepath.Join(root, "crates", "vida", "src", "operator_toon_report.rs"),
		}),
		pathPresentCheck("release1 operator output bridge present", filepath.Join(root, "crates", "vida", "src", "release1_operator_output.rs")),
		invokeRgCheck("no legacy vida operator facade imports", `mod operator_(command_text|contracts|toon_report)|crate::operator_(command_text|contracts|toon_report)|use crate::operator_(command_text|contracts|toon_report)::`, vidaPaths, []string{"!**/tests/**", "!**/generated/**", "!**/adapters/**"}, rg),
		invokeRgCheck("no broad runtime_dispatch_state export", `pub\(crate\) use runtime_dispatch_state::\*`, vidaPaths, []string{"!**/tests/**", "!**/generated/**", "!**/adapters/**"}, rg),
	}
	status := "pass"
	for _, item := range checks {
		if item.Status != "pass" {
			status = "blocked"
			break
		}
	}
	return result{Surface: boundarySurface, Status: status, Checks: checks}, nil
}

func existingPaths(paths []string) []string {
	result := make([]string, 0, len(paths))
	for _, path := range paths {
		if _, err := os.Stat(path); err == nil {
			result = append(result, path)
		}
	}
	return result
}

func pathAbsentCheck(name string, paths []string) check {
	matches := make([]string, 0)
	for _, path := range paths {
		if _, err := os.Stat(path); err == nil {
			matches = append(matches, path)
		}
	}
	if len(matches) == 0 {
		return check{Name: name, Status: "pass", Matches: []string{}}
	}
	return check{Name: name, Status: "blocked", Matches: matches}
}

func pathPresentCheck(name, path string) check {
	if _, err := os.Stat(path); err == nil {
		return check{Name: name, Status: "pass", Matches: []string{}}
	}
	return check{Name: name, Status: "blocked", Matches: []string{"missing: " + path}}
}

func invokeRgCheck(name, pattern string, paths, globs []string, rg string) check {
	if rg == "" {
		return invokeFallbackCheck(name, pattern, paths)
	}
	args := []string{"--color", "never", "--line-number", pattern}
	args = append(args, paths...)
	for _, glob := range globs {
		args = append(args, "-g", glob)
	}
	command := exec.Command(rg, args...)
	var stdout, stderr bytes.Buffer
	command.Stdout = &stdout
	command.Stderr = &stderr
	err := command.Run()
	lines := combinedLines(stdout.String(), stderr.String())
	lines = truncate(lines)
	if exitCode(err) == 1 {
		return check{Name: name, Status: "pass", Matches: []string{}}
	}
	if err != nil {
		return check{Name: name, Status: "error", Matches: lines}
	}
	return check{Name: name, Status: "blocked", Matches: lines}
}

func invokeFallbackCheck(name, pattern string, paths []string) check {
	compiled, err := regexp.Compile(pattern)
	if err != nil {
		return check{Name: name, Status: "error", Matches: []string{err.Error()}}
	}
	lines := make([]string, 0)
	for _, path := range paths {
		info, err := os.Stat(path)
		if err != nil {
			continue
		}
		if info.IsDir() {
			_ = filepath.Walk(path, func(filePath string, fileInfo os.FileInfo, walkErr error) error {
				if walkErr != nil || fileInfo == nil {
					return nil
				}
				if fileInfo.IsDir() {
					return nil
				}
				if excludedPath(filePath) {
					return nil
				}
				lines = append(lines, matchingLines(filePath, compiled)...)
				return nil
			})
		} else if !excludedPath(path) {
			lines = append(lines, matchingLines(path, compiled)...)
		}
	}
	lines = truncate(lines)
	if len(lines) == 0 {
		return check{Name: name, Status: "pass", Matches: []string{}}
	}
	return check{Name: name, Status: "blocked", Matches: lines}
}

func matchingLines(path string, pattern *regexp.Regexp) []string {
	content, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	lines := strings.Split(strings.ReplaceAll(string(content), "\r\n", "\n"), "\n")
	matches := make([]string, 0)
	for index, line := range lines {
		if pattern.MatchString(line) {
			matches = append(matches, fmt.Sprintf("%s:%d:%s", filepath.ToSlash(path), index+1, line))
		}
	}
	return matches
}

func excludedPath(path string) bool {
	normalized := "/" + strings.Trim(filepath.ToSlash(path), "/") + "/"
	return strings.Contains(normalized, "/tests/") || strings.Contains(normalized, "/generated/") || strings.Contains(normalized, "/adapters/")
}

func combinedLines(stdout, stderr string) []string {
	combined := stdout
	if stderr != "" {
		if combined != "" && !strings.HasSuffix(combined, "\n") {
			combined += "\n"
		}
		combined += stderr
	}
	combined = strings.ReplaceAll(combined, "\r\n", "\n")
	if strings.TrimSpace(combined) == "" {
		return []string{}
	}
	return strings.Split(strings.TrimRight(combined, "\n"), "\n")
}

func truncate(lines []string) []string {
	if len(lines) <= 80 {
		return lines
	}
	result := append([]string{}, lines[:80]...)
	result = append(result, fmt.Sprintf("... omitted %d additional matches", len(lines)-80))
	return result
}

func exitCode(err error) int {
	if err == nil {
		return 0
	}
	var exitErr *exec.ExitError
	if errors.As(err, &exitErr) {
		return exitErr.ExitCode()
	}
	return -1
}
