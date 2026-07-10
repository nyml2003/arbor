---
name: arbor-agent-team
description: Arbor project-local rules for OpenCode and oh-my-opencode-slim agent team orchestration. Use when planning, delegating, reviewing, or implementing work in this repository with Orchestrator, Explorer, Librarian, Fixer, Designer, Oracle, or Council.
---

# Arbor Agent Team

Use this skill as the project-local operating rule for OpenCode agent teams in Arbor.

## Core Rule

Orchestrator owns planning, delegation, integration, and verification. Specialist agents own bounded subtasks. Do not let multiple writer agents edit the same files at the same time.

## Agent Routing

- Use Explorer for repository mapping, file discovery, call-chain tracing, and impact analysis.
- Use Librarian for external documentation, API behavior, SDK usage, and version-sensitive facts.
- Use Fixer for scoped implementation, failing tests, type errors, and mechanical follow-up.
- Use Designer for UI/UX structure, visual hierarchy, interaction feel, and frontend polish.
- Use Oracle for architecture review, risky debugging, plan challenge, and final review.
- Use Council only for high-cost decisions where competing approaches both have material risk.

## Write Ownership

- Explorer, Librarian, Oracle, and Council are read-only by default.
- Fixer may write code only when the file scope or behavior target is clear.
- Orchestrator may write small glue changes, but should delegate larger implementation.
- If two tasks touch the same file or module, serialize them.
- Use worktrees for parallel high-risk implementation lanes.

## Default Workflow

For small tasks:

1. Ask Explorer for the relevant code map when the impact area is unclear.
2. Delegate one scoped implementation lane to Fixer.
3. Run the narrowest meaningful verification.
4. Ask Oracle only when the change is risky, shared, or architectural.
5. Report changed files, validation, and remaining risk.

For large tasks:

1. Start with `/deepwork`.
2. Record the plan under `.slim/deepwork/`.
3. Ask Explorer and Librarian for confirmed context.
4. Ask Oracle to review the plan before implementation.
5. Execute by phases with clear file ownership.
6. Validate after each phase.
7. Ask Oracle for final review before completion.

## Arbor Repository Constraints

- Keep changes small, reviewable, and reversible.
- Prefer deletion and reuse over new layers.
- Do not add dependencies without a strong reason.
- Keep pure domain logic separate from CLI, UI, IPC, and driver code.
- Match neighboring code style before introducing a new pattern.
- Run tests, typecheck, build, or lint according to the touched area.

## Local Skill Policy

- Project-specific OpenCode skills live under `.agents/skills/<name>/SKILL.md`.
- Do not treat downloaded community skills as Arbor policy.
- Convert repeated workflow friction into a small local skill only when it will be reused.
- Keep each skill short. Put detailed material in `references/` only when needed.

