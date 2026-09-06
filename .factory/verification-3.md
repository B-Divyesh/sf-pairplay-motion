# Verification 3: turn phones into motion controllers

**Verdict: FAIL**

Date: 2026-09-06  
Work order: `pairplay-motion-verify-3`  
Live URL: https://pairplay-motion.sociobot.in  
Implementation candidate: `c802933cb65b64e14ef576ab6a3076d8c1250b50`  
Documentation baseline: `85a33071b0b13475fa177b877f66908b4363133b`

There are **4 findings**: 2 P1, 1 P2, and 1 P3. There are **0 untested
declared claims**. This is not a PASS.

## Job, audience, and first action

- Job: turn spare phones into motion controllers for a shared-screen game.
- Audience: friends and families with spare phones.
- First action: **Try it with sample data**.

The words are clear when reached. Their placement does not meet the first-screen
contract on normal phone and desktop viewports; see the first finding.

## Findings

### P1 — the job, audience, and first action are below the initial viewport

The mobile layout puts the large illustration before the useful copy. In a
fresh iPhone 13-sized browser (390 × 664 CSS px), the job heading starts at
468 px and ends at 717 px, the audience starts at 741 px, and the sample action
starts at 827 px. The initial viewport therefore shows the masthead, the image,
and only part of the job heading. It does not show the audience or first action.

This also fails at common desktop heights. At 1280 × 720, the audience ends at
771 px and the sample action ends at 853 px. At 1440 × 900, the action ends at
922 px. Only the 1440 × 1000 check fit all three items without scrolling.

This is the first-screen requirement from the plain-words and site-structure
contracts, and it was a P1 in review 1. The wording was repaired, but the
placement was not. Put the useful copy and sample action before the mobile
image and make the lead fit a normal desktop viewport.

Evidence: `/work/.evidence/verify-3/live-phone-first-screen.png` and
`/work/.evidence/verify-3/live-desktop-first-screen.png`.

### P1 — the page-view write endpoint has no rate limit

Sixty concurrent `POST /api/page-view` requests with the same
`X-Forwarded-For` address all returned 204. None returned 429 or
`Retry-After`. Source review confirms that the public limiter covers only
`/api/rooms` and `/ws`; `/api/page-view` returns before any limiting rule.

This endpoint writes the durable daily aggregate, so an unauthenticated client
can inflate the count and force unbounded SQLite writes. The backend contract
requires every server endpoint except health to be rate limited, with stricter
limits on writes. Add per-forwarded-client and global limits with a 429 response
and `Retry-After`, then add a claim or backend integration test for this route.

### P2 — the declared Apple touch icon returns the 404 page

The document head and web manifest both declare `/apple-touch-icon.png` as a
180 × 180 PNG. The live request returns HTTP 404, `text/html`, and the 2,137-byte
SPA 404 document. The file exists in the built frontend, but the Axum router
does not serve this root asset. `/icon.svg` and the social preview image both
return 200 with the correct media type.

Serve `/apple-touch-icon.png` as the checked-in PNG and include it in the
offline asset policy.

### P3 — three phone links are narrower than the 44 px touch-target minimum

At a 320 px viewport, the header **Terms** link measures 40.98 × 44 px. The
footer **Privacy** and **Terms** links measure 42.63 × 44 px and 35.39 × 44 px.
The attached accessibility and design contracts require targets of at least
44 × 44 CSS px. Give these links enough horizontal padding or minimum width.

## Declared claims

All 14 commands in `.factory/claims.json` were run individually from the clean
documented setup. Every command passed; none was inferred from the full suite.

| Claim | Result |
| --- | --- |
| `no-app` | PASS |
| `no-account` | PASS |
| `demo-sandbox` | PASS |
| `sensor-retention` | PASS |
| `two-to-four-phone-support` | PASS |
| `free-dead-still` | PASS |
| `price` | PASS |
| `one-time-license` | PASS |
| `license-storage` | PASS |
| `touch-keyboard-fallback` | PASS |
| `server-memory` | PASS |
| `aggregate-page-count` | PASS |
| `no-trackers` | PASS |
| `rate-limits` | PASS |

Public landing, legal, README, and controller copy was cross-checked against
the manifest. No additional unlisted claim was found. Synthetic
`DeviceOrientationEvent` input also calibrated a live phone-sized controller,
sent motion through the deployed WebSocket, and marked it calibrated on the
host. The disclosed browser and hardware variation remains accurate.

The US $8 claim promises an offer and the declared test verifies its wording
and Sociobot product path. The external checkout still returns the already
reported HTTP 404 because billing registration is unavailable. No completed
checkout or entitlement is claimed, so this known operator dependency is not
counted as an additional product finding.

## Live product checks

- Demo: `/demo` immediately showed Ada 34 and Lin 28. The persistent sample
  label remained after playing and resetting. `Reset demo` restored the result,
  `Start for real` cleared `demo:pairplay-motion`, a seeded real-data key stayed
  unchanged, and the entire demo made only same-origin static requests.
