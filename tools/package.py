#!/usr/bin/env python3
"""Pack the mod as a source-only `.hnmod` (a zip) for the Horizon launcher's Mods page.

Holds `horizon-native.json` and `mixins/timber/` (`.rs` and `.toml` only), with fixed dates,
so the same sources always give the same bytes. Adapted from horizon-native-mod-template.
"""
import hashlib
import json
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

compat = json.loads((ROOT / 'compatibility.json').read_text())
bundle = ROOT / 'mixins' / 'timber'
mod = tomllib.loads((bundle / 'horizon-mixin.toml').read_text())['mod']

manifest = dict(schema=1, id=mod['id'], version=mod['version'], game=compat['game'],
                horizon=compat['horizon_revision'], mixins=compat['horizon_mixins_revision'],
                hooks=compat['hooks'])
files = {'horizon-native.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
for path in sorted(bundle.iterdir()):
    if path.suffix in ('.rs', '.toml'):
        files[path.relative_to(ROOT).as_posix()] = path.read_bytes()

out = ROOT / 'target' / f"{mod['id']}-{mod['version']}.hnmod"
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as archive:
    for name, content in sorted(files.items()):
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, content)
print(out, hashlib.sha256(out.read_bytes()).hexdigest())
