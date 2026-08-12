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
	"strings"
	"time"
)

const defaultPath = "tests/fixtures/agent-evaluation-log/pass.md"

type checkIssue struct {
	Code        string `json:"code"`
	ErrorCode   string `json:"error_code"`
	BlockerCode string `json:"blocker_code"`
	Type        string `json:"type"`
	Message     string `json:"message"`
	Section     string `json:"section"`
	SourcePath  string `json:"source_path"`
	SourceField string `json:"source_field"`
}

type section struct {
	Date    string
	Title   string
	Heading string
	Body    string
}

type result struct {
	Surface      string       `json:"surface"`
	Status       string       `json:"status"`
	Path         string       `json:"path"`
	Mode         string       `json:"mode"`
	IssueCount   int          `json:"issue_count"`
	Issues       []checkIssue `json:"issues"`
	BlockerCodes []string     `json:"blocker_codes"`
	ErrorCode    any          `json:"error_code"`
	BlockerCode  any          `json:"blocker_code"`
	SourcePath   string       `json:"source_path"`
	SourceField  any          `json:"source_field"`
}

type options struct {
	path string
	cwd  string
	all  bool
	json bool
}

var (
	scorecardPattern    = regexp.MustCompile(`(?m)^## (\d{4}-\d{2}-\d{2}) - (.+)$`)
	updatedAtPattern    = regexp.MustCompile(`(?m)^updated_at:\s*(\S+)`)
	rfc3339Pattern      = regexp.MustCompile(`^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})(\.\d{1,7})?(Z|([+-])(\d{2}):(\d{2}))$`)
	zeroTestPattern     = regexp.MustCompile(`(?i)(running\s+0\s+tests|\b0\s+tests?\b|\b0\s+passed\b)`)
	shrinkPattern       = regexp.MustCompile(`(?i)(proof_count_shrinkage|under[- ]?run|unexpected\s+test[- ]count\s+shrinkage)`)
	substitutionPattern = regexp.MustCompile(`(?i)(omitted_declared_proof|declared proof omitted|command substitution|substituted command)`)
	placeholderPattern  = regexp.MustCompile(`(?i)\bpending\b|\btbd\b|\bto be filled\b|\bto be added\b`)
)

