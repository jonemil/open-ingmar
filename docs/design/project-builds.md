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

## Target constraints and section layout

Proposed boundary: resolve a device and app type for a complete application
build, and supply their constraints to target-dependent selection, resource
generation, and final layout/validation. Shared source, semantic IR, and generic
instruction encoding need not acquire device-specific offsets or require a
device profile for every operation. API validation or conditional input
selection may still need the target before layout.

Keep VM format rules separate from device capacity. Operand widths, address
selectors, and record framing describe encoding; app memory allowances describe
whether a particular target can execute the result. A small memory allowance
must not become an assumed address-field width. The
[string-boundary experiment](../investigations/string-address-boundary.md)
demonstrates this distinction: the same `news 0x01000000` encoding runs after
raising only the simulated watch-app memory allowance.

Do not introduce per-device address regions without evidence that devices use
different VM address formats. First investigate a shared region model. The
external research's high-nibble description suggests DATA addresses in
`0x00000000..0x0fffffff` and CODE addresses in `0x10000000..0x1fffffff`;
the successful `0x01000000` string fits that hypothesis. This is a provisional
interpretation, not a verified 28-bit offset specification. Retain full u32
address representations and distinguish region selectors from offsets. Verify
selector handling and additional offset bits before defining authoring bounds;
unknown addresses must remain preservable numerically.

Device profiles should initially supply capacity and capability constraints,
rather than redefining section regions. Add VM format variants only when
observations establish them. After resolving the shared DATA address rule,
replace the current 24-bit checks consistently across string parsing, DATA
layout, and CODE binding. Keep CODE/method-address changes dependent on their
own evidence.

Layout uses section-relative offsets, rather than placing objects at physical
watch RAM addresses. Two devices with compatible formats and identical selected
inputs may use the same section layout; a smaller budget can cause validation
to reject it instead of requiring different offsets. Target-dependent inputs or
resource encodings can change the layout itself.

Memory accounting needs its own investigation before enforcing precise build
limits. PRG file size is not established as peak runtime memory: loaded DATA,
resources, frame/stack storage, and dynamic objects may have different costs.
Report known static costs and distinguish them from unbounded or estimated
runtime allocation; do not promise that passing a static check guarantees all
execution fits.

Keep low-level assembly/container tools usable without a device profile for
inspection, lossless reconstruction, and format experiments. A complete target
build should validate against its resolved profile and record explicit
experimental capacity overrides. Exact profile fields, accounting rules, and
compatibility checks remain to be specified from evidence.

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
