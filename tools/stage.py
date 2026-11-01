#!/usr/bin/env python3
"""Apply the Create bundle to a Horizon checkout by hand, as horizon-mixins
stages it, so `cargo check -p horizon-client` can type-check the mod.

Only for checking: the launcher (horizon-mixins) is what builds the mod for
players. Usage: stage.py <bundle dir> <horizon worktree>
"""
import json
import re
import shutil
import sys
import tomllib
from pathlib import Path

bundle = Path(sys.argv[1]).resolve()
ws = Path(sys.argv[2]).resolve()
manifest = tomllib.loads((bundle / "horizon-mixin.toml").read_text())
mod_id = manifest["mod"]["id"]
hooks = {h["name"]: h for h in json.loads((ws / "native-hooks.json").read_text())["hooks"]}

# Crates: copied to native-crates/<mod>/<crate>, workspace member, dependency.
for c in manifest.get("crate", []):
    dest = ws / "native-crates" / mod_id / c["id"]
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(bundle / c["path"], dest, ignore=shutil.ignore_patterns("target"))
    pkg = tomllib.loads((dest / "Cargo.toml").read_text())["package"]["name"]
    root = ws / "Cargo.toml"
    text = root.read_text()
    rel = f"native-crates/{mod_id}/{c['id']}"
    if rel not in text:
        text = re.sub(r"members\s*=\s*\[", lambda m: m.group(0) + f'\n    "{rel}",', text, count=1)
        root.write_text(text)
    client = ws / "crates/horizon-client/Cargo.toml"
    text = client.read_text()
    if f"{pkg} =" not in text:
        text = text.replace("[dependencies]\n", f'[dependencies]\n{pkg} = {{ path = "../../{rel}" }}\n', 1)
        client.write_text(text)

# Modules: inline modules at the end of their file.
for m in manifest.get("module", []):
    target = ws / m["file"]
    text = target.read_text()
    body = (bundle / m["source"]).read_text()
    text += f"\n#[allow(dead_code, unused_imports)]\npub(crate) mod {m['name']} {{\n{body}\n}}\n"
    target.write_text(text)

# HEAD injections: the helper is a local fn called first in the target's body.
for inj in manifest.get("injection", []):
    target = hooks[inj["hook"]]["target"]
    fn = target.rsplit("::", 1)[1]
    impl = target.rsplit("::", 2)[1]  # Game<'_> or GameLevel<'_>
    impl_name = impl.split("<")[0]
    helper = (bundle / inj["helper"]).read_text()
    helper_fn = re.search(r"fn (\w+)\s*[(<]", helper).group(1)
    args = ", ".join(b["value"] for b in inj["bindings"])
    src = ws / "crates/horizon-client/src/game/native_hooks.rs"
    text = src.read_text()
    # The function inside the right impl block.
    impl_starts = [m.start() for m in re.finditer(rf"^impl {re.escape(impl_name)}<'_> \{{", text, re.M)]
    found = None
    for start in impl_starts:
        m = re.compile(rf"fn {fn}(<[^>]*>)?\(").search(text, start)
        if m and (found is None or m.start() < found.start()):
            # Must be before the next top-level impl.
            nxt = re.compile(r"^impl ", re.M).search(text, start + 1)
            if nxt is None or m.start() < nxt.start():
                found = m
    if found is None:
        sys.exit(f"{fn} not found in impl {impl_name}")
    # The body's opening brace: the first `{` at depth 0 after the signature.
    i = found.end()
    depth = 1
    while depth:
        depth += {"(": 1, ")": -1}.get(text[i], 0)
        i += 1
    brace = text.index("{", i)
    inject = f"\n        {{\n{helper}\n        {helper_fn}({args});\n        }}\n"
    text = text[: brace + 1] + inject + text[brace + 1 :]
    src.write_text(text)
    print("hooked", fn)
