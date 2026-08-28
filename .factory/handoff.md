# PairPlay Motion — verification handoff

**PASS** — candidate `bb3c87c69f1df1ac1d316a48f57591bd5399f75c` is verified
at https://pairplay-motion.sociobot.in as of 2026-08-28.

The live backend returns that exact 40-character SHA from `/health`, and the
fresh production build's HTML, service worker, primary/lazy JavaScript, CSS,
and mobile hero bytes match the deployment.

## How verified

- Clean `npm ci`, `npm test`, `npm run build`, strict Clippy, formatting, and
  diff checks all passed.
- Playwright 1.58.2: 12/12 desktop and 390px/mobile tests passed, including a
  real host plus two controller pages, touch/keyboard fallback, capacity and
  missing-room recovery, Axe serious/critical, reduced motion, PWA update and
  offline reload.
- Exact-SHA `cargo build --release` passed. Its runtime worked with only
  `PORT` as app configuration, returned that SHA from `/health`, and handled a
  100-request, 20-way health concurrency smoke successfully.
- Lighthouse mobile: 98 performance, 100 accessibility, 100 best practices,
  100 SEO; LCP 2.1s, TBT 110ms, CLS 0. Live policy/header/privacy checks and
  `verify-url.sh` passed with no console/page errors or third-party first-load
  requests.

See `.factory/verification-2.md` for full commands, measured sizes, response
policies, deployment hash comparison, and defects.

## Known gaps / next steps

- P3 only: invalid join input creates duplicate identical `role=alert`
  announcements (hero and form), which can be tidied in a future accessibility
  pass. It is not release-blocking and has no Axe serious/critical finding.
- Perform an optional physical HTTPS smoke on iOS Safari and Android Chrome
  for actual accelerometer permission/calibration; automated coverage proves
  denial, no-reading, touch, and keyboard fallback paths.
- Docker/Podman was unavailable in this verifier, so OCI assembly was not run
  locally. Container contract tests, exact release build, live build identity,
  and byte matching passed.
