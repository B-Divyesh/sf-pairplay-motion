# PairPlay Motion demo

Open [`/demo`](https://pairplay-motion.sociobot.in/demo), or choose **Try it
with sample data** on the landing page.

The demo immediately shows a realistic finished Dead Still round: Ada scores
34 and Lin scores 28. It is a browser-only sample room; it does not call the
room API, open a WebSocket, send a page view, read a saved license, or change
real browser data.

The persistent banner says **Demo — sample data, nothing is saved**. **Reset
demo** restores the two-player result. **Start for real** discards the
`sessionStorage` key `demo:pairplay-motion` and returns to the ordinary landing
page. Real browser data uses its existing non-demo keys and is not read or
written while the banner is visible.
