# Task Execution Instructions

## CRITICAL: Execution Efficiency Rules (MUST FOLLOW)

These two rules are NON-NEGOTIABLE. Violating them wastes 25-35% of your turn budget.

### Rule A: ALWAYS batch independent tool calls in the SAME turn.
Every time you are about to call a tool, ask: "Is there another independent call I can make at the same time?" If yes, emit BOTH tool calls in the SAME message.

**Mandatory parallel patterns — use these EVERY session:**
- Session start: `bd ready` + `blacksmith progress show --bead-id <id>` → ONE turn, TWO tool calls
- Reading source + test: `Read foo.rs` + `Read foo_test.rs` → ONE turn
- Multiple greps: `Grep("pattern1")` + `Grep("pattern2")` → ONE turn
- Session end: `Bash(cargo clippy --fix --allow-dirty)` + `Bash(cargo test --release)` → ONE turn (if they don't depend on each other's output)
- Reading multiple related files: `Read config.rs` + `Read main.rs` → ONE turn

**A session with ZERO parallel calls is a failure.** Target at least 5 turns with 2+ parallel calls per session.

### Rule B: NEVER emit a text-only turn. Every assistant message MUST include at least one tool call.
WRONG: "Let me check the tests." (turn 1) → `Grep(tests/)` (turn 2)
RIGHT: "Let me check the tests." + `Grep(tests/)` (turn 1 — one message, includes both text AND tool call)

If you want to narrate what you're doing, include the narration AND the tool call in the same message. A text-only turn doubles your turn count for zero benefit.

### Rule C: After closing your bead, EXIT IMMEDIATELY.
Do NOT triage other beads. Do NOT run `bd ready` to find more work. Do NOT explore what to do next.
The sequence after closing is: `blacksmith progress add --bead-id <id> --stdin` -> run `blacksmith finish` -> STOP.
Each session handles exactly ONE bead. The loop script handles picking the next one.

---

## Context Loading

The project architecture is documented in MEMORY.md — do NOT re-explore the codebase.
Only read files you are about to modify. Do NOT launch explore subagents (this means NO `Task` tool with `subagent_type: Explore`).

1. Run `bd ready` AND `blacksmith progress show --bead-id <id>` in the SAME turn (Rule A — two parallel tool calls)

## Task Selection
Pick ONE task from the ready queue. **Always pick the highest-priority (lowest number) ready task.** Only deviate if recent `blacksmith progress list --bead-id <id>` entries explain why a specific lower-priority task should go next (e.g., it's a quick follow-up to the last session's work).

**Remember Rule C**: You will work on exactly ONE task this session. After closing it, exit immediately.

### Failed-Attempt Detection
Before claiming a task, run `bd show <id>` and check its notes for `[FAILED-ATTEMPT]` markers.

- **0 prior failures**: Proceed normally.
- **1 prior failure**: Proceed, but read the failure reason carefully. If the reason mentions "too large" or "ran out of turns," consider whether you can realistically finish in 55 turns. If not, skip to the decomposition step below.
- **2+ prior failures**: Do NOT attempt implementation. Instead, decompose the bead into smaller sub-beads:
  1. Analyze the bead description and failure notes to understand why it keeps failing
  2. Break it into 2-5 smaller sub-beads (follow the break-down-issue workflow: create children, wire deps, make original blocked-by children)
  3. Record decomposition with `blacksmith progress add --bead-id <id> --stdin`, then exit cleanly via `blacksmith finish`
  4. The next session will pick up the newly-unblocked child beads

If ALL top-priority ready beads have 2+ failures and you've decomposed them, move to the next priority level.

### No Work Available
If `bd ready` returns no tasks, exit immediately:
1. Do NOT create any git commits
2. Do NOT write a progress entry
3. Simply exit — the harness will handle retry/shutdown

## Execution Protocol
For the selected task (e.g., bd-X):

1. **Claim**: `bd update bd-X --status in_progress`

2. **Understand**: Run `bd show bd-X` for full task description. If the task references a PRD section, read it with an offset (see PRD index in AGENTS.md).

