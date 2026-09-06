FROM node:22-alpine AS web
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json vite.config.ts ./
COPY frontend ./frontend
RUN npm run build

FROM rust:1-alpine AS server
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY src ./src
# ACR supplies the full source commit. The default keeps clean local and
# source-tarball builds reproducible without consulting repository metadata.
ARG BUILD_SHA=dev
ENV BUILD_SHA=${BUILD_SHA}
RUN cargo build --release

FROM alpine:3.21
RUN addgroup -S pairplay && adduser -S pairplay -G pairplay && mkdir -p /data && chown pairplay:pairplay /data
WORKDIR /app
COPY --from=server /build/target/release/pairplay-motion /usr/local/bin/pairplay-motion
COPY --from=web /build/dist ./dist
USER pairplay
ENV PORT=8080
EXPOSE 8080
CMD ["pairplay-motion"]
