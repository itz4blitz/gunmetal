#!/usr/bin/env bash
# Builds the demo client and deploys it to the AtomicStudio Unraid host,
# which serves it to the tailnet at
# https://atomicstudio.taild1bbf.ts.net:8788/ (tailscale serve fronts the
# host's own 8788, where the gunmetal-listen container answers).
#
# The library document and the media stay on the host: the script copies
# the built client only, never library.json or media/.
#
# Root SSH to the host is assumed, which is Unraid's own account. An
# alternative host can be passed as the first argument.
set -euo pipefail
cd "$(dirname "$0")/.."

host="${1:-root@192.168.1.120}"
ui="/mnt/user/premierstudio/gunmetal-listen/ui"
url="https://atomicstudio.taild1bbf.ts.net:8788"

echo "==> build the demo client"
(cd clients && pnpm build:demo)
dist="target/clients/demo"

echo "==> deploy to ${host}:${ui}"
rsync -az --delete --max-size=50m "${dist}/assets/" "${host}:${ui}/assets/"
rsync -az --exclude assets --exclude media --exclude library.json \
  "${dist}/" "${host}:${ui}/"

echo "==> verify"
js="$(grep -oE 'assets/index-[A-Za-z0-9_-]+\.js' "${dist}/index.html" | head -1)"
curl -sk --max-time 15 -o /dev/null -w "index:   %{http_code}\n" "${url}/"
curl -sk --max-time 15 -o /dev/null -w "script:  %{http_code}  ${js}\n" "${url}/${js}"
curl -s  --max-time 15 -o /dev/null -w "library: %{http_code}\n" "${url}/library.json"
