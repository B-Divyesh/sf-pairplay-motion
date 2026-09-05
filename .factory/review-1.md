# Review: turn spare phones into motion controllers

**Verdict: FAIL**

Date: 2026-09-05

Work order: `pairplay-motion-review-1`

Live URL: https://pairplay-motion.sociobot.in

Implementation candidate: `bb3c87c69f1df1ac1d316a48f57591bd5399f75c`

Documentation commit: `ae5821e9f2e741a80103f2e04eb35e7ed7b5314e`

There are **7 findings**: 1 P0, 4 P1, 1 P2, and 1 P3. There are **14 untested public claims**. This is not a PASS.

## What a visitor sees before scrolling

- Job stated in supporting text: turn two to four spare phones into motion controllers for shared room games.
- Audience stated in the brief: friends and families with spare phones who want a quick local party-game setup.
- First action shown: **Host a game**.

The first-screen headline is “Phones up. Game on.” It does not name the job. There is no “Try it with sample data” action.

## Live browser checks

Clean Chromium desktop (1440 × 1000) and iPhone 13 browser contexts loaded without console errors, page errors, third-party requests, horizontal overflow, or Axe serious/critical violations. Screenshots are at `/work/.evidence/live-desktop.png` and `/work/.evidence/live-phone.png`.

The basic host journey is broken in production. On two separate fresh visits, pressing **Host a game** returned a code, then the host WebSocket opened and immediately closed with code `4004`, reason `Room not found`. The host showed LOST before any player could join. The observed room codes were `VHDXRB` and `GYZJD8`. This is consistent with the in-memory room being created on one backend instance and the WebSocket reaching another.

The normal two-controller, touch-fallback, keyboard, round, invalid-code, missing-room, capacity, offline, and update paths pass in the local declared Playwright suite. That does not make the broken live host path pass.

## Findings

### P0 — newly created live rooms cannot host a game

Two live host attempts each created a room, opened `wss://pairplay-motion.sociobot.in/ws`, and received `4004 Room not found` immediately. Friends and families cannot reach calibration or a playable round. Make room creation and WebSocket relay use shared/durable room state, or deploy one relay with verified sticky routing. Retest fresh live host creation and two independent controller joins after deployment.

### P1 — no one-click sample sandbox exists

Clean live desktop and phone pages contained zero “Try it with sample data” actions, zero persistent demo labels, zero Reset demo controls, and zero Start for real controls. `/demo` returns the ordinary landing page. The source has no demo route or storage namespace, and `.factory/demo.md` is absent. The required sample flow cannot be entered, inspected, reset, or shown not to change real data. Add a seeded `/demo`, persistent label, reset/start controls, separate demo storage, documentation, and demo-only tests.

### P1 — the required claims manifest and claim tests are missing

`.factory/claims.json` does not exist. No test is tagged `@claim:<id>`, and no declared claim command can run from a clean demo entry. General E2E tests are not a replacement for individually named claims in a demo sandbox.

The following 14 concrete public claims are unlisted and untested under the claims contract: no app; no account; no sensor history; two-to-four phone support; setup in ninety seconds; same-Wi-Fi joining; motion is not kept; Dead Still is free; US $8 price; one-time/no-subscription purchase; real motion plus touch/keyboard fallback; rooms and motion only in server memory; anonymous aggregate-only SQLite storage; and source/service/relay rate limits. Inventory each claim with exactly one clean-demo tagged test or remove it.

### P1 — the first screen does not use the required plain job language

The sole H1 is the header brand (`PairPlay Motion`). The first content heading is an H2, “Phones up. Game on.” The header’s “The room-play edition,” the hero caption “Yesterday’s phones, tonight’s controllers,” and “Three ways to move the news” are decorative wording rather than useful section names. The landing does not provide a plain main-content job H1, as required. Replace them with a short job H1, direct audience sentence, plain section labels, and the sample action beside the real first action.

### P1 — source rate limiting ignores the forwarded client address

`src/main.rs` keys `public_rate_limit` from `ConnectInfo<SocketAddr>` and never reads `X-Forwarded-For`, despite the backend contract requiring the first forwarded hop behind ingress. Fifteen rapid live room creations all returned 200, exceeding the configured source burst of 12; only the later global allowance produced 429. The 130-request continuation returned 41 × 200 and 89 × 429, and each 429 included `Retry-After: 5`. Parse and validate the first forwarded hop for the per-client bucket, retain the global bucket, and test the proxy-header case.

### P2 — routes lack route titles and a designed 404 page

`/`, `/demo`, `/privacy`, `/terms`, and `/missing-page` all use the title `PairPlay Motion — phones up, game on`. `/missing-page` returns 200 and renders the landing screen, not a designed 404 with a route-specific title and way back. Implement `/demo` and `/404`, update titles on each SPA route, and render the 404 state for unknown URLs.

### P3 — invalid join errors are announced twice

On live `/?join=X`, entering `Ada` and choosing Join room rendered two identical `role=alert` nodes: “Enter the six-character room code.” This repeats the P3 in `.factory/verification-2.md` and the prior handoff. Render the error once, or make one instance non-live and reference it with `aria-describedby`.

## Earlier findings and limits

| Earlier item | Current disposition |
| --- | --- |
| Initial P1 build identity | Fixed. Live `/health` returns `bb3c87c69f1df1ac1d316a48f57591bd5399f75c`. |
| Initial P2 full/missing-room recovery | Locally covered by the passing Playwright suite. Live host creation now fails earlier with the P0 above. |
| Initial P2 absent rate limiting | Partly fixed: live global limiting returns 429 plus Retry-After. The forwarded-client key is still missing (P1 above). |
| Earlier P3 duplicate alerts | Still present (P3 above). |
| Physical phone sensor smoke | Not proved in this container. The browser phone context covered layout only; the public sensor claim remains untested. |
| OCI assembly | Docker and Podman are unavailable here. The container contract test passed, but an OCI image build was not repeated. |

## Commands and other checks

From the clean checkout, `npm ci`, `npm test`, `npm run build`, `npm run test:e2e`, `npm audit --audit-level=low`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, and `git diff --check` passed. The Playwright last-run record reports `passed` with no failed tests. `npm audit` reported zero vulnerabilities.

The local restart check posted one aggregate page view, stopped the service, then restarted it using the same temporary SQLite file. The second process returned a healthy response. The repository's Rust persistence test also passes. Live `/health`, legal routes, manifest, robots, sitemap, and service worker returned 200. Live headers included CSP, nosniff, referrer policy, and accelerometer/gyroscope permissions policy. The rebuilt `index.html`, service worker, main JavaScript, CSS, and mobile hero image had matching SHA-256 bytes to the live deployment.

The absent claims manifest is not treated as a passing empty set; it is the P1 and the 14 untested claims above.
