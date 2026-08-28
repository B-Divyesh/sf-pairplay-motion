# PairPlay Motion — independent verification 2

**Result: PASS**

Date: 2026-08-28  
Verifier work order: `pairplay-motion-verify-2`  
Candidate and deployed backend identity: `bb3c87c69f1df1ac1d316a48f57591bd5399f75c`  
Live URL: https://pairplay-motion.sociobot.in

This is a fresh verification from a clean checkout. It supersedes the failed
report for `6c0b0f2`; the deployment-only build-identity defect is fixed.

## Quality gates

| Check | Result and evidence |
| --- | --- |
| Clean install | `npm ci` succeeded from `package-lock.json`; 158 packages audited, 0 vulnerabilities. |
| Static/type/lint | `svelte-check` reported 0 errors and 0 warnings. `cargo clippy --all-targets -- -D warnings`, `cargo fmt -- --check`, and `git diff --check` passed. |
| Unit/integration | `npm test` passed: 5 Vitest game-rule tests, 2 container-contract tests, and 7 Rust route/unit tests. |
| Production build | `npm run build` passed. `BUILD_SHA=bb3c87c69f1df1ac1d316a48f57591bd5399f75c cargo build --release` passed. |
| Browser E2E | `npm run test:e2e` passed **12/12** Playwright 1.58.2 cases across desktop Chromium and iPhone 13/390px Chromium. |
| Runtime | The exact-SHA release binary started with an otherwise empty environment (only `PORT=8090`, plus process `PATH`), returned the exact SHA from `/health`, created a room, rejected malformed WebSocket room input with HTTP 400, and shut down gracefully. A 20-way concurrent smoke of 100 `/health` requests returned 100/100 successful responses. |
| Lighthouse (mobile) | Local production runtime: **98 performance, 100 accessibility, 100 best practices, 100 SEO**; FCP 1.2s, LCP 2.1s, TBT 110ms, CLS 0. |

The Dockerfile's exact OCI build was not run because neither Docker nor Podman
is installed in this verification worker. The checked container contract tests
pass; the exact source build, non-root Dockerfile structure, default build
identity, and live deployed identity were independently verified.

## Product exercise

- Created a host room, joined two controller pages, sent calibration, chose
  the touch fallback, drove it with Arrow keys and Space, and started a
  playable Dead Still round. The browser suite also covers four-player capacity
  and the fifth-player recovery, missing-room recovery, premium lock screen,
  legal routes, and controller/host WebSocket flow.
- Invalid and boundary input covered: malformed/short room codes show “Enter
  the six-character room code”; missing scoreboard names use required native
  validation; malformed WebSocket room input returns 400; a fifth player is
  told the room is full; missing rooms give a specific recovery message.
- Desktop and 390px mobile had no horizontal overflow. Visual inspection found
  the deployed editorial layout legible and intact in both sizes.
- Keyboard and motion: initial Tab reaches the skip link with the tested 3px
  focus outline; controller touch mode accepts arrows and Space. Reduced-motion
  emulation reduces UI transition duration to 0.00001s.
- Accessibility: Axe reported **0 serious or critical** findings. The app has a
  title, `lang=en`, exactly one h1, main landmark, skip link, labelled fields,
  and image alt text. No console errors or page errors occurred on local normal
  use or a fresh live desktop/mobile browser smoke.
- PWA: the E2E suite confirms service-worker registration, `no-cache` update
  policy, versioned cache (`pairplay-shell-v2`), `skipWaiting`/`clients.claim`,
  and an offline reload that retains the visible host action and OFFLINE state.

## Privacy, network, and response policy

- A first-load browser capture on both live desktop and 390px mobile made no
  third-party requests and produced no console/page errors. The only normal
  request besides assets is same-origin `/api/page-view`; the source permits
  Sociobot's billing origin only when a license is being verified/restored.
- No analytics or external fonts/scripts were found. SQLite migration contains
  only `day` and aggregate `views`; rooms and sensor samples are in memory.
- Live responses include CSP, `X-Content-Type-Options: nosniff`,
  `strict-origin-when-cross-origin` referrer policy, and a restricted sensor /
  payment permissions policy. Hashed assets are immutable for one year;
  `sw.js` is `no-cache`.

## Deployment comparison

`GET /health` on the live URL returned:

```json
{"build":"bb3c87c69f1df1ac1d316a48f57591bd5399f75c","status":"ok"}
```

SHA-256 values matched the fresh `dist/` for `index.html`, `sw.js`, primary
JS, lazy QR JS, CSS, and `hero-broadsheet-720.webp`. The initial primary JS is
66,493 B raw (25,890 B gzip); the lazy QR chunk is 25,836 B raw; CSS is 11,255
B raw; no webfonts ship; and the mobile hero is 33,204 B. All are within the
applicable budgets.

`verify-url.sh` against the live URL also passed: HTTP 200 in 627ms, correct
title/lang/one h1/main, no missing alt text or unnamed buttons, and no browser
errors.

## Defects

| Severity | Finding |
| --- | --- |
| P0 | None. |
| P1 | None. |
| P2 | None. |
| P3 | On invalid join input, the same error is rendered in both the hero and join form as `role=alert`, so some screen readers may announce it twice. This does not produce an Axe serious/critical finding and does not block use. |

## Remaining non-blocking verification limits

- Physical iOS Safari and Android Chrome accelerometer permission/calibration
  require a real HTTPS handset; automated coverage verifies explicit denial,
  missing readings, touch fallback, and keyboard fallback.
- An OCI engine was unavailable locally, as noted above. Live build identity
  and shipped-byte matching provide deployment evidence for this candidate.
