Consult the masters — review the entire gunita codebase through the lens of great software engineers.

## Phase 1: Map the Codebase

Thoroughly explore the full project structure. Use Glob and Grep to build a complete picture:

1. **Source code** — find all `.rs` files in `src/`
2. **Configuration** — `Cargo.toml`, migration files
3. **Frontend** — `static/` (app.js, style.css, index.html)
4. **Infrastructure** — `scripts/`, any CI configs

Read ALL source files. Every module, every handler, every test. Do not skip files or skim — each agent needs the full picture.

## Phase 2: Launch Review Agents in Parallel

Send a single message with 8 Task tool calls so they run concurrently. Each agent should be `subagent_type: "general-purpose"` with access to file-reading tools.

**IMPORTANT**: Tell each agent to read the source files directly. The prompt should describe the project and direct the agent to the relevant directories.

Shared context for every agent prompt:
```
You are reviewing gunita, a Rust/Axum household media server.
Tech stack: Rust 2021, Axum 0.8, SQLite (rusqlite + r2d2), Tokio, vanilla JS frontend.
Key directories: src/ (main.rs, lib.rs, config.rs, db.rs, state.rs, cache.rs, processing.rs, error.rs, models.rs, salita_client.rs, api/), static/, migrations/
Domain: Media browsing, RAW image processing, thumbnail caching, device integration via Salita, memory curation.

Read ALL source files before forming your review.
Report your top 5 findings ranked by impact. For each finding, cite the specific file and line.
Do NOT suggest changes that would reduce capabilities or fight Rust/Axum idioms.
```

### Agent 1: Rich Hickey — Simplicity & Data Orientation
Look for complecting, data hiding behind abstractions, mutable state that could be values, accidental complexity, and opportunities for composition.

### Agent 2: Alan Kay — Message Passing & Late Binding
Check if modules communicate through clear interfaces, if decisions are made too early (hardcoded vs parameterized), if the architecture would hold at 100x scale.

### Agent 3: Eric Evans — Domain-Driven Design
Check ubiquitous language (do code names match the media/memory domain?), bounded contexts between browsing/processing/curation, entity vs value object modeling.

### Agent 4: Composition & Functional Design
Look for pure core / impure shell separation, total functions, algebraic data types that could replace stringly-typed values, and referential transparency.

### Agent 5: Joe Armstrong — Fault Tolerance & Isolation
Check failure isolation between Salita integration, image processing, and SQLite. Is error handling defensive or does it let failures cascade? What happens when Salita is down?

### Agent 6: Sandi Metz — Practical Object Design
Check single responsibility, dependency direction, Tell Don't Ask patterns, method/module sizes, and cost of change.

### Agent 7: Leslie Lamport — State Machines & Temporal Reasoning
Enumerate the states of a memory item lifecycle, catalog caching, and thumbnail processing. Are transitions explicit? Are there race conditions in concurrent access?

### Agent 8: Kent Beck — Simple Design & Courage to Change
Apply the four rules (passes tests, reveals intention, no duplication, fewest elements). Find YAGNI violations, missing tests, and code that resists change.

## Phase 3: Distill

1. **Cross-reference** — Find findings multiple agents agree on.
2. **Filter** — Discard findings that fight Rust idioms or reduce capabilities.
3. **Rank** — Order by impact on clarity, maintainability, or correctness.

## Phase 4: Build the Execution Plan

Create a phased plan grouped by tier:
- **Tier 1**: Critical fixes (bugs, safety issues)
- **Tier 2**: Type safety & cleanup (dead code, stringly-typed fixes)
- **Tier 3**: Structural improvements (decomposition, extraction)
- **Tier 4**: Architectural evolution (cross-cutting changes)

## Phase 5: Present Plan and Get Feedback

Present the plan and ask: "How should I proceed?" with options:
- **Execute all**
- **Execute Tier 1-2 only**
- **Let me adjust first**

## Phase 6: Execute

Work through approved tiers, committing after each phase.

## Phase 7: Ship

Run full test suite and `/ship-it`.
