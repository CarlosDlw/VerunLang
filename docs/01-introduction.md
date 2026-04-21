# Introduction

## What is Verun?

Verun is a formal specification language designed for one purpose: **proving that your system behaves correctly before a single line of production code is written**.

You describe your system as a state machine — its data, its rules, its transitions — and Verun mathematically proves that your rules can never be broken. Not by testing a few cases. Not by running simulations. By exhaustive formal proof using an SMT solver.

If the proof passes, Verun can then generate verified implementation code in Rust, TypeScript, or Solidity. The generated code carries the guarantees you specified — precondition checks, postcondition assertions, invariant enforcement — all derived from the spec, not written by hand.

Verun is not a general-purpose programming language. It is a **verifiable specification language** that sits upstream of your implementation. You write the spec, prove it correct, and generate the code.

## Why Verun Exists

Every non-trivial system has rules that must never be broken:

- A token's total supply must always equal the sum of all balances.
- A voting system must never count more votes than there are voters.
- An auction's highest bid must always be above the minimum.
- A buffer must never be read beyond its valid length.

These rules are usually encoded as ad-hoc assertions, unit tests, or comments that say "this should never happen". They get violated anyway — in edge cases, under concurrency, after refactors, by developers who didn't read the comments.

Verun takes a different approach. You declare these rules as **invariants**, and the tool proves — at compile time, with mathematical certainty — that no sequence of valid operations can ever violate them. If it finds a way to break your invariant, it gives you a concrete counterexample showing exactly how.

## When to Use Verun

Verun is built for systems where **correctness matters more than speed of development**:

**Smart Contracts** — Financial logic where a single bug can drain millions. Verun specs compile directly to Solidity with `require()` guards derived from proven invariants.

**Protocol Design** — State machines that coordinate distributed systems, consensus mechanisms, or message-passing protocols. Prove your protocol correct before implementing it in any language.

**Financial Systems** — Balance conservation, transaction ordering, settlement rules. Verun proves your accounting invariants hold across all possible transaction sequences.

**Safety-Critical State Machines** — Embedded controllers, access control systems, workflow engines. Anywhere a state machine governs behavior that cannot fail silently.

**API Contract Specification** — Define the valid states and transitions of your backend, prove they're consistent, then generate typed implementations with built-in validation.

## When Not to Use Verun

Verun is **not** a replacement for your application framework. It does not handle:

- UI rendering or frontend logic
- Database queries or ORM mappings
- Network I/O or HTTP handling
- General-purpose computation

Verun specifies the **core invariants and state transitions** of your system. The generated code integrates into your existing codebase as a verified module.

## Design Philosophy

**Specifications are the source of truth.** Implementation is derived, not manually written. If the spec is correct and verified, the generated code inherits those guarantees.

**Declarative over imperative.** You describe *what must be true*, not *how to check it*. The SMT solver figures out whether your properties hold across all reachable states.

**Prove, don't test.** Testing checks a finite number of cases. Verification checks all of them. A passing test suite means "I didn't find a bug". A passing verification means "no bug exists for these properties".

**Fail early and loud.** If your spec has a flaw, Verun tells you before any code is generated. If a transition can violate an invariant, you get a counterexample — not a runtime crash in production.

**Minimal and precise.** Verun has a small, focused syntax. No inheritance hierarchies, no generic metaprogramming, no runtime reflection. Just states, transitions, invariants, and types. Everything that exists in the language serves the goal of verifiable specification.
