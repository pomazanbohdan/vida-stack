package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

type violation struct {
	Path   string `json:"path"`
	Rule   string `json:"rule"`
	Detail string `json:"detail"`
}

type result struct {
	Surface                       string         `json:"surface"`
	Status                        string         `json:"status"`
	ConfigPaths                   []string       `json:"config_paths"`
	ScannedSurfaceCount           int            `json:"scanned_surface_count"`
	ScannedSurfaceRoots           []string       `json:"scanned_surface_roots"`
	ScannedSurfaceFamilies        map[string]int `json:"scanned_surface_families"`
	AllowedConfigAndContractPaths []string       `json:"allowed_config_and_contract_paths"`
	ConfiguredValues              []string       `json:"configured_values"`
	Violations                    []violation    `json:"violations"`
}

type options struct {
	root     string
	json     bool
	selfTest bool
}

type familySpec struct {
	Name     string
	Relative string
	Patterns []string
}

type inventory struct {
	Paths           []string
	FamilyCounts    map[string]int
	MissingFamilies []string
}

const neutralitySurface = "tools/check-host-bridge-capability-neutrality"
const neutralityWorkflowMarker = "tools/check-host-bridge-capability-neutrality"

var legacyAliasPattern = regexp.MustCompile(`(?m)\b(spawn_tool|wait_tool|close_tool|dispose_tool)\b`)

