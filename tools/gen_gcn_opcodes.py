#!/usr/bin/env python3
"""Generate `gpu/shader_recompiler/frontend/opcodes_gen.rs` from the shadPS4 C++ header.

The C++ side spells its GCN enumerations as `enum class X : u32`. Three of them (and only
those three) give two enumerators the same value, which a Rust `enum` rejects, so the port
emits a `u32` newtype with associated constants for every one of them: same names, same
values, same declaration order, aliases included.

Usage: python3 tools/gen_gcn_opcodes.py [--check]
"""

from __future__ import annotations

import collections
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "legacy/gpu/shadps4/shader_recompiler/frontend/opcodes.h"
TARGET = ROOT / "gpu/shader_recompiler/frontend/opcodes_gen.rs"

ENUM_RE = re.compile(r"enum class (\w+) : u32 \{")
NUMBER_RE = re.compile(r"(0x[0-9A-Fa-f]+|\d+)[uUlL]+")
TOKEN_RE = re.compile(r"(\(u32\))?\s*([A-Za-z_]\w*)(::([A-Za-z_]\w*))?|[-+<<|&^()0-9|]+")


def parse_enums(text: str) -> collections.OrderedDict[str, list[tuple[str, str | None]]]:
    """Every `enum class X : u32 { ... }` in the header, as (name, expression) pairs."""
    enums: collections.OrderedDict[str, list[tuple[str, str | None]]] = collections.OrderedDict()
    position = 0
    while True:
        match = ENUM_RE.search(text, position)
        if not match:
            return enums
        end = match.end()
        depth = 1
        while depth:
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
            end += 1
        body = text[match.end() : end - 1]
        enums[match.group(1)] = split_entries(body)
        position = end


def split_entries(body: str) -> list[tuple[str, str | None]]:
    body = "\n".join(line[: line.find("//")] if "//" in line else line for line in body.splitlines())
    entries: list[str] = []
    current = ""
    depth = 0
    for char in body:
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        if char == "," and depth == 0:
            if current.strip():
                entries.append(current.strip())
            current = ""
        else:
            current += char
    if current.strip():
        entries.append(current.strip())
    out: list[tuple[str, str | None]] = []
    for entry in entries:
        name, _, expression = entry.partition("=")
        out.append((name.strip(), expression.strip() or None))
    return out


class Values:
    """C++ enumerator values, evaluated the way the compiler would."""

    def __init__(self, enums: collections.OrderedDict[str, list[tuple[str, str | None]]]) -> None:
        self.enums = enums
        self.names = {name: [entry[0] for entry in entries] for name, entries in enums.items()}
        self.values: dict[tuple[str, str], int] = {}

    def of(self, enum: str, variant: str) -> int:
        key = (enum, variant)
        if key in self.values:
            return self.values[key]
        index = self.names[enum].index(variant)
        expression = self.enums[enum][index][1]
        if expression is None:
            self.values[key] = 0 if index == 0 else self.of(enum, self.names[enum][index - 1]) + 1
        else:
            self.values[key] = self._eval(expression, enum)
        return self.values[key]

    def _eval(self, expression: str, enum: str) -> int:
        expression = NUMBER_RE.sub(r"\1", expression.replace("(u32)", ""))
        pieces: list[str] = []
        for match in TOKEN_RE.finditer(expression):
            if match.group(4):
                pieces.append(str(self.of(match.group(2), match.group(4))))
            elif match.group(2) and match.group(2) in self.names[enum]:
                pieces.append(str(self.of(enum, match.group(2))))
            else:
                pieces.append(match.group(0))
        return eval("".join(pieces))


def to_rust(expression: str, enum: str, variants: list[str]) -> str:
    """C++ enumerator arithmetic in Rust: casts become `as u32`, names get qualified."""
    # A C++ literal suffix (`1u`, `0x1FFULL`) has no Rust meaning.
    expression = NUMBER_RE.sub(r"\1", expression)
    # A cast to `u32` is the newtype's field, and a bare enumerator of the same table is one too.
    out = re.sub(r"\(u32\)\s*([A-Za-z_]\w*)::([A-Za-z_]\w*)", r"\1::\2.0", expression)
    out = re.sub(r"\(u32\)\s*([A-Za-z_]\w*)", r"\1.0", out)
    for variant in sorted(variants, key=len, reverse=True):
        out = re.sub(
            r"(?<![:A-Za-z0-9_])" + re.escape(variant) + r"(?![:\w])",
            f"{enum}::{variant}.0",
            out,
        )
    return out


def name_table(enum: str, entries: list[tuple[str, str | None]]) -> list[str]:
    rows = [f'    ("{name}", {enum}::{name}),' for name, _ in entries]
    return [
        f"/// `magic_enum::enum_name` for a `{enum}`, in declaration order.",
        f"pub const {snake(enum).upper()}_NAMES: &[(&str, {enum})] = &[",
        *rows,
        "];",
        "",
        f"/// The first name declared with this `{enum}` value, as `magic_enum` reports it.",
        f"pub fn {snake(enum)}_name(value: {enum}) -> &'static str {{",
        f"    for (name, named) in {snake(enum).upper()}_NAMES {{",
        "        if *named == value {",
        "            return name;",
        "        }",
        "    }",
        '    ""',
        "}",
        "",
    ]


def snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def generate() -> str:
    text = SOURCE.read_text()
    enums = parse_enums(text)
    values = Values(enums)
    lines = [
        "//! GCN enumerations, generated from",
        "//! `legacy/gpu/shadps4/shader_recompiler/frontend/opcodes.h` by `tools/gen_gcn_opcodes.py`.",
        "//! The header's 34 `enum class ... : u32` tables appear here with the same names, the same",
        "//! enumerators in the same order and the same values.",
        "//!",
        "//! The C++ uses an `enum class`, which allows two enumerators to share a value. A Rust",
        "//! `enum` does not, and three of these tables rely on that (`OpcodeVOP3` and `OpcodeVOPC`",
        "//! alias several instruction encodings, and the unified `Opcode` inherits those aliases),",
        "//! so the port models every table as a `u32` newtype with associated constants. Reading the",
        "//! value is `.0`, and the constants work in `match` patterns.",
        "",
        "// The standalone expressions below are the C++\'s own: `0 + OP_MAP_SOP2` and",
        "// `0x00000000u << 31` are how the header spells the first entries of a table.",
        "#![allow(non_upper_case_globals)]",
        "#![allow(clippy::identity_op)]",
        "",

    ]
    for name, entries in enums.items():
        lines.append(f"/// `Shader::Gcn::{name}`.")
        lines.append("#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]")
        lines.append("#[repr(transparent)]")
        lines.append(f"pub struct {name}(pub u32);")
        lines.append("")
        lines.append(f"impl {name} {{")
        for variant, expression in entries:
            if expression is None:
                lines.append(f"    pub const {variant}: {name} = {name}({values.of(name, variant)});")
            else:
                lines.append(
                    f"    pub const {variant}: {name} ="
                    f" {name}({to_rust(expression, name, values.names[name])});"
                )
        lines.append("}")
        lines.append("")
    for enum in ("Opcode", "InstCategory"):
        lines.extend(name_table(enum, enums[enum]))
    return "\n".join(lines).rstrip() + "\n"


def format_rust(text: str) -> str:
    """The generated text as `cargo fmt` would leave it, so `--check` compares like for like."""
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
