param(
    [string]$Path = "docs/process/agent-model-evaluation-log.md",
    [switch]$All,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function New-CheckIssue {
    param(
        [string]$Code,
        [string]$Message,
        [string]$Section = ""
    )

    [pscustomobject]@{
        code = $Code
        message = $Message
        section = $Section
    }
}

function Get-BlockAfterHeading {
    param(
        [string]$Body,
        [string]$Heading,
        [string[]]$StopHeadings
    )

    $comparison = [System.StringComparison]::OrdinalIgnoreCase
    $start = $Body.IndexOf($Heading, $comparison)
    if ($start -lt 0) {
        return $null
    }

    $contentStart = $start + $Heading.Length
    $end = $Body.Length
    foreach ($stopHeading in $StopHeadings) {
        $candidate = $Body.IndexOf($stopHeading, $contentStart, $comparison)
        if ($candidate -ge 0 -and $candidate -lt $end) {
            $end = $candidate
        }
    }

    return $Body.Substring($contentStart, $end - $contentStart).Trim()
}

function Get-ProofFieldValue {
    param(
        [string]$ProofBlock,
        [string]$FieldName
    )

    $escapedFieldName = [regex]::Escape($FieldName)
    $pattern = "(?ms)^\s*-\s*$escapedFieldName\s*:\s*(?<value>.*?)(?=^\s*-\s+\S|\z)"
    $match = [regex]::Match($ProofBlock, $pattern)
    if (-not $match.Success) {
        return $null
    }

    return $match.Groups["value"].Value.Trim()
}

function Get-ScorecardSections {
    param([string]$Content)

    $matches = [regex]::Matches($Content, "(?m)^## (?<date>\d{4}-\d{2}-\d{2}) - (?<title>.+)$")
    $sections = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $matches.Count; $i++) {
        $match = $matches[$i]
        $start = $match.Index
        $bodyStart = $match.Index + $match.Length
        $end = if ($i + 1 -lt $matches.Count) { $matches[$i + 1].Index } else { $Content.Length }
        $sections.Add([pscustomobject]@{
            date = $match.Groups["date"].Value
            title = $match.Groups["title"].Value.Trim()
            heading = $match.Value.Trim()
            body = $Content.Substring($bodyStart, $end - $bodyStart)
        })
    }
    return $sections
}

