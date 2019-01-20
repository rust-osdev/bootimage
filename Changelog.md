- Fix build on Windows

# 0.6.1

- Fix: bootimage should now work correctly with `--manifest-path`

# 0.6.0

(Yanked from crates.io because of a bug fixed in 0.6.1.)

**Breaking**:

- When no `--manifest-path` argument is passed, `bootimage` defaults to the `Cargo.toml` in the current directory instead of the workspace root.
  - This fixes compilation of projects that are part of a workspace
