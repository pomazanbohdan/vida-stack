package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"strconv"
	"strings"
	"time"
)

const defaultMinimumVersion = "1.97.1"

var rustcVersionPattern = regexp.MustCompile(`^rustc ([0-9]+\.[0-9]+\.[0-9]+)`)

type options struct {
	root      string
	minimum   string
	format    string
	textStyle string
}

type rustVersion struct {
	major int
	minor int
	patch int
}

type toolPaths struct {
	rustc  string
	rustup string
	cargo  string
}

type verificationResult struct {
	Status          string `json:"status"`
	RequiredMinimum string `json:"required_minimum"`
	Rustc           string `json:"rustc"`
	ActiveToolchain string `json:"active_toolchain"`
	PackageCount    int    `json:"package_count"`
}

type cargoPackage struct {
	Name        string  `json:"name"`
	RustVersion *string `json:"rust_version"`
}

type cargoMetadata struct {
	Packages []cargoPackage `json:"packages"`
}

type commandRunner interface {
	run(context.Context, string, []string, string) ([]byte, error)
}

type execCommandRunner struct{}

func (execCommandRunner) run(ctx context.Context, executable string, args []string, dir string) ([]byte, error) {
	command := exec.CommandContext(ctx, executable, args...)
	command.Dir = dir
	return command.Output()
}

func main() {
	if err := run(os.Args[1:], os.Stdout); err != nil {
		_, _ = fmt.Fprintf(os.Stderr, "[rust-toolchain] ERROR: %s\n", err)
		os.Exit(1)
	}
}

