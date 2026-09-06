# PairPlay Motion

PairPlay Motion turns spare phones into motion controllers for a shared screen.
It is for friends and families who want a room game without an app or
account.

Start with [the sample](https://pairplay-motion.sociobot.in/demo). It opens a
finished two-player Dead Still round and does not change real browser data.
Choose **Start for real** to host a room and share its six-character code.

## What is included

- Dead Still is free to play.
- The full collection has a US $8 offer.
- The full collection uses a one-time license with no subscription.
- Touch and keyboard controls work when motion permission is unavailable.
- A room admits four players and refuses a fifth.

Rooms and motion are not written to the durable SQLite database. A page view
changes only the daily aggregate count. The sample makes only same-origin
requests and does not record a page view. Room creation is rate limited per
forwarded client with a `Retry-After` response.

Read [the demo guide](.factory/demo.md), [privacy](https://pairplay-motion.sociobot.in/privacy),
and [terms](https://pairplay-motion.sociobot.in/terms) before a hosted session.

## Run locally

Requirements: Node 22+, npm, Rust stable, Python 3 (for the SQLite claim
checks), and SQLite build dependencies.

```sh
npm ci
npm run build
cargo run
```

Open `http://localhost:8080`. Use HTTPS when checking phone motion permission.
The service starts with no required configuration. It uses `PORT=8080` by
default, serves `dist/` by default, and keeps durable SQLite under `/data` when
that mount exists; otherwise it uses `data/` beside the binary. On Azure Files, SQLite's
live lock files are unsupported, so the service uses a local working copy and
atomically mirrors the complete SQLite database to `/data` after each anonymous
aggregate write. `DATABASE_URL` and `STATIC_DIR` may override those defaults
for local development.

## Test and verify

```sh
npm test
npm run build
npm run test:e2e
npm run test:claims
npm audit --audit-level=low
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Every public claim has a clean sample-sandbox command in
[`.factory/claims.json`](.factory/claims.json). Run each listed command after
`npm ci`; the claim suite starts the built frontend and Axum service itself.

## Deploy

The factory deploys the root Dockerfile as one non-root container. It mounts
the product Azure Files share at `/data` and keeps the service at one replica,
because the live WebSocket relay is process-local. The factory supplies
`BUILD_SHA`; `/health` returns it with `status: ok`. Do not configure payment
credentials in this repository. The hosted Sociobot billing endpoint is the
only checkout integration.

## Browser limits

iOS asks for motion permission from a user action. Older phones can send fewer
sensor readings. Touch controls remain available when permission is denied or
readings do not arrive.

## License

MIT. The generated hero illustration is original to this product. Its source
prompt and provenance are in [`assets/src/hero-broadsheet.json`](assets/src/hero-broadsheet.json).
