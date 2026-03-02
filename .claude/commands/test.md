Build, lint, and test gunita.

## Step 1: Format check

```bash
cargo fmt --all -- --check
```

If formatting issues are found, report them but continue with the remaining steps.

## Step 2: Lint

```bash
cargo clippy --all-targets -- -D warnings
```

If clippy warnings are found, list them with file:line references.

## Step 3: Build

```bash
cargo build 2>&1
```

If the build fails, stop and report the errors.

## Step 4: Test

```bash
cargo test --workspace 2>&1
```

Report test results. If there are no tests, note that and suggest adding tests for the most critical modules:
- `db.rs` — migration and query correctness
- `cache.rs` — cache key generation and path safety
- `processing.rs` — image resize and format handling
- `salita_client.rs` — response deserialization

## Step 5: Summary

```
## gunita build report

| Check       | Result |
|-------------|--------|
| Format      | PASS / FAIL |
| Clippy      | PASS / N warnings |
| Build       | PASS / FAIL |
| Tests       | PASS / N failures / No tests |
```

If any step failed, provide specific fix recommendations.
