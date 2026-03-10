# VIDA Mobile Bundle

Purpose: keep `vida-mobile` project-specific material isolated from the main `vida-stack` framework cleanup while preserving it for later extraction into its own repository.

Rules:
1. This folder is a local separation bundle, not the active framework canon.
2. Do not delete project-specific content from here during framework cleanup.
3. Do not publish or commit this bundle as part of public framework history.
4. Treat this folder as the source package to copy later into the dedicated `vida-mobile` project.

Contents:
1. `vida.config.yaml`
   - current `vida-mobile` project overlay snapshot preserved from the root project-owned config
2. `docs/`
   - extracted project documentation surfaces that do not belong inside `vida/config/instructions/**`
