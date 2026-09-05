# PairPlay Motion — review handoff

**FAIL** as of 2026-09-05.

Implementation candidate: `bb3c87c69f1df1ac1d316a48f57591bd5399f75c`.

Review documentation commit: `ae5821e9f2e741a80103f2e04eb35e7ed7b5314e`.

The live host journey is release-blocking: creating a room then opening the host WebSocket twice produced `4004 Room not found` and the LOST screen. The product also has no required sample demo/sandbox, no claims manifest or claim-level tests, non-plain first-screen copy, forwarded-client rate limiting is absent, route titles/404 are incomplete, and the earlier duplicate live alert remains.

See `.factory/review-1.md` for reproduction, evidence, all seven findings, 14 untested public claims, command results, and repair steps. Do not treat the earlier PASS reports as current release approval.

## How to verify after repair

Run `npm ci`, `npm test`, `npm run build`, `npm run test:e2e`, `npm audit`, Clippy, formatting, and the new per-claim demo commands. On the deployed URL, create a host room and join two fresh controller browsers through calibration, touch fallback, and a round. Then verify `/demo`, reset, route titles, `/404`, the per-forwarded-IP 429 response, and physical phone sensor permission.

Docker and Podman were unavailable for this review, so OCI assembly was not repeated.