3. **Implement**: Complete the task fully
   - Only read files you need to modify — architecture is in MEMORY.md
   - Follow existing code patterns (see MEMORY.md for architecture and testing conventions)

4. **Verify** (use parallel calls per Rule A):

   **4a. Bead-specific verification:**
   Run `bd show bd-X` and look for a "## Verify" section in the description. If it exists, execute those exact steps. If any verification step fails, fix the issue before proceeding.

   If the bead has NO "## Verify" section, add one now:
   ```bash
   bd update bd-X --notes="## Verify\n- Run: <command you used to test>\n- Expect: <what you observed>"
   ```

   **4b. Code quality gates:**
   ```bash
   # Run full test suite FIRST, then lint in parallel:
   cargo test --release
   # Then in ONE turn with TWO parallel Bash calls:
   cargo clippy --fix --allow-dirty
   cargo fmt --check
   ```
   Run lint and format exactly ONCE each. Do not repeat them.

   **4c. Integration check:**
   Before closing, verify your changes don't break existing callers. Grep for the function/struct names you changed or renamed. If other code references them, confirm those references still work.

5. **Finish** — record progress and call `blacksmith finish`, then STOP (Rule C):
   - **Write a progress entry** with `blacksmith progress add --bead-id bd-X --stdin` and include a short handoff note:
     - What you completed this session
     - Current state of the codebase
     - Suggested next tasks for the next session
   - **Run the finish command**:
     ```bash
     blacksmith finish bd-X "<brief description>" src/file1.rs src/file2.rs
     ```
     This runs quality gates (check + test), verifies bead deliverables, then handles: staging, committing, bd close, bd sync, auto-committing .beads/, recording bead closure metadata, and git push — all in one command.
     **If quality gates fail, the bead is NOT closed.** Fix the issues and re-run.
   - If no specific files to stage, omit the file list and it will stage all tracked modified files.
   - **After `blacksmith finish` completes, STOP. Do not triage more work. Do not run bd ready. Session is done.**

## Turn Budget (R1)

You have a **hard budget of 80 assistant turns** per session. Track your turn count.

- **Turns 1-55**: Normal implementation. Write code, run targeted tests (`--filter`).
- **Turns 56-65**: **Wrap-up phase.** Stop new feature work. Run the full test suite + `lint:fix` + `analyze`. If passing, commit and close.
- **Turns 66-75**: **Emergency wrap-up.** If tests/lint are failing, make minimal fixes. If you can't fix in 10 turns, revert your changes (`git checkout -- .`), mark the failure (see below), record a progress entry, and exit cleanly.
- **Turn 76+**: **Hard stop.** Do NOT start any new work. If you haven't committed yet: revert, mark the failure, record a progress entry, and exit immediately. An uncommitted session is worse than a cleanly abandoned one.

If you realize before turn 40 that the task is too large to complete in the remaining budget, STOP immediately. Mark the failure, and exit. Do not burn 40 more turns on a doomed session.

### Marking a Failed Attempt
When bailing out of a task for any reason, always run:
```bash
bd update <id> --status=open --notes="[FAILED-ATTEMPT] <YYYY-MM-DD> <reason>"
```
Use a specific reason: `too-large`, `tests-failing`, `lint-unfixable`, `missing-dependency`, `context-overflow`, or a brief custom description. This marker is read by future sessions to detect beads that need decomposition (see Task Selection).

## Stop Conditions
- Complete exactly ONE task per iteration, then STOP (Rule C)
- After calling `blacksmith finish`, do NOT continue. Do NOT triage. Do NOT run bd ready again.
- If task cannot be completed, mark the failure (see above), record progress with `blacksmith progress add`, exit cleanly
- If tests fail, debug and fix within this iteration

## Improvement Recording

Record institutional lessons using `blacksmith improve add` when you encounter reusable insights during your session. This builds the project's knowledge base so future sessions avoid repeated mistakes and adopt proven patterns.

