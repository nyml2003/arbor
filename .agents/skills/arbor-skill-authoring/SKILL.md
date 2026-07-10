---
name: arbor-skill-authoring
description: Create or update local Arbor OpenCode skills under .agents/skills. Use when the user wants to write personal/project skills, turn repeated workflow rules into SKILL.md files, or adjust local agent behavior without installing online skill packs.
---

# Arbor Skill Authoring

Create local OpenCode skills under `.agents/skills`. Do not install or fetch community skill packs for Arbor policy unless the user explicitly asks.

## Structure

Use this shape:

```text
.agents/skills/<skill-name>/
  SKILL.md
  references/        optional
  scripts/           optional
  assets/            optional
```

`SKILL.md` must contain only `name` and `description` in front matter.

## Writing Rules

- Make the description specific. It is the trigger surface.
- Keep `SKILL.md` short and procedural.
- Put long details in `references/`.
- Do not add README files unless a tool requires them.
- Prefer one narrow skill over one broad vague skill.
- Do not duplicate existing built-in agents. Write routing rules instead.

## Good Skill Candidates

- Repeated Arbor repository rules.
- Repeated review checklists.
- Repeated validation sequences.
- Project-specific architecture boundaries.
- Personal agent-team preferences that should survive sessions.

## Bad Skill Candidates

- One-off task notes.
- Temporary bug context.
- Generic advice the model already knows.
- Large copied documentation.
- Rules that conflict with the repository's existing conventions.

## Validation

After editing a local skill:

1. Check the folder name matches the front matter `name`.
2. Check front matter has only `name` and `description`.
3. Read the description and confirm it says when to use the skill.
4. Confirm the skill can be used without hidden context from the current chat.

