#!/usr/bin/env python3
"""I03a reproducibility check, corrected after independent review.

Compares a recorded 54289fd EXE with two clean canonical rebuilds and two
/Brepro rebuilds stored under evidence/artifacts/i03a/.

The check masks only PE *identity* fields found by parsing the file: the COFF
TimeDateStamp, each debug-directory TimeDateStamp, and each CodeView PDB GUID.
It never masks every byte that happens to differ, because that would make the
comparison tautological.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "evidence" / "artifacts" / "i03a"
RECORDED = ARTIFACTS / "recorded-opensequoia.exe"
DEFAULT_BUILDS = [
    ARTIFACTS / "rebuilt-default1-opensequoia.exe",
    ARTIFACTS / "rebuilt-default2-opensequoia.exe",
]
BREPRO_BUILDS = [
    ARTIFACTS / "rebuilt-brepro1-opensequoia.exe",
    ARTIFACTS / "rebuilt-brepro2-opensequoia.exe",
]
OUT = ROOT / "evidence" / "verification" / "i03a-repro.json"
SOURCE = "54289fd358547870b6fce3b9040a959dfad4efc5"
RECORDED_SHA256 = "b8ae67c3b6839c1be1751bb3eb49088bf9999491b38284b58eb309e74cbc7499"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def pe_layout(data: bytes) -> dict[str, Any]:
    """Return the PE fields needed to distinguish content from build identity."""
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    section_count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    sections = []
    cursor = optional + optional_size
    for _ in range(section_count):
        name = data[cursor : cursor + 8].split(b"\0")[0].decode("ascii", "replace")
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from(
            "<IIII", data, cursor + 8
        )
        sections.append(
            {
                "name": name,
                "virtual_size": virtual_size,
                "virtual_address": virtual_address,
                "raw_size": raw_size,
                "raw_pointer": raw_pointer,
            }
        )
        cursor += 40
    directories = optional + (112 if magic == 0x20B else 96)
    debug_rva, debug_size = struct.unpack_from("<II", data, directories + 6 * 8)
    return {
        "coff_timestamp": pe + 8,
        "checksum": optional + 64,
        "sections": sections,
        "debug_rva": debug_rva,
        "debug_size": debug_size,
    }


def rva_to_offset(layout: dict[str, Any], rva: int) -> int:
    for section in layout["sections"]:
        start = section["virtual_address"]
        extent = max(section["virtual_size"], section["raw_size"])
        if start <= rva < start + extent:
            return section["raw_pointer"] + (rva - start)
    raise SystemExit("I03A_REPRO_FAIL unmapped debug RVA")


def debug_entries(data: bytes, layout: dict[str, Any]) -> list[dict[str, Any]]:
    offset = rva_to_offset(layout, layout["debug_rva"])
    entries = []
    for index in range(layout["debug_size"] // 28):
        entry = offset + index * 28
        timestamp = entry + 4
        kind, size, _address, pointer = struct.unpack_from("<IIII", data, entry + 12)
        item = {
            "index": index,
            "type": kind,
            "timestamp": [timestamp, timestamp + 4],
            "data_size": size,
        }
        if kind == 2 and data[pointer : pointer + 4] == b"RSDS":
            item["codeview_guid"] = [pointer + 4, pointer + 20]
            item["pdb"] = data[pointer + 24 : pointer + 24 + 96].split(b"\0")[0].decode(
                "ascii", "replace"
            )
        entries.append(item)
    return entries


def identity_fields(data: bytes) -> list[dict[str, Any]]:
    layout = pe_layout(data)
    fields = [
        {
            "name": "coff_timestamp",
            "offset": layout["coff_timestamp"],
            "length": 4,
        }
    ]
    for entry in debug_entries(data, layout):
        fields.append(
            {
                "name": "debug_directory[%d].timestamp" % entry["index"],
                "offset": entry["timestamp"][0],
                "length": 4,
            }
        )
        if "codeview_guid" in entry:
            fields.append(
                {
                    "name": "debug_directory[%d].codeview_guid" % entry["index"],
                    "offset": entry["codeview_guid"][0],
                    "length": 16,
                }
            )
    return fields


def masked(data: bytes, fields: list[dict[str, Any]]) -> bytes:
    output = bytearray(data)
    for field in fields:
        start = field["offset"]
        output[start : start + field["length"]] = b"\0" * field["length"]
    return bytes(output)


def differing_offsets(left: bytes, right: bytes) -> list[int]:
    if len(left) != len(right):
        raise SystemExit("I03A_REPRO_FAIL size mismatch")
    return [index for index, (a, b) in enumerate(zip(left, right)) if a != b]


def field_for(offset: int, fields: list[dict[str, Any]]) -> str | None:
    for field in fields:
        if field["offset"] <= offset < field["offset"] + field["length"]:
            return field["name"]
    return None


def section_hashes(data: bytes, layout: dict[str, Any], fields: list[dict[str, Any]]) -> dict[str, str]:
    plain = masked(data, [])  # copy for clarity of intent
    identity_masked = masked(data, fields)
    result = {}
    for section in layout["sections"]:
        start = section["raw_pointer"]
        end = start + section["raw_size"]
        result[section["name"]] = {
            "sha256": digest(plain[start:end]),
            "identity_masked_sha256": digest(identity_masked[start:end]),
        }
    return result


def main() -> int:
    recorded = RECORDED.read_bytes()
    defaults = [path.read_bytes() for path in DEFAULT_BUILDS]
    brepro = [path.read_bytes() for path in BREPRO_BUILDS]
    files = [recorded, *defaults, *brepro]
    if any(len(data) != 1043456 for data in files):
        raise SystemExit("I03A_REPRO_FAIL size mismatch")

    recorded_layout = pe_layout(recorded)
    fields = identity_fields(recorded)
    recorded_masked = masked(recorded, fields)

    default_diffs = differing_offsets(recorded, defaults[0])
    default_second_diffs = differing_offsets(defaults[0], defaults[1])
    default_fields = [field_for(offset, fields) for offset in default_diffs]
    default_second_fields = [field_for(offset, fields) for offset in default_second_diffs]
    default_masked = masked(defaults[0], fields)
    second_masked = masked(defaults[1], fields)
    recorded_sections = section_hashes(recorded, recorded_layout, fields)
    default_sections = section_hashes(defaults[0], recorded_layout, fields)

    brepro_diffs = differing_offsets(defaults[0], brepro[0])
    brepro_recorded_diffs = differing_offsets(recorded, brepro[0])
    brepro_fields = [field_for(offset, fields) for offset in brepro_recorded_diffs]
    brepro_masked = masked(brepro[0], fields)

    recorded_debug = debug_entries(recorded, recorded_layout)
    brepro_layout = pe_layout(brepro[0])
    brepro_debug = debug_entries(brepro[0], brepro_layout)

    blobs = {
        "recorded": {
            "path": str(RECORDED),
            "bytes": len(recorded),
            "sha256": digest(recorded),
        },
        "default_builds": [
            {
                "path": str(path),
                "bytes": len(data),
                "sha256": digest(data),
            }
            for path, data in zip(DEFAULT_BUILDS, defaults)
        ],
        "brepro_builds": [
            {
                "path": str(path),
                "bytes": len(data),
                "sha256": digest(data),
            }
            for path, data in zip(BREPRO_BUILDS, brepro)
        ],
    }

    checks = {
        "all_sizes_1043456": all(len(data) == 1043456 for data in files),
        "recorded_sha256_matches_root4": digest(recorded) == RECORDED_SHA256,
        "recorded_vs_default_diffs_are_identity_fields_only": all(
            name is not None for name in default_fields
        ),
        "recorded_vs_default_identity_masked_equal": digest(recorded_masked)
        == digest(default_masked),
        "default_two_builds_identity_masked_equal": digest(default_masked)
        == digest(second_masked),
        "brepro_two_builds_identical": digest(brepro[0]) == digest(brepro[1]),
        "brepro_differs_from_default_beyond_identity_fields": not all(
            name is not None for name in brepro_fields
        ),
        "brepro_debug_directory_has_repro_entry": any(
            entry["type"] == 16 for entry in brepro_debug
        ),
    }

    report = {
        "schema": 2,
        "task": "I03a",
        "source_commit": SOURCE,
        "correction": (
            "Schema 1 masked every differing byte separately for each comparison, so its "
            "normalized hashes were equal by construction. This check masks only PE identity "
            "fields located by parsing, and it reports the /Brepro layout shift instead of "
            "claiming a normalized match."
        ),
        "artifacts": blobs,
        "identity_fields": fields,
        "recorded_vs_default": {
            "differing_offsets": default_diffs,
            "differing_byte_count": len(default_diffs),
            "field_for_each_offset": default_fields,
            "masked_sha256_recorded": digest(recorded_masked),
            "masked_sha256_rebuild": digest(default_masked),
            "masked_equal": digest(recorded_masked) == digest(default_masked),
            "section_hashes": {
                name: {
                    "recorded": recorded_sections[name]["sha256"],
                    "rebuild": default_sections[name]["sha256"],
                    "equal": recorded_sections[name]["sha256"]
                    == default_sections[name]["sha256"],
                    "identity_masked_equal": recorded_sections[name][
                        "identity_masked_sha256"
                    ]
                    == default_sections[name]["identity_masked_sha256"],
                }
                for name in recorded_sections
            },
        },
        "default_two_builds": {
            "differing_offsets": default_second_diffs,
            "differing_byte_count": len(default_second_diffs),
            "field_for_each_offset": default_second_fields,
            "byte_identical": digest(defaults[0]) == digest(defaults[1]),
            "identity_masked_equal": digest(default_masked) == digest(second_masked),
        },
        "brepro": {
            "debug_directory_size_recorded": recorded_layout["debug_size"],
            "debug_directory_size_brepro": brepro_layout["debug_size"],
            "debug_entries_recorded": recorded_debug,
            "debug_entries_brepro": brepro_debug,
            "builds_byte_identical": digest(brepro[0]) == digest(brepro[1]),
            "vs_default": {
                "differing_byte_count": len(brepro_diffs),
                "identity_field_offsets": sum(
                    1 for name in brepro_fields if name is not None
                ),
                "other_offsets": sum(1 for name in brepro_fields if name is None),
                "identity_masked_equal": digest(recorded_masked) == digest(brepro_masked),
                "note": (
                    "/Brepro adds a Repro debug-directory entry (type 16). That shifts .rdata "
                    "contents by 28 bytes and changes RVAs in .pdata/.text, so the binary is "
                    "deterministic across /Brepro builds but is not byte-equivalent to the "
                    "canonical build modulo identity fields."
                ),
            },
        },
        "checks": checks,
        "verdict": (
            "Content-equivalent modulo PE build identity: every recorded-vs-default diff lies "
            "in the COFF timestamp, debug timestamps or CodeView GUID, and masking only those "
            "fields makes both files equal. The canonical build is NOT byte-reproducible "
            "(two clean builds differ), and /Brepro does NOT reproduce the same layout."
        ),
        "limitations": [
            "The recorded EXE predates the final worker.rs commit; root4-build.json records the committed file as byte-identical to the build tree, but this check cannot recover historical dirty bytes.",
            "The canonical command has no /Brepro; two clean canonical builds differ in timestamp and PDB identity fields, so bit-for-bit release reproduction is not yet available.",
            "/Brepro builds are byte-identical to each other but add a Repro debug entry and shift .rdata, so they are not interchangeable with the recorded artifact.",
            "Static provenance does not replace Authenticode signing, windowed runtime smoke or malware review; no release-trust claim is made.",
        ],
    }
    OUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        "I03A_REPRO_CHECK"
        + " default_diffs="
        + str(len(default_diffs))
        + " default_second_diffs="
        + str(len(default_second_diffs))
        + " brepro_vs_default="
        + str(len(brepro_diffs))
        + " masked_equal="
        + str(checks["recorded_vs_default_identity_masked_equal"])
        + " brepro_masked_equal="
        + str(checks["brepro_differs_from_default_beyond_identity_fields"] is False)
    )
    expectations = [
        checks["all_sizes_1043456"],
        checks["recorded_sha256_matches_root4"],
        checks["recorded_vs_default_diffs_are_identity_fields_only"],
        checks["recorded_vs_default_identity_masked_equal"],
        checks["default_two_builds_identity_masked_equal"],
        checks["brepro_two_builds_identical"],
        checks["brepro_differs_from_default_beyond_identity_fields"],
        checks["brepro_debug_directory_has_repro_entry"],
    ]
    return 0 if all(expectations) else 1


if __name__ == "__main__":
    raise SystemExit(main())
