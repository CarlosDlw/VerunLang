# CLI Reference

## Synopsis

```
verun <command> [options]
```

## Commands

### `verun check`

Parse, type-check, and formally verify a `.verun` spec using the Z3 SMT solver.

```
verun check <file> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<file>` | Path to the `.verun` file to verify |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Show all individual checks, including passing ones |
| `--format <fmt>` | `-f` | Output format: `text` (default) or `json` |

**Examples:**

```bash
# Basic verification
verun check token.verun

# Verbose output showing every check
verun check token.verun -v

# JSON output for CI pipelines
verun check token.verun -f json
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0`  | All checks passed |
| `1`  | One or more checks failed, or a parse/type error occurred |

**JSON output schema:**

```json
{
  "file": "token.verun",
  "total_checks": 14,
  "passed": 14,
  "warnings": 0,
  "failed": 0,
  "all_passed": true
}
```

---

### `verun gen`

Generate implementation code from a verified spec. **Verification must pass** before code is generated — if any check fails, generation is aborted.

```
verun gen <file> -t <target> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<file>` | Path to the `.verun` file |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--target <lang>` | `-t` | Target language (required) |
| `--output <path>` | `-o` | Output file path. If omitted, prints to stdout |

**Target values:**

| Value | Alias | Language |
|-------|-------|----------|
| `rust` | `rs` | Rust |
| `typescript` | `ts` | TypeScript |
| `solidity` | `sol` | Solidity |
| `java` | - | Java |
| `go` | - | Go |
| `c` | - | C |
| `move` | - | Move |
| `cairo` | - | Cairo |
| `vyper` | `vy` | Vyper |

**Examples:**

```bash
# Print Rust code to stdout
verun gen token.verun -t rust

# Write TypeScript to a file
verun gen token.verun -t ts -o Token.ts

# Generate a Solidity contract
verun gen token.verun -t sol -o Token.sol
```

---

### `verun run`

Execute a spec using the runtime engine. Initializes the state machine and optionally runs a transition.

```
verun run <file> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<file>` | Path to the `.verun` file |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--transition <spec>` | `-t` | Transition to execute, format: `name(arg1,arg2)` |
| `--show-state` | `-s` | Print the state after execution |

**Examples:**

```bash
# Initialize and show state
verun run counter.verun -s

# Run a transition
verun run counter.verun -t "increment(10)" -s

# Run without showing state (just check if it succeeds)
verun run counter.verun -t "decrement(5)"
```

The runtime engine evaluates transitions concretely (with actual values), checking preconditions, executing the body, and validating invariants at runtime. This is useful for:
- Quick testing of specific scenarios
- Debugging transition logic
- Validating that concrete inputs produce expected results

---

### `verun fmt`

Format a `.verun` file to the canonical style. Parses the file and pretty-prints the AST back to source.

```
verun fmt <file> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<file>` | Path to the `.verun` file |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--check` | `-c` | Check formatting without modifying the file. Exits with code 1 if formatting differs |

**Examples:**

```bash
# Format in-place
verun fmt token.verun

# Check formatting (useful for CI)
verun fmt token.verun --check
```

---

### `verun ast`

Parse a `.verun` file and dump the AST. Useful for debugging parser issues or inspecting spec structure.

```
verun ast <file> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<file>` | Path to the `.verun` file |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--format <fmt>` | `-f` | Output format: `pretty` (default) or `json` |

**Examples:**

```bash
# Pretty-printed AST
verun ast token.verun

# JSON AST (for tooling)
verun ast token.verun -f json
```

---

### `verun init`

Scaffold a new `.verun` spec with a minimal valid template.

```
verun init <name> [options]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<name>` | Name of the state machine |

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--output <path>` | `-o` | Output file path. If omitted, creates `{name_lowercase}.verun` in the current directory |

**Examples:**

```bash
# Creates paymentprocessor.verun in current directory
verun init PaymentProcessor

# Write to a specific path
verun init PaymentProcessor -o payment_processor.verun
```

**Generated template:**

```verun
// PaymentProcessor — generated by verun init

state PaymentProcessor {
    value: int

    invariant non_negative {
        value >= 0
    }

    init {
        value = 0
    }

    transition increment(amount: int) {
        where {
            amount > 0
        }
        value += amount
        ensure {
            value == old(value) + amount
        }
    }
}
```

## CI Integration

### GitHub Actions

```yaml
- name: Verify specs
  run: |
    verun check specs/token.verun -f json > results.json
    verun check specs/voting.verun -f json >> results.json

- name: Check formatting
  run: |
    verun fmt specs/token.verun --check
    verun fmt specs/voting.verun --check
```

### Pre-commit Hook

```bash
#!/bin/bash
for file in $(git diff --cached --name-only -- '*.verun'); do
    verun check "$file" || exit 1
    verun fmt "$file" --check || exit 1
done
```

### Build Pipeline

```bash
# Verify all specs, then generate code
for spec in specs/*.verun; do
    verun check "$spec" -f json || exit 1
done

verun gen specs/token.verun -t sol -o contracts/Token.sol
verun gen specs/token.verun -t rs -o src/generated/token.rs
verun gen specs/token.verun -t ts -o src/generated/token.ts
```
