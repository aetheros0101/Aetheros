# AetherOS Components

Workbench UI components will live here. Each subfolder owns a single component family.

```
components/
├── button/
├── input/
├── panel/
├── tabs/
├── tree/
├── toolbar/
├── status/
├── dialog/
└── feedback/
```

**Rule:** Components consume **semantic tokens** (`AetherSurfaces`, `AetherInteraction`,
`AetherAgent`, `AetherGit`, …) — never raw foundation primitives or `ColorScheme` roles
for domain surfaces.

Scaffold component implementations in a later sprint once the workbench shell lands.