function Test-ScorecardSection {
    param([object]$Section)

    $issues = New-Object System.Collections.Generic.List[object]
    $sectionName = $Section.heading
    $body = $Section.body

    $requiredHeadings = @(
        "Proof:",
        "Post-Task Self-Analysis:",
        "Twenty criteria outcome:",
        "Implementation follow-up tasks:",
        "PR / issue processing:",
        "Final dynamic criteria STOP point:",
        "Meta-analysis remediation:",
        "Next-task selection rule:"
    )

    foreach ($heading in $requiredHeadings) {
        if ($body.IndexOf($heading, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
            $issues.Add((New-CheckIssue "missing_heading" "Missing required heading '$heading'." $sectionName))
        }
    }

    $proofBlock = Get-BlockAfterHeading $body "Proof:" @("Executor / validator:", "Post-Task Self-Analysis:", "Twenty criteria outcome:", "Implementation follow-up tasks:", "PR / issue processing:", "Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----")
    if ($null -ne $proofBlock) {
        $proofHasRationale = $proofBlock -match '(?i)\b(rationale|no_task_reason|zero_tests_expected|expected_zero_tests)\b'
        if ($proofBlock -match '(?i)(\brunning\s+0\s+tests\b|\b0\s+tests?\b|\b0\s+passed\b)' -and -not $proofHasRationale) {
            $issues.Add((New-CheckIssue "zero_test_proof" "Proof block reports zero tests without zero_tests_expected, no_task_reason, or rationale." $sectionName))
        }

        if ($proofBlock -match '(?i)(proof_count_shrinkage|under[- ]?run|unexpected\s+test[- ]count\s+shrinkage)' -and $proofBlock -notmatch '(?i)\brationale\s*:') {
            $issues.Add((New-CheckIssue "proof_count_shrinkage_without_rationale" "Proof block reports test-count shrinkage or under-run without rationale." $sectionName))
        }

        $declaredProof = Get-ProofFieldValue $proofBlock "declared_proof"
        $executedProof = Get-ProofFieldValue $proofBlock "executed_proof"
        if ($null -ne $declaredProof -and $null -ne $executedProof) {
            if ($declaredProof -ne $executedProof -and $proofBlock -notmatch '(?i)\brationale\s*:') {
                $issues.Add((New-CheckIssue "declared_executed_proof_mismatch" "declared_proof and executed_proof differ without rationale." $sectionName))
            }
        }

        if ($proofBlock -match '(?i)(omitted_declared_proof|declared proof omitted|command substitution|substituted command)' -and $proofBlock -notmatch '(?i)\brationale\s*:') {
            $issues.Add((New-CheckIssue "proof_substitution_without_rationale" "Proof block reports omitted/substituted proof command without rationale." $sectionName))
        }
    }

    $baseFields = @("Worked", "Waste", "Risk", "Next change", "Docs update", "workflow_score_10")
    foreach ($field in $baseFields) {
        if ($body -notmatch "(?mi)^-\s*$([regex]::Escape($field))\s*:") {
            $issues.Add((New-CheckIssue "missing_base_field" "Missing Post-Task Self-Analysis base field '${field}:'." $sectionName))
        }
    }

    for ($number = 1; $number -le 20; $number++) {
        if ($body -notmatch "(?m)^\s*$number\.") {
            $issues.Add((New-CheckIssue "missing_fixed_criterion" "Missing fixed criterion '$number.'." $sectionName))
        }
    }

    $dynamicBlock = Get-BlockAfterHeading $body "Final dynamic criteria STOP point:" @("Meta-analysis remediation:", "Next-task selection rule:", "-----")
    if ($null -eq $dynamicBlock -or $dynamicBlock -notmatch "(?m)^\s*1\.") {
        $issues.Add((New-CheckIssue "missing_dynamic_criterion" "Final dynamic criteria block must contain at least one numbered criterion." $sectionName))
    } elseif ($dynamicBlock -notmatch "(?is)Evidence\s+source:") {
        $issues.Add((New-CheckIssue "missing_dynamic_evidence" "Final dynamic criteria block must name an evidence source." $sectionName))
    }

    $implementationBlock = Get-BlockAfterHeading $body "Implementation follow-up tasks:" @("PR / issue processing:", "Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----")
    if ($null -ne $implementationBlock) {
        $hasTaskRef = $implementationBlock -match '`[a-z][a-z0-9]+(?:-[a-z0-9]+)+`'
        $hasNoTaskReason = $implementationBlock -match '(?i)\bno_task_reason\b'
        if (-not $hasTaskRef -and -not $hasNoTaskReason) {
            $issues.Add((New-CheckIssue "missing_implementation_task_ref" "Implementation follow-up tasks must cite a TaskFlow task id or explicit no_task_reason." $sectionName))
        }
    }

    $prIssueBlock = Get-BlockAfterHeading $body "PR / issue processing:" @("Final dynamic criteria STOP point:", "Meta-analysis remediation:", "Next-task selection rule:", "-----")
    if ($null -ne $prIssueBlock) {
        if ($prIssueBlock -notmatch "(?mi)^\s*-\s*open_prs\s*:") {
            $issues.Add((New-CheckIssue "missing_open_pr_processing" "PR / issue processing must record open_prs state." $sectionName))
        } elseif ($prIssueBlock -notmatch "(?i)(open_prs\s*:\s*(no_open_prs|processed|not_applicable|no_task_reason)|open_prs\s*:.*left_open_reason)") {
            $issues.Add((New-CheckIssue "invalid_open_pr_processing" "open_prs must be processed, no_open_prs, not_applicable, no_task_reason, or include left_open_reason." $sectionName))
        }

        if ($prIssueBlock -notmatch "(?mi)^\s*-\s*processed_issues\s*:") {
            $issues.Add((New-CheckIssue "missing_processed_issue_closure" "PR / issue processing must record processed_issues closure state." $sectionName))
        } elseif ($prIssueBlock -notmatch "(?i)(processed_issues\s*:\s*(no_processed_issues|closed|not_applicable|no_task_reason)|processed_issues\s*:.*(left_open_reason|kept_open_reason))") {
            $issues.Add((New-CheckIssue "invalid_processed_issue_closure" "processed_issues must be closed, no_processed_issues, not_applicable, no_task_reason, or include a kept-open reason." $sectionName))
        }
    }

    if ($body -match "(?i)(?<![-\w])pending(?![-\w])|\btbd\b|\bto be filled\b|\bto be added\b") {
        $issues.Add((New-CheckIssue "stale_placeholder" "Scorecard contains a stale placeholder such as pending, TBD, or to be added." $sectionName))
    }

    return $issues
}

$issues = New-Object System.Collections.Generic.List[object]
$fullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))

