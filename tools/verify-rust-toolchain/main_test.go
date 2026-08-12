package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

type fakeRunner struct {
	rustcOutput      string
	rustcErr         error
	activeToolchain  string
	rustupErr        error
	rootRustVersion  string
	modelRustVersion string
	metadataErr      error
	invalidMetadata  bool
}

func (runner fakeRunner) run(_ context.Context, executable string, args []string, _ string) ([]byte, error) {
	name := strings.TrimSuffix(filepath.Base(executable), ".exe")
	switch name {
	case "rustc":
		return []byte(runner.rustcOutput), runner.rustcErr
	case "rustup":
		return []byte(runner.activeToolchain), runner.rustupErr
	case "cargo":
		if runner.metadataErr != nil {
			return nil, runner.metadataErr
		}
		if runner.invalidMetadata {
			return []byte("{"), nil
		}
		manifest := strings.Join(args, " ")
		version := runner.rootRustVersion
		if strings.Contains(manifest, "tests") {
			version = runner.modelRustVersion
		}
		return []byte(fmt.Sprintf(`{"packages":[{"name":"fixture","rust_version":%q}]}`, version)), nil
	default:
		return nil, errors.New("unexpected fake executable")
	}
}

func fixtureRoot(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	write := func(path, content string) {
		t.Helper()
		if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	write(filepath.Join(root, "Cargo.toml"), "[workspace]\n")
	if err := os.MkdirAll(filepath.Join(root, "tests", "model"), 0o755); err != nil {
		t.Fatal(err)
	}
	write(filepath.Join(root, "tests", "model", "Cargo.toml"), "[workspace]\n")
	return root
}

func fixtureTools() toolPaths {
	return toolPaths{rustc: "rustc", rustup: "rustup", cargo: "cargo"}
}

func passingRunner() fakeRunner {
	return fakeRunner{
		rustcOutput:      "rustc 1.97.1 (fixture)\n",
		activeToolchain:  "1.97.1-x86_64-pc-windows-msvc (overridden)\n",
		rootRustVersion:  "1.97.1",
		modelRustVersion: "1.97.1",
	}
}

func TestVerifySuccess(t *testing.T) {
	root := fixtureRoot(t)
	result, err := verify(context.Background(), root, "1.97.1", fixtureTools(), passingRunner())
	if err != nil {
		t.Fatalf("verify() error = %v", err)
	}
	if result.Status != "pass" || result.PackageCount != 2 {
		t.Fatalf("unexpected result: %+v", result)
	}
}

func TestVerifyRejectsOlderRustc(t *testing.T) {
	runner := passingRunner()
	runner.rustcOutput = "rustc 1.97.0 (fixture)\n"
	_, err := verify(context.Background(), fixtureRoot(t), "1.97.1", fixtureTools(), runner)
	if err == nil || !strings.Contains(err.Error(), "below required minimum") {
		t.Fatalf("expected minimum-version error, got %v", err)
	}
}

func TestVerifyRejectsPackageMismatch(t *testing.T) {
	runner := passingRunner()
	runner.modelRustVersion = "1.97.0"
	_, err := verify(context.Background(), fixtureRoot(t), "1.97.1", fixtureTools(), runner)
	if err == nil || !strings.Contains(err.Error(), "Package rust-version mismatch") {
		t.Fatalf("expected package mismatch error, got %v", err)
	}
}

func TestVerifyRejectsInvalidMetadata(t *testing.T) {
	runner := passingRunner()
	runner.invalidMetadata = true
	_, err := verify(context.Background(), fixtureRoot(t), "1.97.1", fixtureTools(), runner)
	if err == nil || !strings.Contains(err.Error(), "invalid JSON") {
		t.Fatalf("expected metadata JSON error, got %v", err)
	}
}

func TestVerifyRejectsToolFailures(t *testing.T) {
	runner := passingRunner()
	runner.rustupErr = errors.New("rustup unavailable")
	_, err := verify(context.Background(), fixtureRoot(t), "1.97.1", fixtureTools(), runner)
	if err == nil || !strings.Contains(err.Error(), "active rustup toolchain") {
		t.Fatalf("expected rustup error, got %v", err)
	}
}

func TestParseRustcVersion(t *testing.T) {
	version, err := parseRustcVersion("rustc 1.97.1 (fixture)")
	if err != nil || version != "1.97.1" {
		t.Fatalf("parseRustcVersion() = %q, %v", version, err)
	}
	if _, err := parseRustcVersion("not rustc"); err == nil {
		t.Fatal("expected invalid rustc output to fail")
	}
}

func TestWriteResultFormats(t *testing.T) {
	result := verificationResult{
		Status:          "pass",
		RequiredMinimum: "1.97.1",
		Rustc:           "rustc 1.97.1 (fixture)",
		ActiveToolchain: "fixture-toolchain",
		PackageCount:    2,
	}
	var text bytes.Buffer
	if err := writeResult(&text, result, "text", "bash"); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(text.String(), "minimum=1.97.1") {
		t.Fatalf("unexpected Bash output: %q", text.String())
	}
	var jsonOutput bytes.Buffer
	if err := writeResult(&jsonOutput, result, "json", "powershell"); err != nil {
		t.Fatal(err)
	}
	var decoded verificationResult
	if err := json.Unmarshal(jsonOutput.Bytes(), &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.PackageCount != 2 || decoded.RequiredMinimum != "1.97.1" {
		t.Fatalf("unexpected JSON output: %+v", decoded)
	}
}
