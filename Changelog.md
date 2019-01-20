
- \[Breaking\] When no `--manifest-path` argument is passed, `bootimage` defaults to the `Cargo.toml` in the current directory instead of the workspace root.
  - This fixes compilation of projects that are part of a workspace
