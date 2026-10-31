#!/usr/bin/env bash
# Safe source-installer smoke tests: never build, install packages, or elevate.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

bash -n "$ROOT/install.sh"
bash "$ROOT/install.sh" --help > "$fixture/help"
grep -q -- '--check' "$fixture/help"
grep -q -- '--uninstall' "$fixture/help"
if bash "$ROOT/install.sh" --not-an-option > "$fixture/invalid" 2>&1; then
  echo "Unknown installer option was accepted" >&2
  exit 1
fi

# A read-only preflight may fail in CI without native build prerequisites,
# but it must never invoke sudo or mutate the checkout.
mkdir -p "$fixture/bin"
cat > "$fixture/bin/sudo" <<'EOF'
#!/usr/bin/env bash
echo "unexpected sudo invocation" >&2
exit 94
EOF
chmod +x "$fixture/bin/sudo"
printf 'ID=fedora\n' > "$fixture/os-release"
set +e
MERIDIAN_OS_RELEASE="$fixture/os-release" PATH="$fixture/bin:$PATH" bash "$ROOT/install.sh" --check > "$fixture/check" 2>&1
status=$?
set -e
if [ "$status" = 94 ]; then
  echo "Preflight unexpectedly invoked sudo" >&2
  exit 1
fi
if (( EUID == 0 )); then
  # GitHub Actions' Debian container runs as root. The source installer must
  # refuse root rather than run its normal-user preflight in that environment.
  grep -q 'Run as your regular user' "$fixture/check"
else
  grep -q 'Checking development and runtime requirements' "$fixture/check"
  grep -q 'Meridian source installer' "$fixture/check"
fi
echo "Source installer syntax, help, invalid options and read-only preflight passed."
