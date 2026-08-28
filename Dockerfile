FROM node:22-alpine AS web
WORKDIR /build
COPY package.json package-lock.json* ./
RUN npm install
COPY tsconfig.json vite.config.ts ./
COPY frontend ./frontend
RUN npm run build

FROM rust:1.85-alpine AS server
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY src ./src
ARG BUILD_SHA=container
ENV BUILD_SHA=$BUILD_SHA
RUN cargo build --release

FROM alpine:3.21
RUN addgroup -S pairplay && adduser -S pairplay -G pairplay && mkdir -p /app/data && chown pairplay:pairplay /app/data
WORKDIR /app
COPY --from=server /build/target/release/pairplay-motion /usr/local/bin/pairplay-motion
COPY --from=web /build/dist ./dist
USER pairplay
ENV PORT=8080 STATIC_DIR=/app/dist DATABASE_URL=sqlite:///app/data/pairplay.db?mode=rwc
EXPOSE 8080
CMD ["pairplay-motion"]
