#!/usr/bin/env bash
set -euo pipefail

image="${1:?Usage: test-script-sdk-image.sh IMAGE}"

# Check the packaged asset as production and development users. Root can bypass
# a missing directory search bit, so a root-only Agent smoke test misses this.
for user in 65532:0 1000:1000; do
  docker run --rm -i --network none --read-only --tmpfs /tmp:rw,nosuid,nodev,mode=1777 \
    --user "$user" --entrypoint sh "$image" -s <<'SH'
set -eu
test -x /app/scripts || { echo 'Packaged SDK directory is not traversable' >&2; exit 1; }
test -r /app/scripts/citadel.js
test "$(stat -c '%a' /app/scripts/citadel.js)" = 444
test ! -w /app/scripts/citadel.js
mkdir /tmp/sdk-fixture
cat > /tmp/sdk-fixture/script.ts <<'TS'
import { createCitadelClient } from 'citadel:sdk';
const citadel = createCitadelClient({baseUrl: 'https://fixture.invalid', token: 'disposable'});
if (typeof citadel.platforms.listPlatforms !== 'function') throw Error('SDK catalog missing');
try {
  await Deno.writeTextFile('/app/scripts/citadel.js', 'corrupted');
  throw Error('SDK is writable by scripts');
} catch (error) {
  if (!(error instanceof Deno.errors.NotCapable)) throw error;
}
await Deno.writeTextFile('/tmp/sdk-fixture/payload', 'workspace remains writable');
TS
printf '%s\n' '{"imports":{"citadel:sdk":"file:///app/scripts/citadel.js"}}' > /tmp/sdk-fixture/imports.json
printf '%s\n' "const action = new URL('./script.ts', import.meta.url).href; await import(action);" > /tmp/sdk-fixture/bootstrap.ts
DENO_DIR=/tmp/sdk-fixture/deno-cache deno run --no-prompt --no-config --no-lock --no-npm --deny-import \
  --import-map=/tmp/sdk-fixture/imports.json \
  --allow-read=/tmp/sdk-fixture,/app/scripts/citadel.js --allow-write=/tmp/sdk-fixture \
  /tmp/sdk-fixture/bootstrap.ts
SH
  printf 'Packaged SDK read/import and write isolation passed as %s.\n' "$user"
done