func run(args []string, output io.Writer) error {
	opts, err := parseOptions(args)
	if err != nil {
		return err
	}

	root, err := discoverRepoRoot(opts.root)
	if err != nil {
		return err
	}
	tools, err := resolveTools()
	if err != nil {
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()
	result, err := verify(ctx, root, opts.minimum, tools, execCommandRunner{})
	if err != nil {
		return err
	}
	return writeResult(output, result, opts.format, opts.textStyle)
}

func parseOptions(args []string) (options, error) {
	flagSet := flag.NewFlagSet("verify-rust-toolchain", flag.ContinueOnError)
	flagSet.SetOutput(io.Discard)
	opts := options{
		minimum:   strings.TrimSpace(os.Getenv("VIDA_RUST_MINIMUM_VERSION")),
		format:    "text",
		textStyle: "powershell",
	}
	if opts.minimum == "" {
		opts.minimum = defaultMinimumVersion
	}
	jsonOutput := false
	flagSet.StringVar(&opts.root, "root", strings.TrimSpace(os.Getenv("VIDA_REPO_ROOT")), "repository root")
	flagSet.StringVar(&opts.minimum, "minimum-version", opts.minimum, "minimum Rust version")
	flagSet.StringVar(&opts.format, "format", opts.format, "output format: text or json")
	flagSet.StringVar(&opts.textStyle, "text-style", opts.textStyle, "text style: bash or powershell")
	flagSet.BoolVar(&jsonOutput, "json", false, "emit JSON output")
	if err := flagSet.Parse(args); err != nil {
		return options{}, err
	}
	if flagSet.NArg() != 0 {
		return options{}, fmt.Errorf("unexpected argument: %s", flagSet.Arg(0))
	}
	if jsonOutput {
		opts.format = "json"
	}
	opts.format = strings.ToLower(strings.TrimSpace(opts.format))
	if opts.format != "text" && opts.format != "json" {
		return options{}, fmt.Errorf("unsupported output format: %s", opts.format)
	}
	opts.textStyle = strings.ToLower(strings.TrimSpace(opts.textStyle))
	if opts.textStyle != "bash" && opts.textStyle != "powershell" {
		return options{}, fmt.Errorf("unsupported text style: %s", opts.textStyle)
	}
	if strings.TrimSpace(opts.minimum) == "" {
		return options{}, errors.New("minimum Rust version must not be empty")
	}
	return opts, nil
}

func discoverRepoRoot(configured string) (string, error) {
	starts := make([]string, 0, 2)
	if configured = strings.TrimSpace(configured); configured != "" {
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
	return "", errors.New("repository root not found; set VIDA_REPO_ROOT or --root")
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
		if isFile(filepath.Join(dir, "Cargo.toml")) {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return ""
		}
		dir = parent
	}
}

func resolveTools() (toolPaths, error) {
	rustc, err := resolveTool("rustc")
	if err != nil {
		return toolPaths{}, err
	}
	rustup, err := resolveTool("rustup")
	if err != nil {
		return toolPaths{}, err
	}
	cargo, err := resolveTool("cargo")
	if err != nil {
		return toolPaths{}, err
	}
	return toolPaths{rustc: rustc, rustup: rustup, cargo: cargo}, nil
}

func resolveTool(name string) (string, error) {
	envName := strings.ToUpper(name)
	if override := strings.TrimSpace(os.Getenv(envName)); override != "" {
		if resolved, ok := resolveCandidate(override); ok {
			return resolved, nil
		}
	}

	home, _ := os.UserHomeDir()
	if home == "" {
		home = strings.TrimSpace(os.Getenv("HOME"))
	}
	canonical := filepath.Join(home, ".cargo", "bin", toolFileName(name))
	if isExecutable(canonical) {
		return canonical, nil
	}
	if resolved, err := exec.LookPath(toolFileName(name)); err == nil {
		return resolved, nil
	}
	return "", fmt.Errorf("Unable to resolve %s from %s or PATH", name, canonical)
}

func resolveCandidate(candidate string) (string, bool) {
	if strings.ContainsAny(candidate, `/\\`) || filepath.IsAbs(candidate) {
		return candidate, isExecutable(candidate)
	}
	resolved, err := exec.LookPath(candidate)
	return resolved, err == nil
}

func toolFileName(name string) string {
	if runtime.GOOS == "windows" && !strings.HasSuffix(strings.ToLower(name), ".exe") {
		return name + ".exe"
	}
	return name
}

func isExecutable(path string) bool {
	info, err := os.Stat(path)
	if err != nil || info.IsDir() {
		return false
	}
	if runtime.GOOS == "windows" {
		return true
	}
	return info.Mode().Perm()&0o111 != 0
}

func verify(ctx context.Context, root, minimum string, tools toolPaths, runner commandRunner) (verificationResult, error) {
	minimumVersion, err := parseVersion(minimum)
	if err != nil {
		return verificationResult{}, fmt.Errorf("invalid minimum Rust version %q: %w", minimum, err)
	}

	rustcBytes, err := runner.run(ctx, tools.rustc, []string{"--version"}, root)
	if err != nil {
		return verificationResult{}, errors.New("rustc --version failed")
	}
	rustcOutput := firstLine(rustcBytes)
	actualVersionText, err := parseRustcVersion(rustcOutput)
	if err != nil {
		return verificationResult{}, fmt.Errorf("Unable to parse rustc version: %s", rustcOutput)
	}
	actualVersion, _ := parseVersion(actualVersionText)
	if actualVersion.lessThan(minimumVersion) {
		return verificationResult{}, fmt.Errorf("Rust %s is below required minimum %s", actualVersionText, minimum)
	}

	activeBytes, err := runner.run(ctx, tools.rustup, []string{"show", "active-toolchain"}, root)
	activeToolchain := firstLine(activeBytes)
	if err != nil || activeToolchain == "" {
		return verificationResult{}, errors.New("Unable to resolve the active rustup toolchain")
	}

	manifests := []string{filepath.Join(root, "Cargo.toml")}
	modelManifest := filepath.Join(root, "tests", "model", "Cargo.toml")
	if isFile(modelManifest) {
		manifests = append(manifests, modelManifest)
	}

	invalidPackages := make([]string, 0)
	packageCount := 0
	for _, manifest := range manifests {
		metadataBytes, err := runner.run(ctx, tools.cargo, []string{
			"metadata",
			"--manifest-path", manifest,
			"--no-deps",
			"--format-version", "1",
		}, root)
		if err != nil {
			return verificationResult{}, fmt.Errorf("cargo metadata failed for %s", manifest)
		}
		var metadata cargoMetadata
		if err := json.Unmarshal(metadataBytes, &metadata); err != nil {
			return verificationResult{}, fmt.Errorf("cargo metadata returned invalid JSON for %s", manifest)
		}
		for _, pkg := range metadata.Packages {
			packageCount++
			if pkg.RustVersion == nil || *pkg.RustVersion != minimum {
				invalidPackages = append(invalidPackages, pkg.Name)
			}
		}
	}
	if len(invalidPackages) > 0 {
		return verificationResult{}, fmt.Errorf("Package rust-version mismatch: %s. Expected %s.", strings.Join(invalidPackages, ", "), minimum)
	}

	return verificationResult{
		Status:          "pass",
		RequiredMinimum: minimum,
		Rustc:           rustcOutput,
		ActiveToolchain: activeToolchain,
		PackageCount:    packageCount,
	}, nil
}

func parseRustcVersion(output string) (string, error) {
	match := rustcVersionPattern.FindStringSubmatch(strings.TrimSpace(output))
	if len(match) != 2 {
		return "", errors.New("rustc version format is invalid")
	}
	return match[1], nil
}

func parseVersion(input string) (rustVersion, error) {
	parts := strings.Split(strings.TrimSpace(input), ".")
	if len(parts) != 3 {
		return rustVersion{}, errors.New("expected major.minor.patch")
	}
	values := make([]int, len(parts))
	for index, part := range parts {
		value, err := strconv.Atoi(part)
		if err != nil || value < 0 {
			return rustVersion{}, errors.New("version components must be non-negative integers")
		}
		values[index] = value
	}
	return rustVersion{major: values[0], minor: values[1], patch: values[2]}, nil
}

func (version rustVersion) lessThan(other rustVersion) bool {
	if version.major != other.major {
		return version.major < other.major
	}
	if version.minor != other.minor {
		return version.minor < other.minor
	}
	return version.patch < other.patch
}

func firstLine(output []byte) string {
	for _, line := range strings.Split(strings.ReplaceAll(string(output), "\r\n", "\n"), "\n") {
		if trimmed := strings.TrimSpace(line); trimmed != "" {
			return trimmed
		}
	}
	return ""
}

func writeResult(output io.Writer, result verificationResult, format, textStyle string) error {
	if format == "json" {
		return json.NewEncoder(output).Encode(result)
	}
	if textStyle == "bash" {
		_, err := fmt.Fprintf(output, "[rust-toolchain] pass: %s; minimum=%s\n", result.Rustc, result.RequiredMinimum)
		return err
	}
	_, err := fmt.Fprintf(output, "[rust-toolchain] pass: %s; active=%s; packages=%d\n", result.Rustc, result.ActiveToolchain, result.PackageCount)
	return err
}

func isFile(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}