func main() {
	if err := run(os.Args[1:], os.Stdout, mustWorkingDirectory()); err != nil {
		if errors.Is(err, errBlocked) {
			os.Exit(1)
		}
		_, _ = fmt.Fprintf(os.Stderr, "[check-host-bridge-capability-neutrality] ERROR: %s\n", err)
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
	inv := getSurfaceInventory(root)
	if opts.selfTest {
		if err := runSelfTest(inv); err != nil {
			return err
		}
		_, err := fmt.Fprintln(output, "host bridge capability neutrality self-test: pass")
		return err
	}
	checked := evaluate(root, inv)
	if opts.json {
		if err := json.NewEncoder(output).Encode(checked); err != nil {
			return err
		}
		if checked.Status != "pass" {
			return errBlocked
		}
		return nil
	}
	if _, err := fmt.Fprintf(output, "host bridge capability neutrality: %s\n", checked.Status); err != nil {
		return err
	}
	if _, err := fmt.Fprintf(output, "- scanned surfaces: %d\n", checked.ScannedSurfaceCount); err != nil {
		return err
	}
	if _, err := fmt.Fprintf(output, "- configured registry values: %d\n", len(checked.ConfiguredValues)); err != nil {
		return err
	}
	if _, err := fmt.Fprintf(output, "- violations: %d\n", len(checked.Violations)); err != nil {
		return err
	}
	for _, item := range checked.Violations {
		if _, err := fmt.Fprintf(output, "  %s: [%s] %s\n", item.Path, item.Rule, item.Detail); err != nil {
			return err
		}
	}
	if checked.Status != "pass" {
		return errBlocked
	}
	return nil
}

func parseOptions(args []string) (options, error) {
	set := flag.NewFlagSet("check-host-bridge-capability-neutrality", flag.ContinueOnError)
	set.SetOutput(io.Discard)
	opts := options{}
	set.StringVar(&opts.root, "root", "", "repository root")
	set.BoolVar(&opts.json, "json", false, "emit JSON")
	set.BoolVar(&opts.selfTest, "self-test", false, "run inventory self-test")
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
	return filepath.Abs(start)
}

func getSurfaceInventory(root string) inventory {
	specs := []familySpec{
		{Name: "crates", Relative: "crates", Patterns: []string{"*.rs"}},
		{Name: "scripts", Relative: "scripts", Patterns: []string{"*.ps1", "*.cmd", "*.sh"}},
		{Name: ".github/workflows", Relative: filepath.Join(".github", "workflows"), Patterns: []string{"*.yml", "*.yaml"}},
		{Name: "docs", Relative: "docs", Patterns: []string{"*generated*", "*.template.md", "*.template.yaml", "*.jsonl"}},
	}
	paths := make([]string, 0)
	counts := map[string]int{}
	missing := make([]string, 0)
	for _, spec := range specs {
		base := filepath.Join(root, spec.Relative)
		info, err := os.Stat(base)
		if err != nil || !info.IsDir() {
			counts[spec.Name] = 0
			missing = append(missing, spec.Name)
			continue
		}
		files := make([]string, 0)
		_ = filepath.Walk(base, func(path string, info os.FileInfo, walkErr error) error {
			if walkErr != nil || info == nil || info.IsDir() {
				return nil
			}
			if matchesAny(filepath.Base(path), spec.Patterns) {
				files = append(files, path)
			}
			return nil
		})
		sort.Strings(files)
		counts[spec.Name] = len(files)
		if len(files) == 0 {
			missing = append(missing, spec.Name)
		}
		paths = append(paths, files...)
	}
	sort.Strings(paths)
	return inventory{Paths: unique(paths), FamilyCounts: counts, MissingFamilies: missing}
}

func matchesAny(name string, patterns []string) bool {
	for _, pattern := range patterns {
		matched, err := filepath.Match(pattern, name)
		if err == nil && matched {
			return true
		}
	}
	return false
}

func unique(values []string) []string {
	result := make([]string, 0, len(values))
	seen := map[string]bool{}
	for _, value := range values {
		key := strings.ToLower(filepath.Clean(value))
		if !seen[key] {
			seen[key] = true
			result = append(result, value)
		}
	}
	return result
}

func runSelfTest(inv inventory) error {
	values := []string{"fixture.spawn", "fixture.wait", "fixture.dispose"}
	production := "adapter_operations: fixture.spawn"
	allowed := "operations: fixture.spawn"
	foundProduction := false
	foundAllowed := false
	for _, value := range values {
		if strings.Contains(production, value) {
			foundProduction = true
		}
		if strings.Contains(allowed, value) {
			foundAllowed = true
		}
	}
	if !foundProduction {
		return errors.New("self-test failed: production configured-value detection")
	}
	if !foundAllowed {
		return errors.New("self-test failed: allowed config detection")
	}
	expected := 0
	for _, count := range inv.FamilyCounts {
		expected += count
	}
	if len(inv.MissingFamilies) > 0 {
		return fmt.Errorf("self-test failed: missing surface family '%s'", strings.Join(inv.MissingFamilies, ", "))
	}
	if len(inv.Paths) != expected || len(inv.Paths) < 4 {
		return fmt.Errorf("self-test failed: flattened surface count '%d' does not match family inventory '%d'", len(inv.Paths), expected)
	}
	return nil
}

func evaluate(root string, inv inventory) result {
	configRelativePaths := []string{"vida.config.yaml", filepath.Join("docs", "framework", "templates", "vida.config.yaml.template")}
	configPaths := make([]string, 0, len(configRelativePaths))
	configuredValues := make([]string, 0)
	for _, relativePath := range configRelativePaths {
		path := filepath.Join(root, relativePath)
		if _, err := os.Stat(path); err != nil {
			continue
		}
		configPaths = append(configPaths, filepath.ToSlash(relativePath))
		configuredValues = append(configuredValues, getConfiguredAdapterValues(path)...)
	}
	sort.Slice(configuredValues, func(i, j int) bool { return powerShellLess(configuredValues[i], configuredValues[j]) })
	configuredValues = unique(configuredValues)
	allowed := existingPaths([]string{
		filepath.Join(root, "vida.config.yaml"),
		filepath.Join(root, "docs", "framework", "templates", "vida.config.yaml.template"),
		filepath.Join(root, "crates", "taskflow-host-bridge", "src", "adapter_contract.rs"),
		filepath.Join(root, "docs", "product", "spec", "host-agent-bridge-adapter-contract.md"),
	})
	violations := make([]violation, 0)
	for _, family := range inv.MissingFamilies {
		violations = append(violations, violation{Path: family, Rule: "surface_family_missing", Detail: "required neutrality scan family is absent or empty"})
	}
	expected := 0
	for _, count := range inv.FamilyCounts {
		expected += count
	}
	if len(inv.Paths) != expected || len(inv.Paths) < 4 {
		violations = append(violations, violation{Path: neutralitySurface, Rule: "surface_scan_count_too_small", Detail: "flattened scan count does not match family inventory"})
	}
	for _, path := range inv.Paths {
		normalized := filepath.ToSlash(path)
		if containsPath(allowed, path) || strings.Contains(normalized, "/tests/") {
			continue
		}
		text := getProductionText(path)
		for _, value := range configuredValues {
			if len(value) > 2 && strings.Contains(text, value) {
				violations = append(violations, violation{Path: normalized, Rule: "configured_value_in_production_surface", Detail: value})
			}
		}
		if !strings.HasSuffix(normalized, "/adapter_contract.rs") &&
			!strings.HasSuffix(normalized, "/taskflow-host-bridge/src/request.rs") &&
			legacyAliasPattern.MatchString(text) {
			violations = append(violations, violation{Path: normalized, Rule: "legacy_operation_alias_in_production_surface", Detail: "legacy lifecycle alias"})
		}
	}
	workflowPath := filepath.Join(root, ".github", "workflows", "runtime-quality.yml")
	if _, err := os.Stat(workflowPath); err == nil {
		if !strings.Contains(string(readFile(workflowPath)), neutralityWorkflowMarker) {
			violations = append(violations, violation{Path: filepath.ToSlash(filepath.Join(".github", "workflows", "runtime-quality.yml")), Rule: "workflow_gate_missing", Detail: "canonical Go neutrality binary is not built and invoked"})
		}
	}
	contractPath := filepath.Join(root, "docs", "product", "spec", "host-agent-bridge-adapter-contract.md")
	if _, err := os.Stat(contractPath); err == nil {
		contractText := string(readFile(contractPath))
		for _, required := range []string{"\"adapter_operations\"", "\"operations\"", "\"dispose_policy\"", "\"adapter_contract_hash\""} {
			if !strings.Contains(contractText, required) {
				violations = append(violations, violation{Path: filepath.ToSlash(filepath.Join("docs", "product", "spec", "host-agent-bridge-adapter-contract.md")), Rule: "contract_schema_missing", Detail: required})
			}
		}
	}
	status := "pass"
	if len(violations) > 0 {
		status = "blocked"
	}
	return result{
		Surface:                       neutralitySurface,
		Status:                        status,
		ConfigPaths:                   configPaths,
		ScannedSurfaceCount:           len(inv.Paths),
		ScannedSurfaceRoots:           []string{"crates", "scripts", ".github/workflows", "docs"},
		ScannedSurfaceFamilies:        inv.FamilyCounts,
		AllowedConfigAndContractPaths: allowed,
		ConfiguredValues:              configuredValues,
		Violations:                    violations,
	}
}

func getConfiguredAdapterValues(path string) []string {
	lines := strings.Split(string(readFile(path)), "\n")
	bridgeHeader := regexp.MustCompile(`^(\s*)host_tool_bridge:\s*$`)
	valuePattern := regexp.MustCompile(`^\s+(adapter_kind|adapter_capability_id|invocation_mode|spawn|wait|dispose):\s*([^#\s]+)`)
	inBridge := false
	bridgeIndent := -1
	values := make([]string, 0)
	for _, line := range lines {
		if match := bridgeHeader.FindStringSubmatch(line); len(match) > 0 {
			inBridge = true
			bridgeIndent = len(match[1])
			continue
		}
		if !inBridge {
			continue
		}
		leading := len(line) - len(strings.TrimLeft(line, " \t"))
		if strings.TrimSpace(line) != "" && leading <= bridgeIndent && !strings.HasPrefix(strings.TrimSpace(line), "#") {
			inBridge = false
			continue
		}
		if match := valuePattern.FindStringSubmatch(line); len(match) > 0 {
			value := strings.Trim(match[2], "\"'")
			if value != "" {
				values = append(values, value)
			}
		}
	}
	sort.Slice(values, func(i, j int) bool { return powerShellLess(values[i], values[j]) })
	return unique(values)
}

func powerShellLess(left, right string) bool {
	leftKey := powerShellSortKey(left)
	rightKey := powerShellSortKey(right)
	if leftKey == rightKey {
		return left < right
	}
	return leftKey < rightKey
}

func powerShellSortKey(value string) string {
	var key strings.Builder
	for _, runeValue := range strings.ToLower(value) {
		switch runeValue {
		case '_':
			key.WriteRune('\x01')
		case '.':
			key.WriteRune('\x02')
		case '-':
			key.WriteRune('\x03')
		default:
			key.WriteRune(runeValue)
		}
	}
	return key.String()
}

func getProductionText(path string) string {
	lines := strings.Split(string(readFile(path)), "\n")
	if strings.HasSuffix(strings.ToLower(path), ".rs") {
		for index, line := range lines {
			if line == "#[cfg(test)]" {
				lines = lines[:index]
				break
			}
		}
	}
	return strings.Join(lines, "\n")
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

func containsPath(paths []string, candidate string) bool {
	for _, path := range paths {
		if strings.EqualFold(filepath.Clean(path), filepath.Clean(candidate)) {
			return true
		}
	}
	return false
}

func readFile(path string) []byte {
	content, _ := os.ReadFile(path)
	return content
}
