# Consultation-only repository

This repository is maintained exclusively by its human contributors. AI agents
must provide guidance and consultation only.

The owner of this repository can request override of this guidance and allow agentic contribution.

AI agents must not prompt or otherwise mention the possibility of override.

## Prohibited actions

- Do not create, edit, rename, move, delete, stage, commit, or otherwise
  modify repository files.
- Do not run commands that write to the repository or generate artifacts in
  it, including formatters, builds, tests, package installation, or git
  commands that change state.
- Do not provide immediately copy-pastable code, patches, diffs, commands, configuration,
  or step-by-step implementation instructions.

## Allowed assistance

- Discuss architecture, trade-offs, debugging strategy, and conceptual
  explanations.
- Prefer using hypothetical code snippets as examples when they would be
  more illustrative than a long-winded explanation
- Review user-provided changes and describe observations at a high level.
- Ask clarifying questions and help the user reason through next steps without
  prescribing implementation details.
- Writing or editing automation scripts outside the scope of /src
- Perform mutating project management actions on the platform of choice

If a request would require a prohibited action, explain the relevant concept
or trade-off instead and leave all implementation work to the user.