func main() {
	if err := run(os.Args[1:], os.Stdout, mustWorkingDirectory()); err != nil {
		if errors.Is(err, errBlocked) {
			os.Exit(1)
		}
		_, _ = fmt.Fprintf(os.Stderr, "[check-agent-evaluation-log] ERROR: %s\n", err)
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
	evaluationCwd := cwd
	if strings.TrimSpace(opts.cwd) != "" {
		evaluationCwd = opts.cwd
	}
	checked, err := evaluate(opts.path, opts.all, evaluationCwd)
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
	if checked.IssueCount == 0 {
		_, err = fmt.Fprintln(output, "check-agent-evaluation-log: pass")
		return err
	}
	if _, err = fmt.Fprintln(output, "check-agent-evaluation-log: blocked"); err != nil {
		return err
	}
	for _, issue := range checked.Issues {
		prefix := ""
		if strings.TrimSpace(issue.Section) != "" {
			prefix = issue.Section + ": "
		}
		if _, err = fmt.Fprintf(output, "  - %s%s: %s\n", prefix, issue.Code, issue.Message); err != nil {
			return err
		}
	}
	return errBlocked
}

func parseOptions(args []string) (options, error) {
	set := flag.NewFlagSet("check-agent-evaluation-log", flag.ContinueOnError)
	set.SetOutput(io.Discard)
	opts := options{path: defaultPath}
	set.StringVar(&opts.path, "path", opts.path, "evaluation log path")
	set.StringVar(&opts.cwd, "cwd", "", "base directory for relative paths")
	set.BoolVar(&opts.all, "all", false, "validate all scorecards")
	set.BoolVar(&opts.json, "json", false, "emit JSON")
	if err := set.Parse(args); err != nil {
		return options{}, err
	}
	if set.NArg() != 0 {
		return options{}, fmt.Errorf("unexpected argument: %s", set.Arg(0))
	}
	return opts, nil
}

func evaluate(path string, all bool, cwd string) (result, error) {
	checked := result{
		Surface:      "check-agent-evaluation-log",
		Path:         path,
		Mode:         "latest_scorecard",
		Issues:       []checkIssue{},
		BlockerCodes: []string{},
		SourcePath:   path,
	}
	if all {
		checked.Mode = "all_scorecards"
	}
	fullPath, err := resolvePath(path, cwd)
	if err != nil {
		return result{}, err
	}
	content, err := os.ReadFile(fullPath)
	if errors.Is(err, os.ErrNotExist) {
		checked.Issues = append(checked.Issues, newIssue("missing_file", fmt.Sprintf("File not found: %s", path), "", "", ""))
		return finalize(checked), nil
	}
	if err != nil {
		return result{}, err
	}
	sections := scorecardSections(string(content))
	if len(sections) == 0 {
		checked.Issues = append(checked.Issues, newIssue("missing_scorecards", "No dated scorecard sections found.", "", "", ""))
		return finalize(checked), nil
	}
	updated := updatedAtPattern.FindStringSubmatch(string(content))
	if len(updated) == 0 {
		checked.Issues = append(checked.Issues, newIssue("missing_updated_at", "Footer metadata must contain updated_at.", "", path, "updated_at"))
	} else {
		latestDate, parseErr := time.Parse("2006-01-02", sections[len(sections)-1].Date)
		if parseErr != nil {
			return result{}, parseErr
		}
		updatedDate, errorCode, parseMessage := sourceCalendarDate(updated[1])
		if errorCode != "" {
			checked.Issues = append(checked.Issues, newIssue(errorCode, parseMessage, "", path, "updated_at"))
		} else if updatedDate.Before(latestDate) {
			checked.Issues = append(checked.Issues, newIssue("stale_updated_at", fmt.Sprintf("updated_at '%s' is older than latest scorecard date '%s'.", updated[1], sections[len(sections)-1].Date), "", path, "updated_at"))
		}
	}
	toCheck := sections[len(sections)-1:]
	if all {
		toCheck = sections
	}
	for _, item := range toCheck {
		checked.Issues = append(checked.Issues, validateSection(item)...)
	}
	return finalize(checked), nil
}

func resolvePath(path, cwd string) (string, error) {
	if filepath.IsAbs(path) {
		return filepath.Abs(path)
	}
	return filepath.Abs(filepath.Join(cwd, path))
}

func finalize(checked result) result {
	if len(checked.Issues) == 0 {
		checked.Status = "pass"
	} else {
		checked.Status = "blocked"
	}
	checked.IssueCount = len(checked.Issues)
	seen := map[string]bool{}
	checked.BlockerCodes = checked.BlockerCodes[:0]
	for _, issue := range checked.Issues {
		if !seen[issue.BlockerCode] {
			seen[issue.BlockerCode] = true
			checked.BlockerCodes = append(checked.BlockerCodes, issue.BlockerCode)
		}
	}
	if len(checked.Issues) == 1 {
		checked.ErrorCode = checked.Issues[0].ErrorCode
		checked.BlockerCode = checked.Issues[0].BlockerCode
		if checked.Issues[0].SourceField != "" {
			checked.SourceField = checked.Issues[0].SourceField
		}
	} else if len(checked.Issues) > 1 {
		checked.ErrorCode = "evaluation_log_blocked"
		checked.BlockerCode = "evaluation_log_blocked"
	}
	return checked
}

func newIssue(code, message, sectionName, sourcePath, sourceField string) checkIssue {
	return checkIssue{Code: code, ErrorCode: code, BlockerCode: code, Type: "blocker", Message: message, Section: sectionName, SourcePath: sourcePath, SourceField: sourceField}
}

func scorecardSections(content string) []section {
	matches := scorecardPattern.FindAllStringSubmatchIndex(content, -1)
	sections := make([]section, 0, len(matches))
	for i, match := range matches {
		start := match[0]
		bodyStart := match[1]
		end := len(content)
		if i+1 < len(matches) {
			end = matches[i+1][0]
		}
		date := content[match[2]:match[3]]
		title := strings.TrimSpace(content[match[4]:match[5]])
		sections = append(sections, section{Date: date, Title: title, Heading: strings.TrimSpace(content[start:bodyStart]), Body: content[bodyStart:end]})
	}
	return sections
}

func blockAfterHeading(body, heading string, stops []string) (string, bool) {
	lowerBody := strings.ToLower(body)
	start := strings.Index(lowerBody, strings.ToLower(heading))
	if start < 0 {
		return "", false
	}
	contentStart := start + len(heading)
	end := len(body)
	for _, stop := range stops {
		candidate := strings.Index(strings.ToLower(body[contentStart:]), strings.ToLower(stop))
		if candidate >= 0 && contentStart+candidate < end {
			end = contentStart + candidate
		}
	}
	return strings.TrimSpace(body[contentStart:end]), true
}

func proofFieldValue(block, field string) (string, bool) {
	fieldPattern := regexp.MustCompile(`^(\s*)-\s*` + regexp.QuoteMeta(field) + `\s*:\s*(.*)$`)
	lines := strings.Split(block, "\n")
	for i, line := range lines {
		match := fieldPattern.FindStringSubmatch(line)
		if len(match) == 0 {
			continue
		}
		values := []string{match[2]}
		nextField := regexp.MustCompile(`^` + regexp.QuoteMeta(match[1]) + `-\s+\S[^:\r\n]*\s*:`)
		for j := i + 1; j < len(lines); j++ {
			if nextField.MatchString(lines[j]) {
				break
			}
			values = append(values, lines[j])
		}
		return strings.TrimSpace(strings.Join(values, "\n")), true
	}
	return "", false
}

func sourceCalendarDate(timestamp string) (time.Time, string, string) {
	if strings.TrimSpace(timestamp) == "" {
		return time.Time{}, "updated_at_invalid_format", "updated_at must be an RFC3339 timestamp with an explicit timezone offset."
	}
	match := rfc3339Pattern.FindStringSubmatch(timestamp)
	if len(match) == 0 {
		return time.Time{}, "updated_at_invalid_format", fmt.Sprintf("updated_at '%s' must be RFC3339 with T separator, optional fraction, and explicit Z or +/-HH:mm offset.", timestamp)
	}
	if match[4] != "Z" {
		hour := atoi(match[6])
		minute := atoi(match[7])
		if hour > 14 || minute > 59 || (hour == 14 && minute != 0) {
			return time.Time{}, "updated_at_invalid_offset", fmt.Sprintf("updated_at '%s' has an invalid timezone offset '%s'.", timestamp, match[4])
		}
	}
	parsed, err := time.Parse("2006-01-02T15:04:05", timestamp[:19])
	if err != nil {
		return time.Time{}, "updated_at_invalid_value", fmt.Sprintf("updated_at '%s' is not a valid calendar timestamp.", timestamp)
	}
	return parsed, "", ""
}

func atoi(value string) int {
	var number int
	for _, runeValue := range value {
		number = number*10 + int(runeValue-'0')
	}
	return number
}

func validateSection(item section) []checkIssue {
	issues := make([]checkIssue, 0)
	add := func(code, message string) { issues = append(issues, newIssue(code, message, item.Heading, "", "")) }
	body := item.Body
	required := []string{"Proof:", "Post-Task Self-Analysis:", "Twenty criteria outcome:", "Implementation follow-up tasks:", "PR / issue processing:", "Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:"}
	for _, heading := range required {
		if !strings.Contains(strings.ToLower(body), strings.ToLower(heading)) {
			add("missing_heading", fmt.Sprintf("Missing required heading '%s'.", heading))
		}
	}
	proofStops := []string{"Executor / validator:", "Post-Task Self-Analysis:", "Twenty criteria outcome:", "Implementation follow-up tasks:", "PR / issue processing:", "Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----"}
	if proof, ok := blockAfterHeading(body, "Proof:", proofStops); ok {
		hasRationale := regexp.MustCompile(`(?i)\b(rationale|no_task_reason|zero_tests_expected|expected_zero_tests)\b`).MatchString(proof)
		if zeroTestPattern.MatchString(proof) && !hasRationale {
			add("zero_test_proof", "Proof block reports zero tests without zero_tests_expected, no_task_reason, or rationale.")
		}
		if shrinkPattern.MatchString(proof) && !regexp.MustCompile(`(?i)\brationale\s*:`).MatchString(proof) {
			add("proof_count_shrinkage_without_rationale", "Proof block reports test-count shrinkage or under-run without rationale.")
		}
		declared, declaredOK := proofFieldValue(proof, "declared_proof")
		executed, executedOK := proofFieldValue(proof, "executed_proof")
		if declaredOK && executedOK && declared != executed && !regexp.MustCompile(`(?i)\brationale\s*:`).MatchString(proof) {
			add("declared_executed_proof_mismatch", "declared_proof and executed_proof differ without rationale.")
		}
		if substitutionPattern.MatchString(proof) && !regexp.MustCompile(`(?i)\brationale\s*:`).MatchString(proof) {
			add("proof_substitution_without_rationale", "Proof block reports omitted/substituted proof command without rationale.")
		}
	}
	for _, field := range []string{"Worked", "Waste", "Risk", "Next change", "Docs update", "workflow_score_10"} {
		pattern := regexp.MustCompile(`(?mi)^\s*-\s*` + regexp.QuoteMeta(field) + `\s*:`)
		if !pattern.MatchString(body) {
			add("missing_base_field", fmt.Sprintf("Missing Post-Task Self-Analysis base field '%s:'.", field))
		}
	}
	for number := 1; number <= 20; number++ {
		if !regexp.MustCompile(fmt.Sprintf(`(?m)^\s*%d\.`, number)).MatchString(body) {
			add("missing_fixed_criterion", fmt.Sprintf("Missing fixed criterion '%d.'.", number))
		}
	}
	dynamicStops := []string{"Meta-analysis remediation:", "Next-task selection rule:", "-----"}
	dynamic, dynamicOK := blockAfterHeading(body, "Final dynamic criteria STOP point:", dynamicStops)
	if !dynamicOK || !regexp.MustCompile(`(?m)^\s*1\.`).MatchString(dynamic) {
		add("missing_dynamic_criterion", "Final dynamic criteria block must contain at least one numbered criterion.")
	} else if !regexp.MustCompile(`(?is)Evidence\s+source:`).MatchString(dynamic) {
		add("missing_dynamic_evidence", "Final dynamic criteria block must name an evidence source.")
	}
	implementationStops := []string{"PR / issue processing:", "Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----"}
	if implementation, ok := blockAfterHeading(body, "Implementation follow-up tasks:", implementationStops); ok {
		hasTaskRef := regexp.MustCompile("`[a-z][a-z0-9]+(?:-[a-z0-9]+)+`").MatchString(implementation)
		hasNoTaskReason := regexp.MustCompile(`(?i)\bno_task_reason\b`).MatchString(implementation)
		if !hasTaskRef && !hasNoTaskReason {
			add("missing_implementation_task_ref", "Implementation follow-up tasks must cite a TaskFlow task id or explicit no_task_reason.")
		}
	}
	prStops := []string{"Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----"}
	if prBlock, ok := blockAfterHeading(body, "PR / issue processing:", prStops); ok {
		openField := regexp.MustCompile(`(?mi)^\s*-\s*open_prs\s*:`)
		if !openField.MatchString(prBlock) {
			add("missing_open_pr_processing", "PR / issue processing must record open_prs state.")
		} else if !regexp.MustCompile(`(?i)(open_prs\s*:\s*(no_open_prs|processed|not_applicable|no_task_reason)|open_prs\s*:.*left_open_reason)`).MatchString(prBlock) {
			add("invalid_open_pr_processing", "open_prs must be processed, no_open_prs, not_applicable, no_task_reason, or include left_open_reason.")
		}
		processedField := regexp.MustCompile(`(?mi)^\s*-\s*processed_issues\s*:`)
		if !processedField.MatchString(prBlock) {
			add("missing_processed_issue_closure", "PR / issue processing must record processed_issues closure state.")
		} else if !regexp.MustCompile(`(?i)(processed_issues\s*:\s*(no_processed_issues|closed|not_applicable|no_task_reason)|processed_issues\s*:.*(left_open_reason|kept_open_reason))`).MatchString(prBlock) {
			add("invalid_processed_issue_closure", "processed_issues must be closed, no_processed_issues, not_applicable, no_task_reason, or include a kept-open reason.")
		}
	}
	if placeholderPattern.MatchString(body) {
		add("stale_placeholder", "Scorecard contains a stale placeholder such as pending, TBD, or to be added.")
	}
	return issues
}
