import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const dockerfile = await readFile(new URL('../Dockerfile', import.meta.url), 'utf8');

test('container build identity works without repository metadata', () => {
  assert.match(dockerfile, /^ARG BUILD_SHA=dev$/m);
  assert.match(dockerfile, /^ENV BUILD_SHA=\$\{BUILD_SHA\}$/m);
  assert.doesNotMatch(dockerfile, /^(?:COPY|RUN).*\.git/m);
  assert.doesNotMatch(dockerfile, /git\s+rev-parse/);

  const argument = dockerfile.indexOf('ARG BUILD_SHA=dev');
  const consumed = dockerfile.indexOf('ENV BUILD_SHA=${BUILD_SHA}');
  const compilation = dockerfile.indexOf('RUN cargo build --release');
  assert.ok(argument < consumed && consumed < compilation);
});

test('container remains multi-stage and runs unprivileged on the service port', () => {
  assert.ok((dockerfile.match(/^FROM /gm) ?? []).length >= 3);
  assert.match(dockerfile, /^USER pairplay$/m);
  assert.match(dockerfile, /^ENV PORT=8080\b/m);
  assert.match(dockerfile, /^EXPOSE 8080$/m);
  assert.match(dockerfile, /^CMD \["pairplay-motion"\]$/m);
});
