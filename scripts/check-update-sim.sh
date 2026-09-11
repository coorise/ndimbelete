#!/usr/bin/env bash
# Simulate installed v0.1.0 client checking GitHub for a newer production release.
set -euo pipefail
CURRENT="${1:-0.1.0}"
CHANNEL="${2:-main}"
API="https://api.github.com/repos/coorise/ndimbelete/releases?per_page=20"
json=$(curl -fsSL -H "Accept: application/vnd.github+json" -H "User-Agent: ndimbelente-updater-test" "$API")
if [[ "$CHANNEL" == "dev" ]]; then
  latest=$(echo "$json" | python -c "import json,sys; rs=json.load(sys.stdin); r=next((x for x in rs if x.get('prerelease') or x['tag_name'].lower().startswith('dev-')), None); print(r['tag_name'] if r else '')")
else
  latest=$(echo "$json" | python -c "import json,sys; rs=json.load(sys.stdin); r=next((x for x in rs if not x.get('prerelease') and not x['tag_name'].lower().startswith('dev-') and not x.get('draft')), None); print(r['tag_name'] if r else '')")
fi
norm() { echo "$1" | sed -E 's/^[vV]//;s/^dev-//I'; }
cv=$(norm "$CURRENT")
lv=$(norm "$latest")
echo "channel=$CHANNEL current=$cv latest_tag=$latest latest=$lv"
python - <<PY
from packaging.version import Version
cv, lv = "$cv", "$lv"
ok = Version(lv) > Version(cv) if lv and cv else False
print("update_available=" + str(ok).lower())
raise SystemExit(0 if ok else 1)
PY
