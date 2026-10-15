#!/usr/bin/env python3
"""Audit documented compiler paths and retained example PRGs without running apps.

Keep full diagnostics and exports in a fresh ignored output directory. Unknown
opcodes are discovered only at decoder-confirmed or recorded source-line PCs;
the audit never guesses widths or scans arbitrary bytes as instructions.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shlex
import struct
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree(root):
    return {str(p.relative_to(root)): digest(p)
            for p in root.rglob("*") if p.is_file()}


def sections(path):
    data = path.read_bytes()
    at = 0
    result = {}
    while at + 8 <= len(data):
        kind, size = struct.unpack_from(">II", data, at)
        at += 8
        if kind == 0:
            if size or at != len(data):
                raise ValueError("invalid PRG terminator")
            return result
        if at + size > len(data) or kind in result:
            raise ValueError("truncated or duplicate PRG section")
        result[kind] = data[at:at + size]
        at += size
    raise ValueError("missing PRG terminator")


def commands(directory):
    text = (directory / "README.md").read_text().replace("\\\n", " ")
    found = []
    for line in text.splitlines():
        match = re.search(r"cargo run(?: -q)? -p ciq-compiler -- (.+)", line)
        if not match:
            continue
        args = shlex.split(match[1])
        sources = [x for x in args if x.endswith(".mc") and Path(x).is_file()]
        if sources and all(Path(x).parent == directory for x in sources):
            found.append(args)
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--bin-dir", type=Path, default=Path("target/debug"))
    parser.add_argument("--sdk-index", type=Path,
                        help="JSON mapping example names to freshly built SDK PRGs")
    parser.add_argument("--example", action="append",
                        help="audit only this example directory name; repeat to select several")
    parser.add_argument("--verbose", action="store_true",
                        help="print progress for each example")
    options = parser.parse_args()
    directories = sorted(p for p in Path("examples").iterdir()
                         if p.is_dir() and (p / "README.md").is_file())
    if options.example:
        unknown = set(options.example) - {p.name for p in directories}
        if unknown:
            parser.error(f"unknown examples: {', '.join(sorted(unknown))}")
        directories = [p for p in directories if p.name in options.example]
    output = options.output
    output.mkdir(parents=True, exist_ok=False)
    bins = {name: (options.bin_dir / name).resolve()
            for name in ["ciq-compiler", "ciq-dis", "ciq-asm", "ciq-package"]}
    for binary in bins.values():
        if not binary.is_file():
            raise SystemExit(f"Build the workspace tools first: missing {binary}")
    sdk_index = json.loads(options.sdk_index.read_text()) if options.sdk_index else {}

    def run(tool, args, log):
        result = subprocess.run([str(bins[tool]), *map(str, args)],
                                capture_output=True, text=True, timeout=60)
        log.write_text(result.stdout + result.stderr)
        return {"exit": result.returncode, "stderr": result.stderr}

    def inspect_prg(path, destination):
        destination.mkdir()
        result = {"path": str(path), "sha256": digest(path)}
        exported = run("ciq-dis", ["all", path, destination / "export"],
                       destination / "export.log")
        result["export"] = exported
        if exported["exit"]:
            return result
        manifest = destination / "export" / "sections.ciq"
        entries = [shlex.split(line) for line in manifest.read_text().splitlines()
                   if line.startswith("section ")]
        result["raw_sections"] = [entry[1] for entry in entries if entry[3] == "raw"]
        result["code_structured"] = any(int(e[1], 16) == 0xc0debabe and e[3] == "assembly" for e in entries)
        result["data_structured"] = any(
            int(e[1], 16) == 0xda7ababe and e[3] == "data" and
            not any(line.lstrip().startswith(".bytes ") for line in
                    (manifest.parent / e[4]).read_text().splitlines())
            for e in entries)
        rebuilt = destination / "rebuilt.prg"
        assembled = run("ciq-asm", ["project", manifest.parent, rebuilt],
                        destination / "rebuild.log")
        result["rebuild"] = assembled
        result["byte_identical"] = assembled["exit"] == 0 and digest(path) == digest(rebuilt)
        result["export_identical"] = False
        if assembled["exit"] == 0:
            repeated = run("ciq-dis", ["all", rebuilt, destination / "reexport"],
                           destination / "reexport.log")
            result["reexport"] = repeated
            result["export_identical"] = (repeated["exit"] == 0 and
                                          tree(manifest.parent) == tree(destination / "reexport"))
        payloads = sections(path)
        data = payloads.get(0xda7ababe, b"")
        result["unknown_data_records"] = []
        for offset in re.findall(r"unknown DATA record at 0x([0-9a-f]+)", exported["stderr"]):
            at = int(offset, 16)
            if at < len(data):
                result["unknown_data_records"].append({"offset": at, "byte": data[at]})
        code = payloads.get(0xc0debabe, b"")
        pcs = {0} if code else set()
        for entry in entries:
            if entry[3] == "lines":
                text = (manifest.parent / entry[4]).read_text()
                pcs.update(int(pc, 16) - 0x10000000 for pc in
                           re.findall(r"\.line pc=(0x[0-9a-fA-F]+)", text)
                           if 0x10000000 <= int(pc, 16) < 0x10000000 + len(code))
        unknown = {}
        tail = destination / "tail.prg"
        for pc in sorted(pcs):
            tail.write_bytes(struct.pack(">II", 0xc0debabe, len(code) - pc) + code[pc:] + b"\0" * 8)
            decoded = subprocess.run([str(bins["ciq-dis"]), "code", str(tail)],
                                     capture_output=True, text=True, timeout=60)
            match = re.search(r"code offset 0x([0-9a-f]+): unsupported opcode", decoded.stderr)
            if match:
                at = pc + int(match[1], 16)
                unknown[at] = code[at]
        result["unknown_opcodes"] = [{"offset": at, "byte": byte}
                                     for at, byte in sorted(unknown.items())]
        (destination / "opcode-boundaries.json").write_text(json.dumps(result["unknown_opcodes"], indent=2) + "\n")
        return result

    rows = []
    for directory in directories:
        home = output / directory.name
        home.mkdir()
        row = {"example": directory.name, "source": [], "packages": []}
        documented = commands(directory)
        if not documented:
            files = [p for p in sorted(directory.rglob("*.mc")) if "build" not in p.parts]
            if files:
                row["frontend_probe"] = run("ciq-compiler", ["--unit", "--emit", "assembly", *files, home / "frontend.mcasm"], home / "frontend.log")
        for index, args in enumerate(documented):
            job = home / f"source-{index}"
            job.mkdir()
            source = {"command": args}
            artifact = job / Path(args[-1]).name
            source["compile"] = run("ciq-compiler", [*args[:-1], artifact], job / "compile.log")
            base = []
            at = 0
            while at < len(args) - 1:
                arg = args[at]
                if arg in ["--emit", "--from", "--app-id", "--entry"]:
                    at += 2
                else:
                    base.append(arg)
                    at += 1
            files = [arg for arg in base if arg.endswith(".mc")]
            flags = [arg for arg in base if arg not in files]
            ir = job / "source.ciqir"
            source["ir"] = run("ciq-compiler", [*base, "--emit", "ir", ir], job / "ir.log")
            if source["ir"]["exit"] == 0:
                direct = job / "direct.mcasm"
                resumed = job / "resumed.mcasm"
                source["assembly"] = run("ciq-compiler", [*base, "--emit", "assembly", direct], job / "assembly.log")
                source["resume"] = run("ciq-compiler", [*flags, "--from", "ir", "--emit", "assembly", ir, resumed], job / "resume.log")
                source["assembly_identical"] = (source["assembly"]["exit"] == source["resume"]["exit"] == 0 and digest(direct) == digest(resumed))
            if source["compile"]["exit"] == 0 and artifact.suffix == ".prg":
                prg_flags = [arg for arg in args[:-1] if arg not in files]
                resumed_prg = job / "resumed.prg"
                source["resume_prg"] = run("ciq-compiler", [*prg_flags, "--from", "ir", ir, resumed_prg], job / "resume-prg.log")
                source["prg_identical"] = source["resume_prg"]["exit"] == 0 and digest(artifact) == digest(resumed_prg)
                source["prg"] = inspect_prg(artifact, job / "prg")
            row["source"].append(source)
        retained = list(directory.glob("build/**/*.prg"))
        retained += list((Path("scratch") / directory.name).glob("**/build/fr265/*.prg"))
        retained.sort(key=lambda p: (len(p.parts), str(p)))
        selected = sdk_index.get(directory.name) or (str(retained[0]) if retained else None)
        selected = selected if isinstance(selected, list) else ([selected] if selected else [])
        row["sdk"] = inspect_prg(Path(selected[0]), home / "sdk") if selected else None
        row["sdk_additional"] = [inspect_prg(Path(path), home / f"sdk-{index}")
                                 for index, path in enumerate(selected[1:], 1)]
        for index, package in enumerate(sorted(directory.glob("build/*.barrel"))):
            pkg = home / f"package-{index}"
            pkg.mkdir()
            report = {"path": str(package), "sha256": digest(package)}
            report["list"] = run("ciq-package", ["list", package], pkg / "list.log")
            report["unpack"] = run("ciq-package", ["unpack", package, pkg / "original"], pkg / "unpack.log")
            if report["unpack"]["exit"] == 0:
                report["pack"] = run("ciq-package", ["pack-barrel", pkg / "original", pkg / "repacked.barrel"], pkg / "pack.log")
                if report["pack"]["exit"] == 0:
                    report["reopen"] = run("ciq-package", ["unpack", pkg / "repacked.barrel", pkg / "resumed"], pkg / "reopen.log")
                    report["members_identical"] = report["reopen"]["exit"] == 0 and tree(pkg / "original") == tree(pkg / "resumed")
            row["packages"].append(report)
        rows.append(row)
        (output / "report.json").write_text(json.dumps(rows, indent=2) + "\n")
        if options.verbose:
            print(f"{directory.name}: source={len(row['source'])} sdk={'checked' if row['sdk'] else 'missing'}", flush=True)
    issues = []
    for row in rows:
        if not row["sdk"] and not row["packages"]:
            issues.append(f"{row['example']}: no SDK specimen")
        for label, specimen in [("SDK", row["sdk"]), *[(f"SDK-{n}", s) for n, s in enumerate(row["sdk_additional"], 1)], *[(f"source-{n}", s.get("prg")) for n, s in enumerate(row["source"])]]:
            if specimen and (specimen["export"]["exit"] or not specimen.get("byte_identical") or not specimen.get("export_identical") or not specimen.get("code_structured") or not specimen.get("data_structured")):
                issues.append(f"{row['example']} {label}: incomplete export or reconstruction")
        for source in row["source"]:
            if source["compile"]["exit"] or not source.get("assembly_identical") or ("prg" in source and not source.get("prg_identical")):
                issues.append(f"{row['example']}: source/text pipeline failure")
        for package in row["packages"]:
            if not package.get("members_identical"):
                issues.append(f"{row['example']}: package member round trip failed")
    (output / "issues.txt").write_text("\n".join(issues) + "\n")
    opcodes = {}
    records = {}
    for row in rows:
        specimens = ([row["sdk"]] if row["sdk"] else []) + row["sdk_additional"]
        for specimen in specimens:
            for item in specimen.get("unknown_opcodes", []):
                opcodes.setdefault(f"0x{item['byte']:02x}", set()).add(row["example"])
            for item in specimen.get("unknown_data_records", []):
                records.setdefault(f"0x{item['byte']:02x}", set()).add(row["example"])
    for label, groups in [("Opcode", opcodes), ("DATA record", records)]:
        for byte, examples in sorted(groups.items()):
            print(f"{label} {byte}: {len(examples)} example groups")
    print(f"Audited {len(rows)} examples; {len(issues)} coverage issues. See {output / 'report.json'}.")
    return bool(issues)


if __name__ == "__main__":
    raise SystemExit(main())
