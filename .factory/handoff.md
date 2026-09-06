# PairPlay Motion — repair handoff

## Status

The repaired product is deployed and the free core is verified at
https://pairplay-motion.sociobot.in.

Implementation deployed: `c802933cb65b64e14ef576ab6a3076d8c1250b50`.

Container image: `sociobotregistry.azurecr.io/sf-pairplay-motion@sha256:9b4b88b50bfdc168039ba3aa9a51db80bc296a7642a222d7dacdfc74ef457621`.

Live revision: `sf-pairplay-motion--0000012`, one replica, with the
`sf-pairplay-motion-data` Azure Files share mounted at `/data`.

PairPlay Motion is for friends and families who want a room game with spare
phones. The first action is **Try it with sample data**.

## What changed

- Fixed the release-blocking room relay path. A room is created and held by the
  host process, and the product remains on one replica because rooms and live
  motion are deliberately process-local.
- Added `/demo` with a realistic completed Ada/Lin Dead Still round. The
  persistent banner says **Demo — sample data, nothing is saved**, includes
  Reset demo and Start for real, and uses the `demo:pairplay-motion` session
  namespace without reading or changing real browser data.
- Added `.factory/claims.json` with 14 sandboxed, outcome-based Playwright
  checks and `.factory/demo.md` describing the sandbox.
- Rewrote the first screen and route copy in plain words. It has route titles,
  a designed 404, legal routes, a skip link, one h1 per route, focus handling,
  and a visible focus treatment.
- Added forwarded-client rate limiting to room creation. The 13th request from
  one client gets `429` with `Retry-After: 5`; another client remains allowed.
- Removed the duplicate error announcement and added recovery copy for invalid
  rooms and full rooms.
- Added generated social and touch assets, sitemap updates, metadata, and the
  catalog description.
- Kept the product state local-first. Rooms and motion are not stored. The only
  durable server value is an anonymous daily page-view aggregate.
- Azure Files does not support SQLite's live locking, rename-overwrite, or
  kernel fast-copy operations. The service therefore runs SQLite in a local
  working file and streams the completed database to
  `/data/pairplay-motion-v3.db` after each aggregate write. A regression test
  starts from the durable copy after a simulated restart and verifies the count.

## Verification

From the documented setup (`npm ci`, Node 22+, Rust stable, Python 3), the
following final-source checks passed:

- `npm test` — Svelte check: 0 errors/warnings; 5 frontend tests; 2 Dockerfile
  contract tests; 9 Rust tests.
- `npm run build` — passed; initial JavaScript is 36.48 kB gzip and CSS is
  3.54 kB gzip.
- `npm run test:e2e` — 16/16 desktop and mobile browser checks passed.
- `npm run test:claims` — 14/14 claim checks passed.
- Every exact command in `.factory/claims.json` was also run individually and
  passed; results are in `/work/.evidence/claims-final.json`.
- `npm audit --audit-level=low` — 0 vulnerabilities.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and
  `git diff --check` — passed.
- The live verification helper passed: HTTPS 200, title, `lang=en`, one h1,
  main landmark, alt text, labelled buttons, and no console errors. Its final
  report is `/work/.evidence/final-verify/verify.json`.
- Live Playwright axe scan found 0 violations, including 0 serious or critical.
- Fresh desktop and phone contexts showed the job, audience, and sample action
  before scrolling. The phone layout had no horizontal overflow. Final
  screenshots and browser evidence are in `/work/.evidence/live-*-final.*`.
- Live sample data showed Ada 34 and Lin 28, retained its sample banner after
  reset, and left a seeded real-data key unchanged.
- A new public host room accepted two fresh phone-sized controllers, calibrated
  both with touch fallback, and started Dead Still. Invalid `/missing-page`
  returned the deliberate 404 page with its own title.
- A live page-view request returned 204, produced a non-empty 12,288-byte
  durable SQLite file, and the exact revision was restarted. It came back
  healthy with the same implementation SHA and durable-mirror startup log.
- The deployed rate-limit allowance was checked with forwarded client headers:
  12 room requests returned 200, the 13th returned 429 with `Retry-After: 5`,
  and a different client returned 200.

The frontend did not change after the recorded current-asset Lighthouse run:
mobile Performance 99, Accessibility 100, Best Practices 100, SEO 100; FCP
1.1 s, LCP 1.8 s, TBT 100 ms, CLS 0. The report is
`/work/.evidence/lighthouse.json`.

## Paid offer and known dependencies

The product retains its US $8 one-time full-edition offer for News Desk and
Ink Runner. The free Dead Still game remains usable. Public billing metadata is
in `.factory/billing-offer.json` and `/work/.evidence/billing-offer.json`.

Billing registration is the remaining external dependency: on 2026-09-06 the
Sociobot checkout endpoint returned HTTP 404 with `enabled factory product`.
No checkout or entitlement is claimed as verified until the separate billing
operator registers the offer. The license storage and verification path remain
implemented and tested with a fixture token; no provider credential is in this
repository.

Physical device-motion permission was not automated on a real handset in this
container. The live touch and keyboard fallback was exercised on phone-sized
browser contexts, and permission-denied recovery remains available.

The original external QA files named in the work order,
`/work/.evidence/qa-report.md` and `/work/.evidence/qa-result.json`, were not
present in this worker. The committed review history, including
`.factory/review-1.md`, was read and its findings are addressed above.

## Deploy and run

Run locally with `npm ci`, `npm run build`, then `cargo run`. The container
needs only `PORT` and defaults to 8080. Deploy with the product helper and
`WO_DATA_DIR=/data` so its share and one-replica bound are preserved.