if (-not (Test-Path -LiteralPath $fullPath)) {
    $issues.Add((New-CheckIssue "missing_file" "File not found: $Path"))
} else {
    $content = Get-Content -LiteralPath $fullPath -Raw
    $sections = @(Get-ScorecardSections $content)
    if ($sections.Count -eq 0) {
        $issues.Add((New-CheckIssue "missing_scorecards" "No dated scorecard sections found."))
    } else {
        $updatedAtMatch = [regex]::Match($content, "(?m)^updated_at:\s*(?<value>\S+)")
        if (-not $updatedAtMatch.Success) {
            $issues.Add((New-CheckIssue "missing_updated_at" "Footer metadata must contain updated_at."))
        } else {
            $latestSectionDate = [datetime]::ParseExact($sections[-1].date, "yyyy-MM-dd", [System.Globalization.CultureInfo]::InvariantCulture)
            $updatedDateText = $updatedAtMatch.Groups["value"].Value
            $updatedDate = [datetime]::Parse($updatedDateText, [System.Globalization.CultureInfo]::InvariantCulture)
            if ($updatedDate.Date -lt $latestSectionDate.Date) {
                $issues.Add((New-CheckIssue "stale_updated_at" "updated_at '$updatedDateText' is older than latest scorecard date '$($sections[-1].date)'."))
            }
        }

        $sectionsToCheck = if ($All) { $sections } else { @($sections[-1]) }
        foreach ($section in $sectionsToCheck) {
            foreach ($issue in (Test-ScorecardSection $section)) {
                $issues.Add($issue)
            }
        }
    }
}

$status = if ($issues.Count -eq 0) { "pass" } else { "blocked" }
$checkMode = "latest_scorecard"
if ($All) {
    $checkMode = "all_scorecards"
}
$result = New-Object PSObject
$result | Add-Member -NotePropertyName surface -NotePropertyValue "check-agent-evaluation-log"
$result | Add-Member -NotePropertyName status -NotePropertyValue $status
$result | Add-Member -NotePropertyName path -NotePropertyValue $Path
$result | Add-Member -NotePropertyName mode -NotePropertyValue $checkMode
$result | Add-Member -NotePropertyName issue_count -NotePropertyValue $issues.Count
$result | Add-Member -NotePropertyName issues -NotePropertyValue @($issues.ToArray())

if ($Json) {
    $result | ConvertTo-Json -Depth 6
} else {
    if ($issues.Count -eq 0) {
        Write-Output "check-agent-evaluation-log: pass"
    } else {
        Write-Output "check-agent-evaluation-log: blocked"
        foreach ($issue in $issues) {
            $prefix = if ([string]::IsNullOrWhiteSpace($issue.section)) { "" } else { "$($issue.section): " }
            Write-Output ("  - {0}{1}: {2}" -f $prefix, $issue.code, $issue.message)
        }
    }
}

if ($issues.Count -gt 0) {
    exit 1
}
