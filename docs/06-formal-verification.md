# Formal Verification

This chapter explains how Verun proves your specs correct — what happens under the hood, what the solver actually checks, and how to interpret its results.

## How It Works

Verun uses **Z3**, a state-of-the-art SMT (Satisfiability Modulo Theories) solver developed by Microsoft Research. SMT solvers answer questions of the form: "is there any assignment of values to these variables that makes this formula true?"

Verun translates your spec into a series of logical formulas and asks Z3 to find counterexamples. If Z3 can't find one, the property is proven to hold for all possible inputs.

### The Verification Pipeline

```
source.verun
    ↓ parse
    AST
    ↓ type check
    Typed AST
    ↓ encode to SMT
    Z3 formulas
    ↓ solve
    PASS / FAIL + counterexample
```

Each step must succeed before the next runs. Type errors are caught before verification begins.

## What Gets Verified

For each state machine, Verun performs four categories of checks:

### 1. Init Satisfies Invariants

**Question**: "Does the initial state satisfy every invariant?"

For each invariant `I`, the solver checks:

> Given the init assignments, is `I` true?

If the init block sets `balance = 100` and the invariant says `balance >= 0`, the solver confirms that `100 >= 0` is true. Trivial in this case — but for complex specs with multiple fields and interrelated invariants, this catch is essential.

```
[PASS] init satisfies invariant 'non_negative'
[PASS] init satisfies invariant 'bounded'
```

### 2. Transitions Preserve Invariants

**Question**: "If all invariants hold before a transition, do they still hold after?"

This is the core inductive check. For each transition `T` and each invariant `I`, the solver asks:

> Assume all invariants hold in the current state. Assume the preconditions of `T` are satisfied. Apply the body of `T`. Is it possible that `I` is now false?

If the solver finds values where the invariant breaks, you get a `FAIL` with a counterexample. If it can't find any, the invariant is proven preserved.

```
[PASS] transition 'deposit' preserves invariant 'non_negative'
[PASS] transition 'deposit' preserves invariant 'bounded'
[FAIL] transition 'withdraw' preserves invariant 'non_negative'
       Counterexample: balance = 5, amount = 10
```

The counterexample shows a specific scenario: if `balance` is 5 and someone withdraws 10, the balance becomes -5, breaking `non_negative`. The fix: add `amount <= balance` to the `where` clause.

### 3. Postconditions Hold

**Question**: "After a transition runs, are its `ensure` conditions true?"

For each transition with an `ensure` block, the solver checks:

> Assume all invariants hold. Assume preconditions are satisfied. Apply the body. Do the postconditions hold?

```
[PASS] postcondition of 'transfer' holds
[FAIL] postcondition of 'increment' holds
       Counterexample: value = 50, amount = 10, result: value = 60, expected: value == old(value) + amount (60 == 60 ✓) — but another postcondition failed
```

Postcondition failures usually mean the body doesn't do what you think it does, or the postcondition is too strong for the actual implementation.

### 4. Dead Transition Detection

**Question**: "Can this transition ever fire?"

For each transition, the solver checks:

> Is there any state where all invariants hold AND all preconditions of this transition are satisfied?

If no such state exists, the transition is unreachable — a dead path in your state machine:

```
[WARN] transition 'emergency_stop' has unsatisfiable preconditions (dead transition)
```

This is reported as a warning, not an error. Dead transitions might be intentional (defensive design) or might indicate a logical error in your preconditions.

## The SMT Encoding

Understanding how Verun translates to SMT helps you write better specs and debug failures.

### SSA Encoding

Transitions are encoded using **Static Single Assignment** (SSA) form. Each assignment creates a new version of the variable:

```verun
// Verun source
balance -= amount
balance += fee
```

```
// SSA encoding
balance_1 = balance_0 - amount
balance_2 = balance_1 + fee
```

The solver sees `balance_0` (pre-state), `balance_1` (after first statement), and `balance_2` (post-state). Invariants are checked against the final version (`balance_2`).

This is why compound operations work correctly — each step is a distinct logical assignment, and the solver tracks the evolution precisely.

### Array Theory

Arrays and maps are encoded using Z3's Array theory, which provides `select` (read) and `store` (write) operations:

```verun
values[i] = 42
```

Becomes:

```
values_1 = store(values_0, i, 42)
```

Where `store(arr, idx, val)` returns a new array identical to `arr` except at index `idx`, which is now `val`. This is semantically precise — the solver knows exactly which elements changed and which didn't.

Reading from an array:

```verun
x = values[i]
```

Becomes:

```
x_1 = select(values_0, i)
```

The Array theory is what makes `forall` over arrays efficient — the solver doesn't expand `forall i in 0..8: values[i] >= 0` into 8 separate checks. It reasons about the array as a whole using the theory's axioms.

### Enum Encoding

Enums are encoded as distinct integers. For:

```verun
enum Phase { Active, Paused, Closed }
```

The solver creates:

```
Active  = 0
Paused  = 1
Closed  = 2
distinct(Active, Paused, Closed)
```

With a constraint that the enum field can only equal one of these values. Comparison `phase == Phase::Active` becomes `phase == 0`.

### Uninterpreted Functions

Extern functions are encoded as **uninterpreted functions** in SMT. The solver knows:

