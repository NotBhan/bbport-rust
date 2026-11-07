#!/usr/bin/env python3
"""Generate `gpu/shader_recompiler/frontend/format_gen.rs` from the shadPS4 C++ tables.

`format.cpp` is 4,414 lines of `InstFormat` tables, one entry per opcode of each encoding with
the opcode named in a comment. The generator copies them across unchanged: same arrays, same
lengths, same order, same comments.

Usage: python3 tools/gen_gcn_formats.py [--check]
"""

from __future__ import annotations

import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "legacy/gpu/shadps4/shader_recompiler/frontend/format.cpp"
TARGET = ROOT / "gpu/shader_recompiler/frontend/format_gen.rs"

ARRAY_RE = re.compile(r"constexpr std::array<InstFormat, (\d+)> (\w+) = \{")


def matching_brace(text: str, start: int) -> int:
    """The index of the brace that closes the one at `start`."""
    depth = 0
    index = start
    while True:
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1


def parse_arrays(text: str) -> list[tuple[str, int, list[tuple[list[str], str | None]]]]:
    """Every table, as (name, length, [(comments, entry)]) with `None` for an empty entry."""
    arrays: list[tuple[str, int, list[tuple[list[str], str | None]]]] = []
    for match in ARRAY_RE.finditer(text):
        length = int(match.group(1))
        name = match.group(2)
        # Find the end of the initialiser by brace matching from its first `{`.
        start = text.index("{", match.end() - 1)
        end = start
        depth = 0
        while True:
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            end += 1
        # The initialiser is written either `= { ... }` or `= {{ ... }}`; a second wrapper
        # brace is dropped here so the entries sit at depth zero for the entry splitter.
        body = text[start + 1 : end - 1].strip()
        if body.startswith("{"):
            close = matching_brace(text, text.index("{", start + 1))
            if not text[close + 1 : end - 1].strip():
                body = text[text.index("{", start + 1) + 1 : close].strip()
        arrays.append((name, length, split_entries(body)))
    return arrays


def split_entries(body: str) -> list[tuple[list[str], str | None]]:
    """The entries of an array initialiser, each with the comments that precede it."""
    entries: list[tuple[list[str], str | None]] = []
    comments: list[str] = []
    current: str | None = None
    depth = 0
    index = 0
    while index < len(body):
        if body.startswith("//", index):
            end_of_line = body.find("\n", index)
            if end_of_line < 0:
                end_of_line = len(body)
            comments.append(body[index:end_of_line].strip())
            index = end_of_line
            continue
        char = body[index]
        if char == "{":
            depth += 1
            if depth == 1:
                current = ""
        elif char == "}":
            depth -= 1
            if depth == 0:
                text = (current or "").strip()
                entries.append((comments, None if not text else text))
                comments = []
                current = None
        elif current is not None and depth == 1 and char not in "\n\r":
            current += char
        index += 1
    return entries


def to_rust_entry(entry: str | None) -> str:
    if entry is None:
        return "InstFormat::EMPTY"
    fields = [field.strip() for field in entry.split(",") if field.strip()]
    assert len(fields) == 6, f"unexpected entry {entry!r}"
    names = [
        "inst_class",
        "inst_category",
        "src_count",
        "dst_count",
        "src_type",
        "dst_type",
    ]
    inner = ", ".join(f"{name}: {value}" for name, value in zip(names, fields))
    return f"InstFormat {{ {inner} }}"


def generate() -> str:
    arrays = parse_arrays(SOURCE.read_text())
    lines = [
        "//! GCN instruction format tables, generated from",
        "//! `legacy/gpu/shadps4/shader_recompiler/frontend/format.cpp` by",
        "//! `tools/gen_gcn_formats.py`: the same 17 arrays, the same lengths, the same entries in",
        "//! the same order, with the header's own opcode comments kept beside them. An entry the",
        "//! C++ leaves as `{}` is `InstFormat::EMPTY` here.",
        "",
        "// The tables are `const` because the C++ declares them `constexpr`; clippy would",
        "// rather the largest ones were `static`.",
        "#![allow(clippy::large_const_arrays)]",
        "",
        "use super::opcodes::{InstCategory, InstClass, ScalarType};",
        "use super::decode::InstFormat;",
        "",
    ]
    for name, length, entries in arrays:
        # `InstructionFormatSOP2` becomes `INSTRUCTION_FORMAT_SOP2`.
        rust_name = "INSTRUCTION_FORMAT_" + name.removeprefix("InstructionFormat").upper()
        lines.append(f"/// `{name}`, with `{length}` entries.")
        lines.append(f"pub const {rust_name}: [InstFormat; {length}] = [")
        for comments, entry in entries:
            for comment in comments:
                lines.append(f"    {comment}")
            lines.append(f"    {to_rust_entry(entry)},")
        lines.append("];")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def format_rust(text: str) -> str:
    rustfmt = shutil.which("rustfmt")
    if rustfmt is None:
        return text
    with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as handle:
        handle.write(text)
        path = handle.name
    try:
        subprocess.run([rustfmt, "--edition", "2021", path], check=True)
        return pathlib.Path(path).read_text()
    finally:
        pathlib.Path(path).unlink()


def main() -> int:
    generated = format_rust(generate())
    if "--check" in sys.argv:
        if TARGET.read_text() != generated:
            print(f"{TARGET} is out of date", file=sys.stderr)
            return 1
        print(f"{TARGET} is up to date")
        return 0
    TARGET.write_text(generated)
    print(f"wrote {TARGET} ({len(generated.splitlines())} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
