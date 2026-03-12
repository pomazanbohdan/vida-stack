import stat
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
INSTALLER = REPO_ROOT / "install" / "install.sh"


class InstallDocflowBridgeTest(unittest.TestCase):
    def _install_wrappers(self, tmp_path: Path) -> tuple[Path, Path, Path]:
        install_root = tmp_path / "vida-home"
        bin_dir = tmp_path / "bin"
        vida_root = install_root / "current"
        donor_dir = vida_root / "codex-v0"
        python_bin = vida_root / ".venv" / "bin"

        donor_dir.mkdir(parents=True)
        python_bin.mkdir(parents=True)
        (donor_dir / "codex.py").write_text(
            "print('stub donor runtime')\n",
            encoding="utf-8",
        )

        command = textwrap.dedent(
            f"""\
            set -euo pipefail
            source <(sed '$d' {INSTALLER})
            INSTALL_ROOT={install_root}
            BIN_DIR={bin_dir}
            install_wrappers
            """
        )
        subprocess.run(
            ["bash", "-lc", command],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        return install_root, bin_dir, vida_root

    def _write_python_stub(self, tmp_path: Path, vida_root: Path) -> Path:
        argv_log = tmp_path / "docflow-argv.txt"
        python_stub = vida_root / ".venv" / "bin" / "python3"
        python_stub.write_text(
            textwrap.dedent(
                f"""\
                #!/usr/bin/env bash
                set -euo pipefail
                printf '%s\n' "$@" > {argv_log}
                """
            ),
            encoding="utf-8",
        )
        python_stub.chmod(python_stub.stat().st_mode | stat.S_IEXEC)
        return argv_log

    def test_vida_docflow_help_is_served_by_installed_wrapper(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root, bin_dir, vida_root = self._install_wrappers(tmp_path)
            argv_log = self._write_python_stub(tmp_path, vida_root)

            result = subprocess.run(
                ["bash", "-lc", f"VIDA_HOME={install_root} VIDA_ROOT={vida_root} {bin_dir / 'vida'} docflow help"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertIn("VIDA DocFlow compatibility bridge", result.stdout)
            self.assertIn(
                "installed-mode `vida docflow` compatibility contract is `help|overview only`",
                result.stdout,
            )
            self.assertIn("vida docflow overview [args...]", result.stdout)
            self.assertEqual(result.stderr, "")
            self.assertFalse(argv_log.exists())

    def test_codex_v0_help_prints_migration_guidance(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root, bin_dir, vida_root = self._install_wrappers(tmp_path)
            argv_log = self._write_python_stub(tmp_path, vida_root)

            result = subprocess.run(
                ["bash", "-lc", f"VIDA_HOME={install_root} VIDA_ROOT={vida_root} {bin_dir / 'codex-v0'} help"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertIn("Codex v0 compatibility wrapper", result.stdout)
            self.assertIn("`vida docflow` is the canonical installed launcher contract", result.stdout)
            self.assertIn("installed `codex-v0` is migration-only compatibility", result.stdout)
            self.assertEqual(result.stderr, "")
            self.assertFalse(argv_log.exists())

    def test_vida_docflow_rejects_unsupported_installed_mode_commands(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root, bin_dir, vida_root = self._install_wrappers(tmp_path)
            argv_log = self._write_python_stub(tmp_path, vida_root)

            result = subprocess.run(
                ["bash", "-lc", f"VIDA_HOME={install_root} VIDA_ROOT={vida_root} {bin_dir / 'vida'} docflow summary --format toon"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, "")
            self.assertIn(
                "vida docflow: unsupported installed-mode command: summary",
                result.stderr,
            )
            self.assertIn("VIDA DocFlow compatibility bridge", result.stderr)
            self.assertIn("help|overview only", result.stderr)
            self.assertFalse(argv_log.exists())

    def test_codex_v0_forwards_through_installed_docflow_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root, bin_dir, vida_root = self._install_wrappers(tmp_path)
            argv_log = self._write_python_stub(tmp_path, vida_root)

            result = subprocess.run(
                ["bash", "-lc", f"VIDA_HOME={install_root} VIDA_ROOT={vida_root} {bin_dir / 'codex-v0'} summary --format toon"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, "")
            self.assertIn(
                "vida docflow: unsupported installed-mode command: summary",
                result.stderr,
            )
            self.assertIn("VIDA DocFlow compatibility bridge", result.stderr)
            self.assertFalse(argv_log.exists())

    def test_vida_docflow_overview_bridges_to_bundled_codex_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root, bin_dir, vida_root = self._install_wrappers(tmp_path)
            argv_log = self._write_python_stub(tmp_path, vida_root)

            result = subprocess.run(
                ["bash", "-lc", f"VIDA_HOME={install_root} VIDA_ROOT={vida_root} {bin_dir / 'vida'} docflow overview --format toon"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertEqual(result.stdout, "")
            self.assertEqual(result.stderr, "")
            self.assertEqual(
                argv_log.read_text(encoding="utf-8").splitlines(),
                [
                    str(vida_root / "codex-v0" / "codex.py"),
                    "overview",
                    "--format",
                    "toon",
                ],
            )

    def test_release_build_manifest_records_installed_docflow_boundary(self) -> None:
        build_script = (REPO_ROOT / "scripts" / "build-release.sh").read_text(encoding="utf-8")
        marker = 'manifest = {'
        start = build_script.index(marker) + len(marker)
        end = build_script.index("\n}\nmanifest_path.write_text", start)
        manifest_body = build_script[start:end]
        entrypoints_marker = '"installed_entrypoints": ['
        entrypoints_start = manifest_body.index(entrypoints_marker)
        entrypoints_end = manifest_body.index("    ],", entrypoints_start) + len("    ],")
        installed_entrypoints_body = manifest_body[entrypoints_start:entrypoints_end]

        self.assertIn('"installed_compatibility_contracts"', manifest_body)
        self.assertIn('"vida docflow": "help|overview only"', manifest_body)
        self.assertIn('".codex/"', manifest_body)
        self.assertIn('"codex-v0": "migration-only wrapper -> vida docflow"', manifest_body)
        self.assertNotIn('"codex-v0"', installed_entrypoints_body)

    def test_resolve_version_parses_latest_release_without_short_pipe_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            fake_bin = tmp_path / "bin"
            fake_bin.mkdir()
            fake_curl = fake_bin / "curl"
            fake_curl.write_text(
                textwrap.dedent(
                    """\
                    #!/usr/bin/env bash
                    set -euo pipefail
                    cat <<'EOF'
                    {"tag_name":"v9.9.9","name":"Test Release"}
                    EOF
                    """
                ),
                encoding="utf-8",
            )
            fake_curl.chmod(fake_curl.stat().st_mode | stat.S_IEXEC)

            command = textwrap.dedent(
                f"""\
                set -euo pipefail
                export PATH="{fake_bin}:$PATH"
                source <(sed '$d' {INSTALLER})
                VERSION=latest
                ARCHIVE_FILE=""
                REPO_SLUG="example/repo"
                resolve_version
                """
            )
            result = subprocess.run(
                ["bash", "-lc", command],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertEqual(result.stdout.strip(), "v9.9.9")
            self.assertEqual(result.stderr, "")

    def test_runtime_config_scaffold_falls_back_to_framework_template(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            release_root = tmp_path / "release"
            template_path = release_root / "docs" / "framework" / "templates" / "vida.config.yaml.template"
            target_config = release_root / "vida.config.yaml"
            template_path.parent.mkdir(parents=True)
            template_path.write_text(
                (REPO_ROOT / "docs" / "framework" / "templates" / "vida.config.yaml.template").read_text(
                    encoding="utf-8"
                ),
                encoding="utf-8",
            )

            command = textwrap.dedent(
                f"""\
                set -euo pipefail
                source <(sed '$d' {INSTALLER})
                ensure_runtime_config_scaffold {release_root}
                """
            )
            subprocess.run(
                ["bash", "-lc", command],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertTrue(target_config.is_file())
            self.assertEqual(
                target_config.read_text(encoding="utf-8"),
                template_path.read_text(encoding="utf-8"),
            )

    def test_bootstrap_current_project_copies_framework_protocol_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            release_root = tmp_path / "release"
            project_root = tmp_path / "project"
            project_root.mkdir()

            (release_root / "install" / "assets").mkdir(parents=True)
            (release_root / "vida" / "config" / "instructions").mkdir(parents=True)
            (release_root / ".codex" / "agents").mkdir(parents=True)

            (release_root / "AGENTS.md").write_text("framework bootstrap\n", encoding="utf-8")
            (release_root / "AGENTS.sidecar.md").write_text("project sidecar\n", encoding="utf-8")
            (release_root / "vida" / "config" / "instructions" / "example.md").write_text(
                "protocol body\n", encoding="utf-8"
            )
            (release_root / ".codex" / "config.toml").write_text("[core]\n", encoding="utf-8")
            (release_root / ".codex" / "agents" / "orchestrator.toml").write_text("role='orchestrator'\n", encoding="utf-8")
            (release_root / "install" / "assets" / "vida.config.yaml.template").write_text(
                "project:\n  id: sample\n", encoding="utf-8"
            )

            command = textwrap.dedent(
                f"""\
                set -euo pipefail
                source <(sed '$d' {INSTALLER})
                VIDA_PROJECT_ROOT="{project_root}"
                bootstrap_current_project "{release_root}"
                """
            )
            subprocess.run(
                ["bash", "-lc", command],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertEqual((project_root / "AGENTS.md").read_text(encoding="utf-8"), "framework bootstrap\n")
            self.assertEqual((project_root / "AGENTS.sidecar.md").read_text(encoding="utf-8"), "project sidecar\n")
            self.assertEqual(
                (project_root / "vida" / "config" / "instructions" / "example.md").read_text(encoding="utf-8"),
                "protocol body\n",
            )
            self.assertEqual((project_root / ".codex" / "config.toml").read_text(encoding="utf-8"), "[core]\n")
            self.assertEqual(
                (project_root / "vida.config.yaml").read_text(encoding="utf-8"),
                "project:\n  id: sample\n",
            )

    def test_install_release_skips_download_when_version_is_already_active(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            install_root = tmp_path / "vida-home"
            release_root = install_root / "releases" / "v9.9.9"
            current_link = install_root / "current"
            release_root.mkdir(parents=True)
            current_link.parent.mkdir(parents=True, exist_ok=True)
            current_link.symlink_to(release_root, target_is_directory=True)

            command = textwrap.dedent(
                f"""\
                set -euo pipefail
                source <(sed '$d' {INSTALLER})
                INSTALL_ROOT="{install_root}"
                COMMAND=upgrade
                FORCE=no
                install_release v9.9.9
                """
            )
            result = subprocess.run(
                ["bash", "-lc", command],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                check=True,
            )

            self.assertIn("already the active installed version", result.stdout)
            self.assertIn("Nothing to download or replace", result.stdout)
            self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