- The function has a specific type signature.
- `f(a, b) == f(a, b)` for the same inputs (congruence).
- Nothing else about what it computes.

```verun
fn hash(data: int, nonce: int) -> int
```

The solver will not assume `hash(1, 2)` has any particular value. But it will correctly derive that if `x == y`, then `hash(x, nonce) == hash(y, nonce)`.

This is powerful for abstraction — you can reason about your system's properties without specifying every computation.

### Builtin Functions

Builtin functions (`abs`, `min`, `max`) are encoded as **ITE (if-then-else)** expressions in SMT:

- `abs(x)` → `if x >= 0 then x else -x`
- `min(a, b)` → `if a <= b then a else b`
- `max(a, b)` → `if a >= b then a else b`

Unlike uninterpreted functions, the solver knows exactly what these compute and can reason about their results.

### Implication Encoding

The `==>` operator is encoded as Z3's native implication (`implies`). The formula `a ==> b` is logically equivalent to `!a || b`, but the native encoding can be more efficient for the solver.

### Refinement Type Encoding

When a field or parameter has a [refinement type](05-type-system.md#refinement-types), the verifier automatically injects the refinement constraint into the SMT model:

- **Pre-state fields:** The refinement is asserted as an assumption alongside invariants. If `balance: PositiveInt` where `value > 0`, the solver assumes `balance > 0` in the pre-state.
- **Transition parameters:** If a parameter has a refined type, its constraint is assumed during verification.
- **Init check:** The init values must still satisfy all invariants (refinement assumptions don't apply to init — the init values must stand on their own).

The `value` keyword in the refinement is substituted with the actual field or parameter name during encoding.

## Reading Counterexamples

When verification fails, the solver provides a **counterexample** — a concrete assignment of values that demonstrates the failure. Counterexamples include the specific expression that was violated:

```
[FAIL] transition 'withdraw' preserves invariant 'non_negative'
       expression: balance >= 0
       Counterexample:
         balance: 0 -> -1
         amount: 1
```

This tells you:
1. **Which transition** caused the problem: `withdraw`
2. **Which invariant** was violated: `non_negative`
3. **The invariant expression** that failed: `balance >= 0`
4. **Specific values** that trigger the violation, including pre → post state changes
5. **Unchanged fields** are annotated with `(unchanged)` for clarity

You can manually trace through:
- Pre-state: `balance = 0` (satisfies `balance >= 0`)
- Precondition: `amount > 0` → `1 > 0` ✓ (precondition is satisfied)
- Body: `balance -= amount` → `balance = 0 - 1 = -1`
- Invariant check: `balance >= 0` → `-1 >= 0` ✗ **VIOLATED**

The fix is clear: add `amount <= balance` to the `where` clause.

### Tips for Debugging Failures

1. **Read the counterexample carefully.** It's a real scenario. Walk through the transition step by step with those exact values.

2. **Check your preconditions.** Most invariant violations mean a precondition is missing or too weak. The counterexample shows what input you failed to guard against.

3. **Check your body logic.** If preconditions seem correct, the body might not do what you intended. SSA encoding is precise — maybe an intermediate step goes out of bounds.

4. **Simplify.** If you can't figure out the failure, temporarily remove invariants or transitions to isolate which interaction causes the problem.

5. **Use postconditions as debugging aids.** Add `ensure` clauses to verify your assumptions about what the body actually computes.

## Verification Output Formats

### Text (default)

```bash
verun check spec.verun
```

Human-readable output with PASS/FAIL/WARN for each check.

### JSON

```bash
verun check spec.verun -f json
```

Machine-readable output for CI integration:

```json
{
  "file": "spec.verun",
  "total_checks": 10,
  "passed": 9,
  "warnings": 0,
  "failed": 1,
  "all_passed": false
}
```

### Verbose

```bash
verun check spec.verun -v
```

Shows every individual check, including passing ones that might otherwise be summarized.

## Verification Guarantees and Limits

### What Verification Proves

- **Invariants hold inductively.** If the init state is valid and every transition preserves all invariants, then every reachable state satisfies all invariants.
- **Postconditions hold.** Given valid preconditions and invariants, the body produces the specified result.
- **No dead transitions** (or they're flagged).

### What Verification Does NOT Prove

- **Liveness.** Verun proves safety (bad things don't happen), not liveness (good things eventually happen). It doesn't prove that a transition will eventually fire.
- **Termination.** Verun transitions are single-step — there are no loops to terminate. But the spec doesn't guarantee your system makes progress.
- **Implementation correctness beyond the spec.** If your spec is wrong (missing an invariant, wrong postcondition), the generated code faithfully implements the wrong spec.
- **Concurrency.** Each transition is verified in isolation. If your system has concurrent transitions, you need additional reasoning about interleavings.
- **String operations.** The solver has limited support for string reasoning. Avoid string-dependent invariants.

### Performance

Verification time depends on:
- Number of state fields
- Complexity of invariants (quantifiers are expensive)
- Number of transitions × invariants (each pair is a separate check)
- Use of arrays and maps (Array theory is efficient but adds complexity)

For typical specs (< 20 fields, < 10 transitions, < 5 invariants), verification completes in under a second. Specs with extensive quantifiers over large arrays may take a few seconds.
