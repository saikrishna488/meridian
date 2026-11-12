#!/usr/bin/env python3
"""Generate native policy/type metadata and the parity checklist from Rust.

This deliberately supports only the protocol's closed structs/enums. New Rust
syntax fails generation instead of silently weakening native validation.
"""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def block(source, declaration):
    body = source.split(declaration, 1)[1]
    depth = 1
    for i, char in enumerate(body):
        depth += (char == "{") - (char == "}")
        if depth == 0:
            return body[:i]
    raise ValueError(f"Unclosed {declaration}")


def wire_name(name, style):
    words = re.sub(r"(?<!^)(?=[A-Z])", "-", name).lower()
    return {"kebab-case": words, "snake_case": words.replace("-", "_"),
            "lowercase": name.lower(), "": name}[style]


def inventory(source):
    source = re.sub(r"//[^\n]*", "", source)
    requests = block(source, "pub enum Request {")
    methods = re.findall(r'#\[serde\(rename = "([^"]+)"\)\]\s*(\w+)(?:\(([^\n)]+)\))?,', requests)
    variants = re.findall(r"^\s*(\w+)(?:\([^\n]*\))?,", requests, re.M)
    if len(methods) != len(variants):
        raise ValueError("Unrecognized Request syntax")
    policy = block(source, "impl Request {")
    capabilities = {}
    arm_pattern = r"((?:Request::\w+(?:\(_\))?\s*(?:\|\s*)?)+)=>\s*(?:\{\s*)?(None|Some\(Capability::\w+\))"
    for arm, capability in re.findall(arm_pattern, policy):
        for variant in re.findall(r"Request::(\w+)", arm):
            if variant in capabilities:
                raise ValueError(f"Duplicate capability for {variant}")
            capabilities[variant] = capability.removeprefix("Some(Capability::").removesuffix(")")
    if set(variants) != set(capabilities):
        raise ValueError("Request/capability mismatch")
    surfaces = {}
    policy = block(source, "impl SurfaceKind {")
    for arm, grants in re.findall(r"((?:SurfaceKind::\w+\s*(?:\|\s*)?)+)=>\s*(?:\{\s*)?&\[([^\]]*)\]", policy):
        for surface in re.findall(r"SurfaceKind::(\w+)", arm):
            surfaces[wire_name(surface, "kebab-case")] = re.findall(r"Capability::(\w+)", grants)
    expected = re.findall(r"\b(\w+)\s*,", block(source, "pub enum SurfaceKind {"))
    if len(surfaces) != len(expected):
        raise ValueError("Surface/capability mismatch")
    definitions = {}
    pattern = r"((?:#\[[^\]]*\]\s*)*)pub (struct|enum) (\w+)\s*\{"
    for match in re.finditer(pattern, source):
        attributes, kind, name = match.groups()
        if name in {"Request", "Event"}:
            continue
        body = block(source[match.start():], f"pub {kind} {name} {{")
        if kind == "enum":
            style = re.search(r'rename_all = "([^"]+)"', attributes)
            values = []
            for item in body.split(","):
                item = re.sub(r"#\[[^\]]*\]", "", item).strip()
                if not item:
                    continue
                if not re.fullmatch(r"\w+", item):
                    raise ValueError(f"Unsupported enum {name}")
                values.append(wire_name(item, style[1] if style else ""))
            definitions[name] = {"kind": "enum", "values": values}
        else:
            fields = []
            cursor = 0
            while field := re.search(r"pub\s+(\w+)\s*:\s*", body[cursor:]):
                prefix = body[cursor:cursor + field.start()]
                start = cursor + field.end()
                end, depth = start, 0
                while end < len(body):
                    char = body[end]
                    if char == "," and depth == 0:
                        break
                    depth += (char in "<(") - (char in ">)")
                    end += 1
                field_type = re.sub(r"\s+", "", body[start:end])
                fields.append({"name": field[1], "type": field_type,
                               "optional": field_type.startswith("Option<") or "default" in attributes or "default" in prefix})
                cursor = end + 1
            definitions[name] = {"kind": "object", "fields": fields,
                                 "closed": "deny_unknown_fields" in attributes}
    metadata = {"methods": {method: {"type": parameters or "unit", "capability": capabilities[variant]}
                            for method, variant, parameters in sorted(methods)},
                "surfaces": surfaces, "types": definitions}
    # Fail closed if a referenced type has no schema.
    def check_type(value):
        if value in {"unit", "String", "bool", "f64", "u8", "u16", "u32", "i32"}:
            return
        for wrapper in ("Option", "Vec"):
            if value.startswith(wrapper + "<") and value.endswith(">"):
                check_type(value[len(wrapper) + 1:-1])
                return
        if value == "std::collections::HashMap<String,DesktopPosition>":
            check_type("DesktopPosition")
            return
        if value not in definitions:
            raise ValueError(f"Missing native schema: {value}")
        if value in visited:
            return
        visited.add(value)
        for field in definitions[value].get("fields", []):
            check_type(field["type"])
    visited = set()
    for method in metadata["methods"].values():
        check_type(method["type"])
    return methods, capabilities, metadata


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    methods, capabilities, metadata = inventory((ROOT / "protocol/src/lib.rs").read_text())
    rows = ["# Native service parity checklist", "",
            "Generated by `python3 tools/migration-inventory.py`. The Rust protocol",
            "defines the compatibility target. Registration and schema validation alone",
            "do not establish service parity; implementation and integration must pass.", "",
            f"Total requests: {len(methods)}.", "",
            "| Port verified | Method | Parameters | Required capability |",
            "| --- | --- | --- | --- |"]
    registry = "// Generated by tools/migration-inventory.py; do not edit.\n"
    for method, variant, parameters in sorted(methods):
        capability = capabilities[variant]
        rows.append(f"| [ ] | `{method}` | {parameters or '—'} | {capability} |")
        registry += f'    {{"{method}", "{capability}"}},\n'
    encoded = json.dumps(metadata, separators=(",", ":"), sort_keys=True)
    outputs = {ROOT / "docs/MIGRATION-PARITY.md": "\n".join(rows) + "\n",
               ROOT / "native/src/protocol/request_registry.inc": registry,
               ROOT / "native/src/protocol/schema.inc": '// Generated; do not edit.\nR"SCHEMA(' + encoded + ')SCHEMA"\n'}
    for path, content in outputs.items():
        if args.check:
            if not path.exists() or path.read_text() != content:
                raise SystemExit(f"Stale metadata: {path}; run tools/migration-inventory.py")
        else:
            path.write_text(content)
    print(f"Inventoried {len(methods)} requests, {len(metadata['surfaces'])} surfaces and native schemas")


if __name__ == "__main__":
    main()
