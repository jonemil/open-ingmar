# Number-array literal inference

SDK 9.2.0 and independently generated fr265 applications agree on all ten lines
of [the example][example], checked 2026-10-13. This extends the existing
[Char construction experiment][char] to Number-valued inputs.

## Observations and limits

**Experimentally observed:** both programs infer usable Number arrays when the
first element is a local, parameter, user/native call, arithmetic result, or a
guarded nullable Number. A guarded String receiver also supplies a converted
Number. Typed array calls and returns retain indexed Number results. Helpers
print `10` then `20` before the completed array is printed; each executes once.
Mixed Char/Number arithmetic yielding Char retains a Char-array contract.

**Implementation checks:** source and saved IR reject mixed elements and
unguarded nullable uses. Source, saved IR, assembly text, and generated unsigned
application paths agree. Candidate inference selects a concrete element tag;
whole-function checking still establishes validity and branch proofs.

**Limits:** these specimens do not establish SDK allocation order, failure
timing, or memory equivalence. The
[lowering experiments](natural-lowering.md#array-construction) cover direct
construction separately. Current accepted elements and contextual inference are
specified in [the source contract](../specs/compiler-subset.md#number-arrays).

## Reproduction

Build/sign the independent program using the example's commands. Copy the
[console wrapper][wrapper] into `scratch/compiler-number-array-literals/sdk.mc`
and create `group.jungle` there:

```text
project.manifest = ../native-names/manifest.xml
base.sourcePath = ../../examples/compiler-number-array-literals;sdk.mc
base.resourcePath = ../native-names/resources
```

Build the SDK specimen:

```sh
CIQ_JUNGLE_FILES="$PWD/scratch/compiler-number-array-literals/group.jungle" \
  sh scripts/build-example.sh scratch/compiler-number-array-literals
```

Run the SDK-produced PRG and independently generated PRG separately:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' \
  python3 scripts/simulator-session.py --prg <program.prg> --observe-seconds 15
```

Save `scratch/simulator/monkeydo.log` after each run; compare complete logs with
each other and the example's expected output. Retained captures are under
`scratch/compiler-number-array-literals/`.

[example]: ../../examples/compiler-number-array-literals/README.md
[char]: compiler-array-literals.md
[wrapper]: compiler-boolean-negation.md#reference-and-specimen
