# PairPlay Motion — build handoff

Date: 2026-08-28  
Work order: `pairplay-motion-build-1`  
Artifact: one-container web app with Axum backend and Svelte PWA

## What shipped

- A complete host-to-phone room flow with cryptographically random six-character
  room codes, an original QR invite, manual join, 2–4 player roster, disconnect
  handling, and reconnect action.
- Explicit motion permission and two-second neutral calibration that handles
  iOS permission methods. Browsers that deny access or emit no sensor readings
  get a fully playable touch and keyboard controller.
- Three original 40-second reaction games with live shared scores:
  - **Dead Still** — alternate between moving and holding still (free).
  - **News Desk** — race to match changing tilt directions (full edition).
  - **Ink Runner** — shake only while the presses say GO (full edition).
- One-time US $8 Sociobot license unlock: hosted checkout link, URL token
  capture/cleanup, local storage, daily verification cache, background
  reconciliation, invalid-license notice, and paste-to-restore flow. No product
  ID or payment provider is embedded.
- Privacy-local architecture: live names and motion samples exist only in the
  room process memory. SQLite stores only one aggregate page-view integer per
  UTC day. There are no accounts, tracking scripts, or third-party fonts.
- Installable offline shell with a versioned service worker, automatic hashed
  asset discovery, offline status, responsive icon/manifest, Brotli/gzip, and
  immutable asset caching.
- Real `/privacy` and `/terms` routes with 200 responses, safety copy, direct-
  route SPA fallback, structured JSON logs, security headers, a build-aware
  `/health`, graceful shutdown, and a non-root multi-stage container.
- A product-specific monochrome broadsheet system and original generated hero
  collage. Source, exact generation prompt, and provenance are under
  `assets/src/`; mobile/desktop WebPs are 33 KB and 104 KB.

## Run and deploy

```sh
npm ci
npm run build             # emits dist/ with index.html at its root
cargo run                 # serves dist, API, and WebSockets on PORT=8080
```

Container build command:

```sh
docker build --build-arg BUILD_SHA="$(git rev-parse --short HEAD)" -t pairplay-motion .
```

Persist `/app/data` for the anonymous daily count. The service otherwise has no
persistent room or sensor state. See `README.md` for all environment variables.

## Verification completed

- `npm test`: passed; Svelte/TypeScript check reports 0 errors and 0 warnings,
  5 Vitest game-rule tests passed, and 5 Rust route/unit tests passed.
- `npm run test:e2e`: 8/8 passed against the built Axum app across desktop
  Chromium and a 390px mobile profile. This includes a real host plus two
  independent controller pages, calibration fallback, round start, direct
  legal routes, Axe, horizontal overflow, and full offline reload.
- `npm run build`: passed. Initial JavaScript is 66.55 KB uncompressed
  (25.84 KB QR chunk loads only after hosting); CSS is 11.26 KB; no fonts ship.
- `cargo build --release`: passed.
- `npm audit`: 0 vulnerabilities.
- `/opt/fleet/lib/verify-url.sh`: 200 response, 561 ms local load, no console
  errors, title and `lang` present, exactly one `h1`, main landmark present,
  zero missing image alts, and zero unlabeled buttons.
- Lighthouse 12 mobile: **99 performance / 100 accessibility / 100 best
  practices / 100 SEO**; FCP 1.1 s, LCP 2.0 s, TBT 20 ms, CLS 0. The CLI emitted
  a browser-tab shutdown warning after writing a complete valid report; scores
  and audits were present.
- Load smoke: 200 `/health` requests at 20-way concurrency completed 200/200;
  this exceeds the documented 100 rps smoke volume.
- Manual screenshot review at 1366×900 and 390×844 confirmed the broadsheet
  composition, mobile stacking, legibility, and no horizontal overflow.
- Direct `/privacy` and `/terms` requests return 200. Hashed assets return
  `content-encoding: br` when accepted and
  `cache-control: public, max-age=31536000, immutable`.

## Known gaps and release notes

- Real accelerometer/gyroscope behavior must receive a final smoke test on an
  HTTPS-served iPhone and Android phone; the headless worker cannot emit real
  hardware events. Permission denial, missing readings, touch, and keyboard
  paths are automated.
- The factory still needs to register the `pairplay-motion` paid product and its
  US $8 price/return URL. The client already uses the production Sociobot
  contract and `VITE_BILLING_BASE` can target the pilot API for staging.
- V1 uses an ephemeral same-origin WebSocket relay rather than negotiating a
  peer-to-peer WebRTC data channel. This keeps first-time setup deterministic
  across mobile browsers; sensor streams remain unpersisted and room-scoped.
- The container definition was inspected and both of its build stages were
  reproduced locally (`npm run build`, `cargo build --release`), but this worker
  has no Docker/Podman executable, so the assembled OCI image was not launched.
- Session-completion analytics are intentionally absent. The brief's 80%
  five-minute retention target should be assessed with consented user testing,
  not identity-bearing telemetry.

## Next release checks

1. Register the test and live billing product, verify checkout return and refund
   revocation, then leave the default base on the live API.
2. Run the 90-second first-use script on one current iPhone/Safari and one
   mid-range Android/Chrome over HTTPS.
3. Build and smoke the OCI image in CI, mount a writable `/app/data` volume, and
   confirm the injected build SHA at `/health`.
