# Deterministic barrel packing

Implemented 2026-10-10 in `ciq-package::pack_barrel(source, output, Limits)` and:

```sh
ciq-package unpack library.barrel scratch/library
ciq-package pack-barrel scratch/library scratch/library-repacked.barrel
```

Evidence: [SDK consumption and round trips](../investigations/barrel-packing.md). This is a writer for the [canonical barrel profile](barrel-package.md), not a project builder or Jungle interpreter.

## Input and preservation

The input is an ordinary directory containing ordinary `manifest.xml`, `content/manifest.xml`, and `barrel.jungle` files. Both manifests must be **byte identical**, valid supported manifest XML, and declare a barrel identity. The reader permits differing XML with matching identity; this first writer deliberately requires identical copies instead of deciding which to rewrite. XML and Jungle bytes are never regenerated. Source, resources, and unknown ordinary files are copied unchanged.

Input names must be UTF-8 and satisfy the reader's portable path policy, including case collision checks. Links and special files are rejected. Every directory discovered counts toward the input entry limit; files count toward per-member and total decoded-byte limits. Manifests have their separate size cap. Empty directories are emitted; nonempty directories are represented by descendants. Extraction cannot recover whether an original archive explicitly recorded a nonempty directory, so directory record identity is not promised.

The source must remain unchanged while packing. Observed type and size changes are rejected, but filesystem inspection and opening are separate operations. This API does not provide a snapshot or protection against concurrent malicious filesystem replacement.

## ZIP representation

Members use ascending Rust string order of their UTF-8 names, Deflate level 6, DOS timestamp 1980-01-01 00:00:00, ordinary file permissions 0644 and empty directory permissions 0755. No source timestamps, permissions, archive comment, or application-specific extra metadata are copied. Determinism applies to unchanged contents and this pinned ZIP/Deflate implementation; codec upgrades may change compressed bytes.

The compressed ZIP buffer is bounded by `archive_bytes`, including backpatched headers. Before writing any output file, the archive passes the existing reader's preflight, manifest/profile, path, decoded-size and CRC checks. This also enforces the header cap and rejects unsupported ZIP64. The implementation retains a bounded compressed buffer and a reader copy during validation, and streams file contents rather than retaining all decoded members.

## Output lifecycle

The output's parent must exist. The output must be outside the canonical source tree and must be a new file; existing files or symlinks are never overwritten. Invalid input or archive validation leaves no output file. After validation, exclusive creation is followed by writing and flushing. A write/flush failure removes the file created by this operation on a best-effort basis. Publication is not atomic, no durability sync is promised, and process termination can leave a partial output.

No developer key or signature is required for this observed source-barrel format. IQ writing remains a separate stage.
