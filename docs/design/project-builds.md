# Future project builds and Jungle configuration

Status: planned; no Rust Jungle parser or project build driver exists. Recorded
after the user's clarification that the eventual open-source compiler should
work with real application and barrel projects. This is future scope and does
not replace the immediate [package implementation stages](package-crate.md).

## Inputs and boundaries

Read the project's existing Jungle configuration, commonly `monkey.jungle`,
together with its manifest. A future build layer resolves configuration into
explicit source files, resources, library dependencies, target settings, and
exclusions. The Monkey C compiler consumes those resolved inputs; archive
writing remains in `ciq-package`, and signing remains in `ciq-sign`. Decide
crate names and public APIs when implementation begins.

The future compiler should collect declarations across all selected source files
before binding references, allowing forward references and mutual recursion
without requiring dependency-ordered input files. Source file boundaries and
module namespaces are separate concerns; investigate `using`/`import` visibility
rules before implementing cross-file name resolution.

Support two project workflows:

- **Application:** resolve inputs, compile source and resources into a target
  PRG, sign with the supplied key, and optionally export an IQ distribution
  using the package layer.
- **Barrel:** resolve inputs and collect the selected source/resources and
  metadata into the package layout, generating the archive's `barrel.jungle` and
  manifest copies as required by the supported export profile. The observed
  barrel is distributed as source; executable bytecode is produced when a
  consuming application is compiled.

The project Jungle and exported Jungle are distinct inputs/outputs. In our SDK
specimen, export rewrites `manifest.xml` to `content/manifest.xml` and `source`
to `content/source`; see [the barrel profile](../specs/barrel-package.md). The
current `pack-barrel` only consumes an already exported canonical tree and
preserves its Jungle bytes. It does not perform this project-to-package
transformation.

## Investigation and implementation sequence

1. Before implementation, document a supported Jungle grammar and resolution
   rules using bundled Garmin documentation and controlled SDK experiments.
   Investigate variables/substitution, path bases, lists and multiple Jungle
   inputs, setting inheritance/precedence, device overrides, annotations,
   resource personalities/localization, and barrel dependencies. These semantics
   are not yet locally specified.
2. Implement configuration parsing and resolution separately from Monkey C
   parsing. Produce a deterministic resolved build description with diagnostics
   identifying configuration file and location. Report unsupported constructs
   explicitly rather than silently selecting different inputs.
3. Add project-to-barrel export using resolved inputs: define file selection,
   archive path mapping, generated Jungle content, manifest handling, and
   dependency policy. Validate exported member bytes and SDK consumption before
   claiming compatibility.
4. When the compiler and resource generation support the necessary target
   features, connect the same build description to complete application builds
   and IQ export. Keep direct assembly and canonical-tree packing available
   independently.

Acceptance examples should cover a multi-file app, code-only and resource
barrels, an app consuming multiple barrels, device-specific selection, and
different project working directories. Compare resolved inputs and observable
results against Garmin's SDK; archive byte identity is not required.
Shared-source variants and multiple barrel outputs from one repository need
explicit configurations and experiments before promising a particular project
model.

No compiler frontend, Jungle evaluator, or new build crate is introduced by this
plan.
