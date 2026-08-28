# PairPlay Motion — independent verification

**Result: FAIL**

Date: 2026-08-28  
Verifier work order: `pairplay-motion-verify-1`  
Candidate: `6c0b0f250271f166365e004decf63c1647788bb4`  
Live URL: `https://pairplay-motion.sociobot.in`

## Scope and environment

Verification started from a clean `main` checkout at the candidate SHA. Node
dependencies were installed with `npm ci`; Playwright 1.58.2 and its installed
Chromium were used. The local app was served from the fresh `dist/` through the
Axum binary. Docker and Podman are not installed in this worker, so the OCI
image itself could not be assembled; both image build stages were reproduced
with the repository commands below.

## Local quality gates — PASS

| Check | Evidence |
| --- | --- |
| Install/audit | `npm ci` completed; `npm audit` reported 0 vulnerabilities. |
| Type/Svelte check | `svelte-check --tsconfig ./tsconfig.json`: 0 errors, 0 warnings. |
| Unit/integration | `npm test`: 5 Vitest game-rule tests and 5 Rust route/unit tests passed. |
| Production builds | `npm run build` passed; `cargo build --release` passed. |
| Browser E2E | `npm run test:e2e`: **8 passed** in 49.7 s (desktop Chromium and iPhone-13/390px Chromium). |
| Bundle budget | Fresh `dist/`: initial application JS 66,551 B plus an on-demand QR chunk 25,836 B; CSS 11,255 B; no fonts; mobile hero WebP 33,204 B. All are within the stated budgets. |
| PWA/offline | The E2E suite registered the service worker, reloaded offline, showed `OFFLINE`, and retained the usable host action. `sw.js` is `no-cache`, calls `skipWaiting()`/`clients.claim()`, and cache names are versioned (`pairplay-shell-v2`). |
| Concurrency/persistence | 100 `/health` requests with 20-way concurrency completed successfully. Rust test `page_view_stores_only_daily_count` passed; source review confirms the SQLite schema holds daily aggregate counts only and rooms/motion are in memory. |

## Product and browser exercise — PASS except defects below

- Normal path: created a host room, joined four independent controllers, used
  the touch-control calibration fallback, and started a round. The shipped
  E2E suite separately exercised two controllers through calibration and a
  playable Dead Still round.
- Boundary/invalid path: short codes produce “Enter the six-character room
  code.”; empty required names receive native browser validation; six-character
  joining, four-player capacity, and room creation work.
- Keyboard and focus: initial Tab reaches the skip link. Its computed focus
  treatment is `rgb(182, 59, 34) solid 3px` with a 3px offset. The controller
  supplies keyboard arrow/Space mappings in touch mode.
- Responsive/motion: inspected fresh 1366×900 and 390px screenshots; no
  horizontal overflow at 390px. With reduced motion emulated, the media query
  matched and button transition duration was `0.00001s`.
- Accessibility: fresh Axe scan found **0 serious or critical violations**;
  `<title>`, `lang`, one H1, main/skip-link, labelled controls, and image
  alternatives were present. No browser console errors or `pageerror` events
  were recorded on normal use.
- Privacy/network: a normal browser session made requests only to its own
  origin (the aggregate page-view POST plus app assets). Source and CSP review
  found no analytics or third-party runtime code/fonts; the only permitted
  external origin is Sociobot billing when a user initiates license work.
- Response policies: live assets use Brotli on `Accept-Encoding: br,gzip`,
  immutable cache control for hashed assets, `no-cache` for `sw.js`, and CSP,
  `nosniff`, referrer, and sensor permissions policies. `/privacy`, `/terms`,
  `/manifest.webmanifest`, and `/sw.js` returned 200.

## Deployment comparison

The live frontend is the candidate exactly. SHA-256 digests match the fresh
build for `index.html`, `sw.js`, `assets/index-BA5V5d2s.js`,
`assets/index-DP9Fu-lo.css`, and `assets/hero-broadsheet-720.webp`.

The backend identity cannot be confirmed: live `GET /health` returns
`{"build":"container","status":"ok"}` rather than the tested commit SHA.
The Dockerfile defaults `BUILD_SHA` to `container`, so deployment must inject
the candidate SHA and then be redeployed before this can pass.

## Defects

### P1 — release identity is not the candidate SHA

Live `/health` reports `build: "container"`, not
`6c0b0f250271f166365e004decf63c1647788bb4` (or its short SHA). This violates
the backend build-identity requirement and leaves the deployed backend
unverifiable even though its frontend bytes match the candidate.

**Reproduce:** `curl -sS https://pairplay-motion.sociobot.in/health`.

### P2 — fifth player gets a misleading recovery message

After four controllers have joined a room, a fifth controller receives a
rejected WebSocket handshake. Browsers expose this as an abnormal close, not
the `4003` close code expected by the UI; it displays “Connection lost. Rejoin
when Wi-Fi returns.” rather than explaining that the room is full. A missing
room has the same handshake-status limitation.

**Reproduce:** create a room, join four named controllers, then join a fifth.

### P2 — public room and WebSocket endpoints have no rate limiting

Source review confirms `/api/rooms` and `/ws` have body limits/capacity but no
per-client or global rate limiting. The room route can allocate up to 2,000
in-memory rooms and WebSocket paths relay at unbounded application rate. This
does not block the normal flow but fails the supplied backend security
baseline and leaves a public service readily exhaustible.

## Required release disposition

Do not mark this candidate/deployment verified until the live build is rebuilt
with `BUILD_SHA=6c0b0f250271f166365e004decf63c1647788bb4` and `/health` proves
it, then retest the capacity message and apply a rate-limit design to the
public endpoints. Real HTTPS physical-device sensor testing remains a
recommended release smoke test; browser permission denial and touch fallback
were covered automatically, but headless Chromium cannot emulate actual phone
accelerometer hardware.
