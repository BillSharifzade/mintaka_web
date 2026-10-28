# mintaka.tj web server: a static musl binary in a distroless image, non-root, read-only.
#
# Build from the repository root, after the site generator has written dist/:
#
#   cargo xtask build
#   docker build -f deploy/Dockerfile -t mintaka-web .
#
# Run (read-only root file system, no capabilities; the container listens on unprivileged ports,
# the host maps 443/80 onto them):
#
#   docker run --read-only --cap-drop ALL --security-opt no-new-privileges \
#     -p 443:8443 -p 80:8080 \
#     -v mintaka-data:/data \
#     -v /etc/letsencrypt/live/mintaka.tj:/certs:ro \
#     -e MINTAKA_CONTACT_RECIPIENT=age1… \
#     mintaka-web --region fra-1 --location "Frankfurt,DE,50.1109,8.6821"
#
# (A local musl build outside Docker needs the `rust-musl` Arch package for the
# x86_64-unknown-linux-musl standard library; no rustup on that machine.)

# ---- build ---------------------------------------------------------------------------------
FROM rust:1.99-alpine AS build
# aws-lc-sys (the rustls crypto provider) compiles C: a toolchain is enough, cmake and perl
# are only a fallback. No OpenSSL anywhere.
RUN apk add --no-cache musl-dev linux-headers gcc g++ make cmake perl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
COPY crates crates
COPY xtask xtask
# Static by construction on musl; spelled out so a toolchain default can't change it silently.
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --locked -p server \
 && cp target/release/mintaka-server /mintaka-server \
 && ! ldd /mintaka-server 2>/dev/null | grep -q '=>'

# ---- run -----------------------------------------------------------------------------------
# distroless/static: no shell, no package manager; ships CA roots, tzdata and a `nonroot` user.
FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /mintaka-server /usr/local/bin/mintaka-server
COPY --chown=0:0 dist /srv/dist
# Contact briefs (age-encrypted, mode 0600) live on a volume; everything else is read-only.
VOLUME ["/data"]
USER nonroot:nonroot
EXPOSE 8443 8080
STOPSIGNAL SIGTERM
ENTRYPOINT ["/usr/local/bin/mintaka-server", \
            "--dist", "/srv/dist", \
            "--data-dir", "/data", \
            "--listen", "127.0.0.1:9080", \
            "--tls-listen", "[::]:8443", \
            "--tls-cert", "/certs/fullchain.pem", \
            "--tls-key", "/certs/privkey.pem", \
            "--redirect-http", "[::]:8080"]
