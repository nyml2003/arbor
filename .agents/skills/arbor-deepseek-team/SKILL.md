---
name: arbor-deepseek-team
description: DeepSeek-first model routing and cost-control rules for Arbor's OpenCode oh-my-opencode-slim team. Use when configuring or running DeepSeek-backed Orchestrator, Explorer, Librarian, Fixer, Designer, Oracle, or Council agents.
---

# Arbor DeepSeek Team

Use DeepSeek as the default cost-effective team model. Escalate only when the task needs stronger judgment than the active DeepSeek model provides.

## Model Routing

- Use DeepSeek reasoner-class models for Orchestrator and Oracle.
- Use DeepSeek chat-class models for Explorer, Librarian, Fixer, and Designer.
- Use the cheapest reliable DeepSeek model for read-only exploration.
- Use the strongest available DeepSeek model for plan review, architecture, and debugging.
- Keep Council disabled in normal work. Use it only for expensive architectural choices.

## Delegation Rules

- Start with Explorer when the repository area is unknown.
- Use Fixer for one bounded implementation lane at a time.
- Use Oracle before and after risky phases.
- Do not ask Council to solve routine bugs, lint failures, or small refactors.
- If a DeepSeek response is vague, ask for file-grounded evidence before continuing.

## Cost Control

- Prefer one Explorer pass over repeated broad searches.
- Keep Librarian prompts specific. Ask for official docs or exact API behavior.
- Keep Fixer tasks small enough to validate quickly.
- Ask Oracle for a focused review question, not a general opinion.
- Use `/deepwork` only for multi-file, risky, or staged work.

## Verification

Before declaring completion:

- Confirm the final answer is grounded in files or command output.
- Run the narrowest meaningful verification command.
- State what was not tested.
- Include remaining risk when model uncertainty remains.