- Normal room: two fresh phone-sized controller pages joined a fresh live room,
  calibrated with touch, sent Arrow and Space motion frames, and started Dead
  Still. Synthetic phone motion events also completed the motion calibration
  path and reached the live host.
- Boundaries and recovery: four players joined; a fifth received the full-room
  message; a long name was limited to 24 characters; a short code produced one
  alert; and a missing room produced the specific recovery message.
- Isolation: two fresh rooms had different codes. Joining room A changed its
  roster to one player while room B stayed at zero.
- Routing: `/`, `/demo`, `/privacy`, and `/terms` returned 200 with unique
  titles, one h1, and a main landmark. `/missing-page` deliberately returned
  404 with the designed page, its own title, and a way home.
- Keyboard and motion: Tab reached the skip link first. Its focus ring was a
  3 px solid signal outline with a 3 px offset. Route changes and browser Back
  focused the new h1. Reduced motion reduced the tested transition to 0.00001 s.
- Accessibility: Playwright Axe found zero violations on the 320 px landing
  page and zero serious or critical violations on every route. There were no
  console or page errors, no horizontal overflow at 320 or 390 px, and the
  privacy page provides a working privacy contact.
- Offline and update: the service worker registered, used
  `pairplay-shell-v3`, fetched with `Cache-Control: no-cache`, called
  `skipWaiting()` and `clients.claim()`, and reloaded the shell offline with an
  explicit offline status.
- Links and assets: all ordinary internal links returned 200. The deliberate
  unknown route returned 404 as designed. The Apple icon failure is listed
  above.

## Backend checks

- `/health` returned 200 and build
  `85a33071b0b13475fa177b877f66908b4363133b`. One hundred concurrent health
  requests all returned 200.
- Room creation returned 200 for the first 12 requests from one forwarded
  client, then 429 with `Retry-After: 5`; a different client remained allowed.
- WebSocket handshakes returned normal validation responses for the first 30
  requests from one forwarded client, then 429 with `Retry-After: 5`.
- The page-view rate-limit failure is listed above.
- The targeted durable-mirror restart test passed and retained the aggregate
  count. The full Rust suite also passed its SQLite schema and persistence
  tests.
- Security headers include CSP, `nosniff`, strict-origin referrer policy, and a
  sensor permissions policy.

## Candidate and deployment comparison

`c802933` is the last implementation commit. The only repository change from
that commit to documentation baseline `85a3307` is `.factory/handoff.md`; no
product source changed. The live `/health` identity is the later documentation
SHA `85a3307`.

Fresh local and live SHA-256 values matched for `index.html`, the main JS and
CSS, the service worker, manifest, robots file, sitemap, and hero image. The
Apple icon mismatch is the routing defect above. The built initial bundles are
26.34 kB gzip JS plus a 10.14 kB lazy JS chunk and 3.54 kB gzip CSS. The mobile
hero is 33,204 bytes.

## Local checks

- `npm ci` — passed; 158 packages audited with 0 vulnerabilities.
- `npm test` — passed: 0 Svelte errors or warnings, 5 frontend tests, 2
  container-contract tests, and 9 Rust tests.
- `npm run build` — passed and produced `dist/`.
- `npm run test:e2e` — 16/16 passed. Its mobile project does not cover the
  phone placement finding because controller pages opened through
  `browser.newPage()` use default desktop dimensions.
- Every exact claim command — 14/14 passed individually.
- `npm audit --audit-level=low`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt -- --check`, and `git diff --check` — passed.
- `/opt/fleet/lib/verify-url.sh` — passed: HTTP 200, 623 ms load, correct title
  and language, one h1, main landmark, alt text, labelled buttons, and no
  browser errors.

## Earlier findings

| Earlier finding | Current disposition |
| --- | --- |
| Live rooms closed with `4004 Room not found` | Fixed. Multiple fresh live rooms and controller joins passed. |
| No demo sandbox | Fixed. Populated, resettable, persistent-label isolation passed live. |
| Missing claims manifest and 14 untested claims | Fixed. All 14 exact commands passed individually. |
| First screen lacked plain job wording | Partly fixed. The words are correct, but their placement still fails the first-screen contract; P1 above. |
| Forwarded-client room limiting missing | Fixed for `/api/rooms` and `/ws`. The separate unprotected page-view write is a new P1. |
| Missing route titles and designed 404 | Fixed. |
| Duplicate invalid-code alerts | Fixed; exactly one alert was present. |
| Build identity did not identify the release | Fixed. The live documentation SHA contains the unchanged `c802933` implementation. |
| Full-room and missing-room recovery were misleading | Fixed live. |
| Durable aggregate restart was not proved | Fixed by the existing live restart record and a fresh targeted restart test. |

## Final result

**FAIL — 4 findings, 0 untested claims.**

