# PairPlay Motion — repair handoff

Date: 2026-08-28
Work order: `pairplay-motion-repair-1`
Artifact: the existing Rust/Axum + Svelte PWA one-container web app

## Release disposition — PASS

This repair addresses every blocker in the independent report at
`.factory/verification.md` for candidate
`6c0b0f250271f166365e004decf63c1647788bb4`. The deployed repair source is
commit `40d42ba0a21ae7debf6ba5cc6085ad1abffc2e56`; it is pushed to `main` and
live at `https://pairplay-motion.sociobot.in`.

Live proof, after the container deployment:

```json
{"build":"40d42ba0a21ae7debf6ba5cc6085ad1abffc2e56","status":"ok"}
```

The published image is
`sociobotregistry.azurecr.io/sf-pairplay-motion:40d42ba0a21a` at digest
`sha256:68eda5139b30f9ee476247b16a71d36ee05af2367ad7dc3d80f8696169d5bfa0`.

## What changed

- Container releases now require a non-empty, non-`container` `BUILD_SHA` at
  build time. `/health` reports that compiled identifier, preventing an
  unverifiable placeholder deployment.
- WebSocket admission is now atomic with player insertion. A full or missing
  room upgrades first and sends an application close frame (`4003` or `4004`),
  which browsers expose to the controller UI. The fifth player sees “This room
  already has four players.”; a missing room receives code recovery guidance.
- Public `/api/rooms` and `/ws` requests have source-address and service-wide
  token buckets. Room creation allows a burst of 12 then returns `429` with
  `Retry-After: 5`; WebSocket upgrades allow a burst of 30. Each live socket
  additionally limits relay input to a 30-message burst and 15 messages per
  second. Existing message/body/capacity limits remain in place.
- Added regression coverage for both specific controller recovery paths, room
  rate-limit response semantics, bounded relay bursts, and the no-placeholder
  build identity invariant.

## How verified

- Clean install: `npm ci` completed; `npm audit` reported 0 vulnerabilities.
- `npm test`: Svelte/TypeScript check has 0 errors and 0 warnings; 5 Vitest
  game-rule tests and 7 Rust route/unit tests passed.
- `npm run build` passed. Initial app JS is 66.49 KB uncompressed, the
  on-demand QR chunk is 25.84 KB, and CSS is 11.26 KB.
- `cargo build --release` passed. A local production build compiled with the
  exact repair SHA returned it from `/health`; 13 immediate room requests
  returned twelve `200`s then a `429`.
- `npm run test:e2e`: **12/12 passed** using Playwright 1.58.2 on desktop
  Chromium and iPhone 13 Chromium (390px). It covers the original room flow,
  touch calibration, keyboard controls, desktop/mobile layout, Axe serious and
  critical violations, offline shell, legal routes, fifth-player recovery, and
  missing-room recovery.
- `verify-url.sh https://pairplay-motion.sociobot.in`: 200, 603 ms load, no
  console/page errors, title and `lang`, one H1, main landmark, no missing
  image alts, and no unlabelled buttons. Live GET asset responses have Brotli,
  immutable cache control, CSP, `nosniff`, referrer policy, and sensor/payment
  permissions policy.
- Lighthouse mobile local production run: **99 performance / 100
  accessibility / 100 best practices / 100 SEO**; FCP 1.1 s, LCP 2.0 s, CLS 0.

## Run or redeploy

```sh
npm ci
npm test
npm run build
cargo build --release
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t pairplay-motion .
```

The factory deployment used the required build argument and the existing
container deployment class. Persist `/app/data` for the anonymous daily counter.

## Remaining release smoke

Physical iPhone/Safari and Android/Chrome sensor permission/calibration checks
remain advisable on HTTPS because headless Chromium cannot generate actual
accelerometer readings. Permission denial, absent readings, touch fallback,
and keyboard fallback are automated and remain usable.
