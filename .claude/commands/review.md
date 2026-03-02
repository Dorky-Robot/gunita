Run a multi-perspective review on a pull request for gunita.

## Step 1: Fetch the PR diff

```bash
gh pr diff $ARGUMENTS --repo $(gh repo view --json nameWithOwner --jq .nameWithOwner)
```

Also fetch the PR description for context:

```bash
gh pr view $ARGUMENTS --json title,body
```

## Step 2: Launch review agents in parallel

Send a **single message** with Task tool calls so they run concurrently. Each agent receives:

```
You are reviewing PR #<N> for gunita, a Rust/Axum household media server with SQLite, image processing, and Salita integration.

<pr-description>
<the PR title and body>
</pr-description>

<diff>
<the full diff output>
</diff>
```

The agents:

1. **Security reviewer** (`security-reviewer` agent) — Scan for path traversal, SQL injection, SSRF through Salita proxy, cache poisoning, and unauthenticated endpoint risks.

2. **Correctness reviewer** (`correctness-reviewer` agent) — Check SQLite transaction safety, async/blocking boundaries, cache race conditions, Salita error handling, and processing edge cases.

3. **Code quality reviewer** (`code-quality-reviewer` agent) — Evaluate Rust idioms, Axum patterns, AppError usage, rusqlite query structure, and frontend JS quality.

4. **Media pipeline reviewer** (`media-pipeline-reviewer` agent) — Only if the diff touches `processing.rs`, `cache.rs`, `media.rs`, or image/video related code. Check processing correctness, cache coherence, and streaming.

Each agent must end with exactly one verdict line:

```
VERDICT: APPROVE
VERDICT: APPROVE_WITH_NOTES
VERDICT: REQUEST_CHANGES
```

## Step 3: Synthesize verdicts

Combine all agent responses into a single review summary:

```
## Review Summary for PR #<N> — gunita

### Security
<verdict> — <key findings or "No issues">

### Correctness
<verdict> — <key findings or "No issues">

### Code Quality
<verdict> — <key findings or "No issues">

### Media Pipeline
<verdict or "N/A — no media changes"> — <key findings or "No issues">

### Overall
<APPROVE / APPROVE_WITH_NOTES / REQUEST_CHANGES>
<1-2 sentence summary>
```

## Step 4: Post as PR comment

```bash
gh pr comment $ARGUMENTS --body "<the review summary>"
```
