# Verun Documentation

**Programming by Executable Invariants** — A formal specification language with SMT verification and multi-target code generation.

## Table of Contents

### Learning Verun

1. **[Introduction](01-introduction.md)** — What Verun is, why it exists, and when to use it
2. **[Getting Started](02-getting-started.md)** — Installation, first spec, verification, code generation
3. **[Core Concepts](03-core-concepts.md)** — States, invariants, transitions, preconditions, postconditions, and the verification model

### Language Reference

4. **[Language Guide](04-language-guide.md)** — Full syntax: imports and aliases, expressions, statements, `else if`, `let`, `match`, constants, operators, implication, quantifiers, enums, types, refinement types, builtin functions
5. **[Type System](05-type-system.md)** — Primitives, arrays, maps, enums, structs, refinement types, and type checking rules

### Verification & Generation

6. **[Formal Verification](06-formal-verification.md)** — How the SMT solver works, what it proves, SSA encoding, refinement constraints, counterexamples, and limits
7. **[Code Generation](07-code-generation.md)** — Rust, TypeScript, and Solidity targets — how specs map to code

### Tools

8. **[CLI Reference](08-cli-reference.md)** — Complete command reference: `check`, `gen`, `run`, `fmt`, `ast`, `init`