**When to record** (pick at most 2 per session — don't spend turns on this):
- You discover a non-obvious debugging technique or root cause
- You find a code pattern that should be followed (or avoided) project-wide
- You notice a workflow inefficiency (e.g., unnecessary file reads, redundant test runs)
- A test failure reveals a subtle invariant that isn't documented

**When NOT to record:**
- Routine task completion (closing a bead is not an insight)
- Obvious things already in MEMORY.md or PROMPT.md
- Session-specific context that won't help future sessions

**How to record:**
```bash
blacksmith improve add "Short descriptive title" \
  --category <workflow|cost|reliability|performance|code-quality> \
  --body "What you learned and why it matters" \
  --context "Evidence: session number, file, or error message"
```

**Example:**
```bash
blacksmith improve add "Always check Cargo.toml when adding new modules" \
  --category reliability \
  --body "New module files need their crate dependencies added to Cargo.toml. Cargo check catches this but only if run before bead closure." \
  --context "Session 50 closed a bead with uncompilable code because Cargo.toml was missing the fs2 dependency"
```

Record improvements as you work — don't batch them to the end of the session.

## Analysis Session

> **CRITICAL (R44): Turn 1 MUST be tool calls.** Emit the python3 comprehensive session parse script AND `blacksmith improve list` in ONE parallel turn. Turn 1 narration is a Rule A violation.

Analysis sessions parse a past session's `.jsonl` file and file improvements. They do NOT follow the normal Execution Protocol.

**Skip Context Loading entirely.** Do NOT read MEMORY.md or PROMPT.md — all context is in the task prompt.

**Turn 1 is mandatory**: emit TWO parallel tool calls — session parse script + `blacksmith improve list`. No narration-only turns allowed. All tool calls must be parallel (0 parallel turns = Rule A violation).

### Step 1: Load metrics + improvement list (ONE parallel turn, TWO tool calls)

Run the python3 one-pass script (Step 2) AND `blacksmith improve list` in the same turn. Check existing improvements to avoid duplicates before filing new ones.

### Step 2: Parse session in ONE pass

Use this template. Do NOT re-read the file incrementally — one script, one Bash call:

```python
import json, sys
from collections import defaultdict

SESSION = ".blacksmith/sessions/<N>.jsonl"  # replace <N> with actual session number

with open(SESSION) as f:
    lines = [json.loads(l) for l in f if l.strip()]

# Aggregate assistant turns by message ID (same ID can span multiple lines)
turns = defaultdict(list)   # message_id -> [content_block, ...]
turn_order = []

for msg in lines:
    if msg.get("type") == "assistant":
        m = msg.get("message", {})
        mid = m.get("id", "unknown")
        if mid not in turns:
            turn_order.append(mid)
        turns[mid].extend(m.get("content", []))

# Compute metrics
total_messages   = len(lines)
total_assistant  = len(turn_order)
tool_call_turns  = 0
narration_only   = 0
parallel_tool    = 0
tool_calls_by_name = defaultdict(int)
sample_narrations  = []

for mid in turn_order:
    blocks     = turns[mid]
    tool_use   = [b for b in blocks if b.get("type") == "tool_use"]
    text_only  = [b for b in blocks if b.get("type") == "text"]
    if tool_use:
        tool_call_turns += 1
        for b in tool_use:
            tool_calls_by_name[b.get("name", "unknown")] += 1
        if len(tool_use) > 1:
            parallel_tool += 1
    elif text_only:
        narration_only += 1
        if len(sample_narrations) < 3:
            sample_narrations.append(text_only[0]["text"][:120])

narration_ratio = narration_only / total_assistant if total_assistant else 0.0

print(f"total_messages:      {total_messages}")
print(f"total_assistant:     {total_assistant}")
print(f"tool_call_turns:     {tool_call_turns}")
print(f"narration_only:      {narration_only}")
print(f"narration_ratio:     {narration_ratio:.1%}")
print(f"parallel_tool_turns: {parallel_tool}")
print(f"tool_calls_by_name:  {dict(tool_calls_by_name)}")
print(f"sample_narrations:   {sample_narrations}")
```

### Step 3: Identify improvements

Compare metrics against targets:
- narration_ratio > 10% → Rule B violation, note specific narration texts
- parallel_tool_turns == 0 → Rule A violation (session had zero parallel calls)
- narration_ratio == 0% AND parallel_tool_turns > 0 → healthy session, record nothing

### Step 4: Dismiss resolved improvements

If any `blacksmith improve list` entries are already addressed by recent PROMPT.md changes, call `blacksmith improve dismiss <id>` for each in ONE parallel turn.

### Step 5: File new improvements

Emit exactly N `blacksmith improve add` calls in ONE parallel turn — one per insight found. Do not batch them to the end; do not file more than 3 per analysis session.

---

## Verification

Before closing a task, run these commands and ensure they pass:

- test: `cargo test --release`
- lint: `cargo clippy --fix --allow-dirty`
- format: `cargo fmt --check`

## Important
- Do not ask for clarification — make reasonable decisions
- Do NOT launch explore/research subagents (NO `Task` with `subagent_type: Explore`) — the architecture is in MEMORY.md
- Do NOT re-read files you already know from MEMORY.md
- Prefer small, atomic changes over large refactors
- Always run `cargo test --release` before committing
- Always run `cargo clippy --fix --allow-dirty` then `cargo fmt --check` before committing — exactly ONCE each
- Always use `blacksmith finish` to close out — do NOT manually run git add/commit/push/bd close/bd sync
- **NEVER call `bd close` directly** — always go through `blacksmith finish` which enforces quality gates
- **EFFICIENCY**: Re-read Rules A, B, C above. Every text-only turn and every sequential-when-parallel tool call wastes your limited turn budget. Aim for 5+ parallel turns per session and 0 narration-only turns.

<!-- Promoted from R1 [prompt] -->
- In PROMPT.md under ## Execution Protocol step 3 (Implement), add after 'Complete the task fully': 'CRITICAL: If after bd show you determine acceptance criteria are already satisfied by prior work, do NOT write a progress entry yet. Skip to step 4 immediately and run ALL commands in the ## Verify section of the bead. Only after every verify command passes should you record progress and close. Skipping verify on a pre-satisfied bead is a session failure — as seen when a session claimed done but missed a required test fixture.'

<!-- Promoted from R2 [prompt] -->
- In PROMPT.md under ## Execution Protocol, add after step 2 (Understand) a new sub-step 2b: '**2b. Pre-satisfied check**: If all acceptance criteria appear already met by prior sessions work, take the PRE-SATISFIED PATH: do NOT read any source files, do NOT write any code. Skip step 3 entirely. Jump to step 4a (Verify section). This avoids wasting turns on implementation exploration when nothing needs to be built.'

<!-- Promoted from R3 [cost] -->
- In PROMPT.md under step 4b (Code quality gates), add: 'If you took the pre-satisfied path (zero files modified this session), skip cargo clippy --fix --allow-dirty since there is nothing to fix. Run only: cargo test --release and cargo fmt --check. This saves 1-2 turns per trivial session and avoids running a mutation pass with nothing to mutate.'

<!-- Promoted from R4 [prompt] -->
- In PROMPT.md ## Context Loading section, add after 'do NOT re-explore the codebase': '(If MEMORY.md does not exist, skip it — do NOT fall back to ls, explore agents, or re-reading known files. Proceed directly to step 1.)'

<!-- Promoted from R5 [prompt] -->
- In PROMPT.md under step 4a (Bead-specific verification), add after 'execute those exact steps': 'If the ## Verify section lists multiple independent commands (e.g. cargo test AND a curl check), emit all of them as parallel Bash calls in ONE turn per Rule A — do not run them sequentially.'

<!-- Promoted from R6 [prompt] -->
- In PROMPT.md under '### Marking a Failed Attempt', change the bd update example from: '--notes="[FAILED-ATTEMPT] <YYYY-MM-DD> <reason>"' to: '--notes="[FAILED-ATTEMPT] <YYYY-MM-DD> <reason>: <first 2 lines of the actual error>"'. This helps future sessions (2+ failures path) diagnose root cause instead of only knowing the category label.

<!-- Promoted from R7 [prompt] -->
- In PROMPT.md line 11, change '- Session start: `bd ready` + `blacksmith progress show --bead-id <id>` → ONE turn, TWO tool calls' to '- Session start: `bd ready` + `Read MEMORY.md` → ONE turn, TWO tool calls'. Also update line 37: change 'Run `bd ready` AND `blacksmith progress show --bead-id <id>` in the SAME turn (Rule A — two parallel tool calls)' to 'Run `bd ready` AND `Read MEMORY.md` in the SAME turn (Rule A — two parallel tool calls). Then once you have the bead ID, run `bd show <id>` AND `blacksmith progress show --bead-id <id>` in the SAME turn.' Rationale: blacksmith progress show requires the bead ID from bd ready, so they cannot be emitted in one turn; Read MEMORY.md has no such dependency.

<!-- Promoted from R9 [prompt] -->
- In PROMPT.md under '## Context Loading', after step 1 ('Run bd ready AND Read MEMORY.md in the SAME turn'), add step 1b: '1b. Once `bd ready` returns the bead ID, immediately run `bd show <id>` AND `blacksmith progress show --bead-id <id>` in the SAME turn (Rule A — these are independent of each other). Do NOT run them in separate turns.' This creates a concrete two-step bootstrap that gives agents two forced parallel opportunities at session start, rather than one broken example.

<!-- Promoted from R10 [prompt] -->
- In PROMPT.md line 14, replace '- Session end: `Bash(cargo clippy --fix --allow-dirty)` + `Bash(cargo test --release)` → ONE turn (if they don\'t depend on each other\'s output)' with '- Integration check (step 4c): `Bash(grep -r "fn_name" src/)` + `Bash(grep -r "struct_name" tests/)` → ONE turn (independent search calls)'. Rationale: cargo clippy --fix modifies source files; cargo test reads those same files, so they are NOT parallel-safe. Keeping this wrong example causes agents to correctly detect the contradiction and skip all parallelism.

<!-- Promoted from R11 [prompt] -->
- In PROMPT.md lines 86-90, replace the comment '# Then in ONE turn with TWO parallel Bash calls:' with '# Then sequentially (clippy modifies files that fmt reads):'. Change the two commands on separate lines to show them as sequential, not parallel. Rationale: cargo clippy --fix --allow-dirty writes changes to source files; cargo fmt --check then reads those same files to verify formatting. Running them in parallel risks fmt reading partially-modified files and producing incorrect results.

<!-- Promoted from R12 [prompt] -->
- In PROMPT.md, add to the Turn Budget (R1) section after the 'Turns 1-55' bullet: '**Turn 10 checkpoint**: Count your parallel turns so far (turns where you emitted 2+ tool calls in one message). If the count is 0, you are violating Rule A — immediately re-read Rule A and make your next turn emit at least 2 parallel tool calls. A session that reaches turn 10 with zero parallel calls will almost certainly finish with zero parallel calls.'

<!-- Promoted from R13 [prompt] -->
- Add to PROMPT.md Step 3 (Implement), after 'Only read files you need to modify': '**Batch all file reads in ONE turn (Rule A)**: From the bead description, identify ALL source files you will need upfront. Emit ALL Read calls in a SINGLE turn — never read files one at a time sequentially.'

<!-- Promoted from R14 [prompt] -->
- Change PROMPT.md Steps 1-2 to merge them into a single parallel turn: After bd ready resolves the bead ID, emit in ONE turn: `bd update <id> --status in_progress` + `bd show <id>` + `blacksmith progress show --bead-id <id>` (THREE parallel tool calls). Remove the separate Step 1 and Step 2 headings and replace with: '1. Run `bd ready`, then in ONE turn: `bd update <id> --status in_progress` + `bd show <id>` + `blacksmith progress show --bead-id <id>`'

<!-- Promoted from R15 [prompt] -->
- Add inline Rule B reminders at three transition points in PROMPT.md: (1) At Step 3 header add: '(Rule B: every narration message MUST include a tool call — never emit text-only turns)'; (2) At Step 4 Verify header add the same reminder; (3) At Step 5 Finish header add: '(Rule B: include the blacksmith commands as tool calls in the same message as any narration)'. These hotspots account for 5-8 wasted text-only turns per session.'

<!-- Promoted from R16 [prompt] -->
- Remove the '## Verification' section at the end of PROMPT.md (lines 168-174: 'Before closing a task, run these commands...cargo test, clippy, fmt'). Replace with a one-line note: '## Verification\nAll quality gates are run automatically by `blacksmith finish`. Do not run them manually — Step 4b and blacksmith finish handle this.' Rationale: agents currently run cargo test+clippy+fmt in Step 4b, then again as the standalone Verification section, then blacksmith finish runs them a third time. This triple-execution wastes 3-4 turns per session.

<!-- Promoted from R17 [prompt] -->
- In PROMPT.md ## Turn Budget section, line that reads 'Run the full test suite + `lint:fix` + `analyze`' — replace with: 'Run `cargo test --release`, then call `blacksmith finish` which handles lint and format.' The commands `lint:fix` and `analyze` do not exist in this Rust project and will confuse agents in the wrap-up phase (turns 56-65) when they try to run them and get errors.

<!-- Promoted from R18 [prompt] -->
- Add to PROMPT.md immediately before Rule A section: '**Turn 1 Rule**: Your very first message MUST include a tool call — emit `bd ready` (and optionally `Read MEMORY.md` in parallel) as turn 1. Do NOT write any introductory text before your first tool call. A text-only turn 1 is an immediate Rule B violation and sets a pattern of narration-first that persists all session.'

<!-- Promoted from R19 [prompt] -->
- Add to PROMPT.md under 'Marking a Failed Attempt': 'After marking the failure, always exit via the standard finish sequence: (1) `blacksmith progress add --bead-id <id> --stdin` with your failure summary, (2) `blacksmith finish <id>` — this handles git sync and session cleanup even when nothing was committed. Do NOT exit by just stopping — always call blacksmith finish.'

<!-- Promoted from R20 [cost] -->
- Add to PROMPT.md Step 3 (Implement), before the first bullet: '**Prohibited single-tool turns** (each wastes 1 turn from your budget):\n- Do NOT run `cargo check` alone — `cargo test --release` already compiles everything.\n- Do NOT run `ls`, `find`, or bare `Glob` to explore directories — the project structure is in MEMORY.md and AGENTS.md.\nIf you catch yourself about to run one of these alone, combine it with another necessary call (Rule A) or skip it entirely.'

<!-- Promoted from R21 [cost] -->
- Add to PROMPT.md ## CRITICAL Execution Efficiency Rules section after Rule B: 'Rule D: Do NOT re-read the same file multiple times with incrementally refined scripts. When extracting data from a file (e.g., a session JSONL), write ONE comprehensive script that extracts ALL needed metrics in a single Bash call. Iteratively refining and re-running a script against the same file wastes one turn per iteration.'

<!-- Promoted from R22 [prompt] -->
- Add to PROMPT.md Rule B section, after the existing WRONG/RIGHT example: 'WRONG: "Now I have enough data. Let me synthesize my findings." (text only, no tool call) → action in next turn. RIGHT: Synthesize findings AND emit the first resulting tool call (e.g., blacksmith improve add) in ONE message. Analysis/thinking never justifies a text-only turn.'

<!-- Promoted from R23 [prompt] -->
- Add to PROMPT.md ## Improvement Recording section, before 'Record improvements as you work': '**Rule A applies here too**: When filing 2+ improvements in one session, emit ALL `blacksmith improve add` calls in ONE parallel turn — they are independent of each other.'

<!-- Promoted from R24 [prompt] -->
- Add to PROMPT.md Rule A, after the mandatory parallel patterns list: '**Look-ahead scan**: Before emitting ANY single tool call, ask: "Will I need another independent call in the next 2 turns?" If yes, emit BOTH now in one turn. Never defer an independent call you already know you need.'

<!-- Promoted from R25 [prompt] -->
- Add to PROMPT.md Rule A mandatory parallel patterns list (after 'Multiple greps' line): '- Running independent test subsets: `cargo test --release --filter suite_a` + `cargo test --release --filter suite_b` → ONE turn'

<!-- Promoted from R26 [prompt] -->
- Add to PROMPT.md ## Context Loading or ## Important section: 'When reading session .jsonl files (.blacksmith/sessions/*.jsonl), NEVER use the Read tool — it will fail on files >25000 tokens. Always use Bash+python3: python3 -c "import json; lines=[json.loads(l) for l in open(\".blacksmith/sessions/N.jsonl\")]; ..."'

<!-- Promoted from R27 [prompt] -->
- Add to PROMPT.md Step 5 (Finish) or ## Improvement Recording section, after the bd create example: 'Note: valid --type values for bd create are: task, bug, feature. Do NOT use --type process — it is invalid and will cause an error.'

<!-- Promoted from R28 [prompt] -->
- Add to PROMPT.md ## Improvement Recording section, after existing examples: 'WRONG (Rule B violation): emit text "Now creating beads..." as a standalone turn, then call bd create in the next turn. RIGHT: include narration text AND the first bd create call in the SAME turn. Never emit a standalone "Now creating beads" or "Now filing improvements" message.'

<!-- Promoted from R29 [prompt] -->
- Add to PROMPT.md Improvement Recording section: 'When reviewing open improvements, emit ALL blacksmith improve promote <REF> and blacksmith improve dismiss <REF> calls in ONE parallel turn — they are completely independent.'

<!-- Promoted from R30 [prompt] -->
- Add to PROMPT.md Failed-Attempt Detection section step 2: 'Emit ALL bd create <sub-bead> calls in ONE parallel turn — they are independent of each other. Example: emit bd create for sub-bead-A, bd create for sub-bead-B, and bd create for sub-bead-C in the same turn.'

<!-- Promoted from R31 [prompt] -->
- Add to PROMPT.md Failed-Attempt Detection section step 2: 'Always include --design flag: bd create --type feature --priority 1 "<title>" --design "<description of what this sub-bead should accomplish and why>". Beads without descriptions lack context and generate warnings.'

<!-- Promoted from R32 [prompt] -->
- Add new section '## Analysis Session Workflow' to PROMPT.md (after Rule C): 'If the assigned task is a self-improvement analysis session, follow these parallel patterns: Turn 1: batch Bash(cat session.jsonl | python3 all_metrics.py) + Bash(blacksmith improve list) + Read PROMPT.md in ONE turn (THREE parallel calls). All blacksmith improve add calls MUST be batched in ONE parallel turn. All bd create calls MUST be batched in ONE parallel turn. Never run multiple incremental python3 scripts on the same session file.'

<!-- Promoted from R33 [cost] -->
- Add to PROMPT.md Analysis Session Workflow section: 'Write ONE comprehensive python3 analysis script that extracts ALL needed metrics (tool_call counts, parallel turn counts, narration-only turns, tool name breakdown, thinking-only turns) in a SINGLE Bash call. Example pattern: cat session.jsonl | python3 -c "import sys,json; turns=[json.loads(l) for l in sys.stdin]; tool_turns=sum(1 for t in turns if t.get(type)==assistant and any(c.get(type)==tool_use for c in t.get(message,{}).get(content,[]))); print(tool_turns)". Never run 5+ separate python3 scripts on the same .jsonl file in separate turns.'

<!-- Promoted from R35 [prompt] -->
- Add to PROMPT.md Analysis Session Workflow section: 'Session files are at .blacksmith/sessions/<N>.jsonl — NOT .beads/sessions/. Use this exact path in python3 scripts. Example: open("/home/admin/imagen/.blacksmith/sessions/{session_id}.jsonl"). Never try .beads/sessions/ first.'

<!-- Promoted from R36 [prompt] -->
- Add to PROMPT.md Analysis Session Workflow section: 'Run `blacksmith improve list` WITHOUT any `| head -N` truncation — always use the full output to review ALL open improvements. Truncating at 60 lines risks missing open improvements and filing duplicates.'

<!-- Promoted from R37 [prompt] -->
- Add to PROMPT.md ## Context Loading section, before step 1: '**Analysis sessions only:** Skip this entire section. The analysis prompt provides all needed context. Do NOT run `bd ready`, do NOT read MEMORY.md. Proceed directly to reviewing open improvements.'

<!-- Promoted from R38 [cost] -->
- Add to PROMPT.md ## Improvement Recording section: '**Analysis sessions**: Do NOT read PROMPT.md to assess open improvements. The `blacksmith improve list` output already reflects current prompt state. Only read PROMPT.md when writing the `--body` for a specific improvement that requires checking exact current text — and only ONE targeted read, not exploratory head/wc-l calls.'

<!-- Promoted from R39 [prompt] -->
- Add to PROMPT.md ## Rules section (after Rule B): 'Rule C — No synthesis turns: After receiving ANY tool result, your next turn MUST include at least one tool call, unless you are delivering the final session answer. Never insert a synthesis, analysis, or summary turn between consecutive tool calls. Anti-pattern: running Bash, then a text-only turn summarizing results, then another Bash. Pattern: run Bash, then immediately run next Bash in the same response or the following turn.'

<!-- Promoted from R40 [prompt] -->
- In PROMPT.md ## Analysis Session section, Step 1 (Close resolved improvements), change the instruction to: 'For each open improvement, check whether the PROBLEM BEHAVIOR no longer appears in recent session data. RESOLVED means: the violation is absent from current session metrics. NOT RESOLVED means: you cannot confirm the behavior stopped. DO NOT read PROMPT.md to check if the fix was applied — that wastes 2-3 turns. Judge solely from tool call patterns and session metrics.'

<!-- Promoted from R41 [prompt] -->
- In PROMPT.md ## Analysis Session section, Step 5 (Create beads for approved edits), add after the bd create example: 'CRITICAL: Count your filed improvements. Emit exactly that many bd create calls simultaneously in ONE turn. Never create beads one at a time. If you filed 2 improvements, emit 2 bd create calls in parallel. A missing bead = a missing implementation.'

<!-- Promoted from R42 [cost] -->
- In PROMPT.md ## Analysis Session section, add at top of Step 1: 'SKIP memory reads: Do NOT read MEMORY.md or PROMPT.md. All needed context (session metrics, open improvements) is provided directly in this task prompt. Reading MEMORY.md wastes a turn and the file may not exist.'

<!-- Promoted from R43 [prompt] -->
- In PROMPT.md ## Analysis Session section, add after opening paragraph: 'Rule A applies here: every turn must batch all independent calls. Minimum parallel patterns: (1) python3 metrics extraction + blacksmith improve list in ONE turn, (2) all blacksmith improve add calls in ONE turn, (3) all bd create calls in ONE turn, (4) git add + commit in ONE turn. A session with 0 parallel turns has violated Rule A on every possible turn.'

<!-- Promoted from R44 [prompt] -->
- In PROMPT.md ## Analysis Session section, add at the very top as a CRITICAL callout: 'Turn 1 MUST be tool calls: emit python3 comprehensive session parse script AND `blacksmith improve list` in ONE parallel turn. Turn 1 narration is a Rule A violation. Session 22 wasted Turn 1 with empty narration, delaying data gathering by 8 turns.'

<!-- Promoted from R45 [cost] -->
- In PROMPT.md ## Analysis Session section, Step 2, add a concrete python3 template that parses .blacksmith/sessions/<N>.jsonl in ONE pass and extracts: total messages, assistant turns, tool-call turns, narration-only turns, narration ratio, parallel-tool turns (turns with >1 tool_use block), tool_calls_by_name dict, and first 100 chars of each narration. Prevents the 5-7 incremental re-read scripts pattern seen in sessions 21 and 22 (wastes 4-6 turns and bloats context with repeated tool results).