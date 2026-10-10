# Unchanged modern developer IQ repacking

Implemented in `ciq-package::repack_iq(source, output, Limits)` and:

```sh
ciq-package unpack application.iq scratch/application
ciq-package repack-iq scratch/application scratch/unchanged.iq
ciq-package verify scratch/unchanged.iq --expected-key scratch/application/dev_key.pub
```

The source is an extracted IQ tree with its existing manifest, public key, signatures, product PRGs, debug files, and any unknown ordinary files. All ordinary file bytes are retained. No private key is loaded, signatures are never regenerated, and manifest XML is not reserialized. Changed signed inputs that invalidate verification are refused. The operation cannot prove equality to an unavailable original archive; unchanged means that the writer itself does not edit the supplied files. Unsigned extras are preserved but remain outside signature coverage.

The output is a deterministic 7z archive: sorted UTF-8 paths, independent single-thread LZMA2 streams at preset 6, no filesystem timestamps or attributes, and the pinned writer's ordinary or single-LZMA encoded header. No encryption is enabled. Empty files receive data streams. Nonempty directories are represented by descendants; **empty directories are explicitly unsupported** because the pinned backend incorrectly emits anti-item flags for streamless entries. This limitation is recorded in [the investigation](../investigations/iq-repacking.md), and does not change reader support for ordinary directories.

Input path, entry-count, member/total-size, manifest-size, source stability, and new-file publication rules are shared with [barrel packing](barrel-packing.md). Output must be outside the source tree, with an existing parent. The compressed output buffer is bounded, codec dictionaries must fit the supplied dictionary cap, and final archive/header limits are checked before publication. Fixed encoder working memory is additional to dictionary storage; limits are not a total-process-memory or CPU-time guarantee.

Before output creation, the archive is reopened and the complete modern developer [verification profile](package-verification.md) must pass, including decoded sizes/CRCs, exact-byte manifest signatures, product attributes, PRG signatures, and key consistency. Older, store, unsigned, and otherwise unsupported signature profiles are rejected. Existing output files/symlinks are never replaced. Write failure cleanup is best effort; process termination can leave a partial output.

Encoded-header reading permits only one internal LZMA stream with bounded compressed and decoded header sizes, a bounded dictionary, and a decoded CRC. Start/next-header and present packed-header CRCs are checked. Decoded metadata passes the existing preflight before archive-backend allocation. Nested encoding, other header codecs, graphs, external metadata, and encryption remain unsupported. Internally the decoder reframes a raw header; this can add up to the header cap to the bounded archive buffer. It does not alter the archive written to disk or any extracted file bytes.

Successful verification establishes integrity under the embedded key. Caller-supplied expected public keys establish identity relative to that key. Garmin store/upload/install acceptance of this new archive representation remains untested.
