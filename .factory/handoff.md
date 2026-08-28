# PairPlay Motion — repair handoff

Date: 2026-08-28

Work order: `pairplay-motion-repair-2`

Base candidate: `e138ae4e1002580b65dfd105248c10e92d2c05cd`

Artifact: existing Rust/Axum + Svelte PWA, deployed as one container

## Repair

The candidate declared `ARG BUILD_SHA` without a default and then ran
`test -n "$BUILD_SHA"`. A clean Docker build without the argument therefore
stopped before Rust compilation. The failure is reproducible directly from the
candidate's build step: an empty `BUILD_SHA` makes the guard exit 1.

The server stage now declares `ARG BUILD_SHA=dev`, consumes it through
`ENV BUILD_SHA=${BUILD_SHA}` before `cargo build --release`, and never reads
`.git`. Rust's `option_env!("BUILD_SHA")` compiles the value into the server
binary; the runtime image does not need repository metadata. Factory/ACR builds
continue to supply the full 40-character source commit. A build with no argument
has the explicit local identity `dev` rather than failing.

Focused regression coverage in `tests/container-contract.test.mjs` checks the
default, argument consumption order, absence of `.git`/`git rev-parse`, three
stages, non-root user, and port contract. The Rust health-route test now parses
the response and checks that `/health.build` equals the compiled identity.

Browser coverage was tightened to assert keyboard-generated motion frames,
visible skip-link focus, reduced-motion timing, same-origin-only first load,
versioned service-worker activation/update behavior, and offline reload.

## Verification evidence

- `npm ci`: completed from the lockfile; 158 packages audited, 0
  vulnerabilities.
- `npm test`: Svelte check 0 errors/0 warnings; 5 Vitest tests, 2 container
  contract tests, and 7 Rust tests passed.
- `npm run build`: passed; initial JS 66,493 B, lazy QR JS 25,836 B, CSS
  11,255 B, no fonts, mobile hero 33,204 B.
- `npm run test:e2e`: 12/12 passed with Playwright 1.58.2 across desktop
  Chromium and iPhone 13/390 px Chromium. Coverage includes host plus two real
  controller pages, calibration/touch fallback, keyboard arrows and Space,
  room capacity/missing-room recovery, mobile overflow, Axe serious/critical,
  reduced motion, privacy/legal routes, no third-party first-load requests,
  service-worker update policy, and usable offline reload.
- Full supplied identity build:
  `BUILD_SHA=e138ae4e1002580b65dfd105248c10e92d2c05cd cargo build --release` passed.
  Started as `env -i PORT=8090 target/release/pairplay-motion`, proving that no
  runtime variable except `PORT` is required. `/health` returned the full
  supplied SHA and `status: ok`.
- `/opt/fleet/lib/verify-url.sh http://127.0.0.1:8090`: HTTP 200 in 615 ms,
  no console/page errors, correct title and `lang`, one H1, main landmark, zero
  missing image alternatives, and zero unnamed buttons.
- Lighthouse 12.8.2 mobile: **98 performance / 100 accessibility / 100 best
  practices / 100 SEO**; FCP 1.1 s, LCP 2.0 s, TBT 120 ms, CLS 0.
- Load smoke: 200 `/health` requests at 20-way concurrency returned 200/200.
- Response checks confirm immutable caching for hashed assets plus CSP,
  `nosniff`, strict-origin referrer policy, and restricted sensor/payment
  permissions. The app's first browser load contacts only its own origin.
- `cargo fmt -- --check` and `git diff --check`: passed.

The clean container build/deploy uses the work-order configuration and the
factory builder's source-tarball path:

```sh
/opt/fleet/lib/deploy-container.sh pairplay-motion /work/repo Dockerfile 8080
```

That invokes ACR with the full current commit in `BUILD_SHA`, `GIT_SHA`, and
`SOURCE_COMMIT`, then configures the Container App with only `PORT=8080`.
Acceptance requires the live `/health.build` to equal the full 40-character
commit at deployed `main`; this was checked after the clean deployment along
with the standard live URL verifier.

For a local Docker engine, the original builder command remains:

```sh
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t pairplay-motion .
```

## Known physical-device gap

Actual accelerometer permission/calibration still needs a final smoke on
iPhone/Safari and Android/Chrome over HTTPS because headless Chromium cannot
emit phone sensor hardware readings. Permission denial, missing readings,
touch fallback, keyboard fallback, desktop, and mobile layouts are automated.
