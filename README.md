# crap4ts

A small CRAP score analyzer for TypeScript and TSX, powered by Rust and
[Oxc](https://oxc.rs/).

```text
CRAP = complexity² × (1 - coverage)³ + complexity
```

`crap4ts` reads TypeScript source code and an Istanbul
`coverage-final.json`, then reports cyclomatic complexity, statement coverage,
and a CRAP score for each function.

## Status

This is an intentionally small MVP. It supports:

- `.ts` and `.tsx` source files;
- functions, arrow functions, and class methods;
- Istanbul JSON produced by Vitest, Jest, or c8;
- human-readable and JSON output;
- an optional threshold for CI gates.

## Install

Rust 1.97 or newer is currently required:

```bash
cargo install --path .
```

## Usage

Generate Istanbul coverage with your test runner first, then run:

```bash
crap4ts src --coverage coverage/coverage-final.json
```

Use JSON for automation:

```bash
crap4ts src \
  --coverage coverage/coverage-final.json \
  --format json
```

Use a threshold as a CI gate:

```bash
crap4ts src \
  --coverage coverage/coverage-final.json \
  --threshold 30
```

Exit codes:

- `0`: analysis or gate passed;
- `1`: at least one function exceeded the threshold;
- `2`: invalid input, coverage, or source code.

If a scanned source file is absent from the coverage report, analysis fails
instead of silently treating it as covered or uncovered. Configure your test
runner to include all files that you want to gate.

## Complexity profile

Each function starts at complexity `1`. The MVP adds one for each `if`, loop,
`catch`, ternary, non-default `case`, logical expression, logical assignment,
default value, and optional chain. Nested functions are measured separately.

Coverage statements are assigned to the innermost containing function, so a
nested function does not change its parent's coverage percentage.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Not in the MVP

Native V8 coverage, baselines, SARIF, HTML reports, and npm binary packaging
are deliberately deferred until the core behavior has been exercised on real
TypeScript projects.

## License

MIT
