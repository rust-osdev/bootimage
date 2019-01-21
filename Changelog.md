# 0.6.4

- Canonicalize paths before comparing them when invoking `bootimage test`
  - This caused an error on Windows where the path in the cargo metadata is not fully canonicalized
- Improve CI infrastructure

# 0.6.3

- Canonicalize paths before comparing them when invoking `bootimage build`
  - This caused an error on Windows where the path in the cargo metadata is not fully canonicalized

# 0.6.2

- Fix build on Windows (don't use the `.` directory)

# 0.6.1

- Fix: bootimage should now work correctly with `--manifest-path`

# 0.6.0

(Yanked from crates.io because of a bug fixed in 0.6.1.)

**Breaking**:

- When no `--manifest-path` argument is passed, `bootimage` defaults to the `Cargo.toml` in the current directory instead of the workspace root.
  - This fixes compilation of projects that are part of a workspace
