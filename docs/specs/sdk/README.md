# SDK declarations and profiles

- [Declaration model](declarations.md): API catalogue, availability, inheritance,
  assignability, and remaining core-library contracts.
- [API context](context.md): explicit target/minimum API checking inputs.
- [Native profiles](profiles.md): higher-level module calls, Views, and drawing.
- [Callbacks](callbacks.md): supported native lifecycle/source overrides.

Core Lang value and container behavior lives with the
[frontend semantics](../frontend/README.md). Declarations contain signatures and
availability, not implementations; executable support is checked separately.
Project permissions, scope, and device selection remain
[build requirements](../build/project-builds.md).
