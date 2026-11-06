#!/usr/bin/env bash
# Exercise production initialization without GTK or a desktop session. No user files change.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
binary="${MERIDIAN_ENGINE_TEST_BINARY:-$ROOT/target/debug/meridian-shell}"
[ -x "$binary" ] || { echo 'Build meridian-shell before running the engine startup tests.' >&2; exit 1; }
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/config/meridian"
XDG_CONFIG_HOME="$fixture/config" "$binary" --engine-status > "$fixture/default.json"
python3 - "$fixture" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
state = json.loads((root / 'default.json').read_text())
assert state['font_config_loaded'], state['font_status']
assert not state['restart_required']
assert 'FreeType' in state['backends'] and 'Cairo' in state['backends']
config = dict(state['config'], font_profile='crisp', acceleration='software', performance='efficient')
(root / 'config/meridian/engines.json').write_text(json.dumps({'schema_version': 1, 'data': config}))
PY
# Simulate a child process inheriting the prior session's scoped profile.
previous_profile=$(find "$fixture/config/meridian/rendering" -name 'fonts-0-*.conf' -print -quit)
XDG_CONFIG_HOME="$fixture/config" FONTCONFIG_FILE="$previous_profile" MERIDIAN_BASE_FONTCONFIG_FILE=/etc/fonts/fonts.conf \
    "$binary" --engine-status > "$fixture/crisp.json"
python3 - "$fixture" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
state = json.loads((root / 'crisp.json').read_text())
assert state['font_config_loaded'], state['font_status']
assert state['active']['font_profile'] == 'crisp'
assert state['active']['acceleration'] == 'software'
assert not state['restart_required']
assert not state['animations_enabled'] and not state['transparency_enabled']
assert state['icon_raster_size'] == 96
xml = next((root / 'config/meridian/rendering').glob('fonts-1-*.conf')).read_text()
assert 'hintfull' in xml and '/etc/fonts/fonts.conf' in xml
assert 'fonts-0.conf' not in xml and 'fonts-1.conf' not in xml
PY
if command -v fc-match >/dev/null; then
    crisp_profile=$(find "$fixture/config/meridian/rendering" -name 'fonts-1-*.conf' -print -quit)
    result=$(FONTCONFIG_FILE="$crisp_profile" fc-match -f '%{hintstyle}|%{rgba}|%{antialias}')
    [ "$result" = '3|5|True' ] || { echo "Unexpected Fontconfig result: $result" >&2; exit 1; }
fi
printf 'Native engine startup, persisted profiles, child inheritance, and Fontconfig matching passed.\n'
