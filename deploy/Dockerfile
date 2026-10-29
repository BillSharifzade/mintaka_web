# mintaka.tj web server: a static musl binary in a distroless image, non-root, read-only.
#
# Build from the repository root, after the site generator has written dist/:
#
#   cargo xtask build
#   docker build -f deploy/Dockerfile -t mintaka-web .
#
# Run (read-only root file system, no capabilities; the container listens on unprivileged ports,
# the host maps 443/tcp, 443/udp and 80 onto them). The server gets its own certificate from
# Let's Encrypt (TLS-ALPN-01 on 443/tcp, so public port 443 must reach the container) and keeps
# the account key and certificates in the /data volume — which must persist, or every restart
# orders a new certificate and runs into Let's Encrypt's rate limits:
#
#   docker run --read-only --cap-drop ALL --security-opt no-new-privileges \
#     -p 443:8443/tcp -p 443:8443/udp -p 80:8080 \
#     -v mintaka-data:/data \
#     -e MINTAKA_CONTACT_RECIPIENT=age1… \
#     mintaka-web --acme-domain mintaka.tj --acme-domain www.mintaka.tj \
#       --acme-contact mailto:ops@mintaka.tj \
#       --region fra-1 --location "Frankfurt,DE,50.1109,8.6821"
#
# Add `--acme-directory staging` for the first run (see README). With certificates from
# elsewhere instead: `-v /etc/letsencrypt/live/mintaka.tj:/certs:ro` and
# `--tls-cert /certs/fullchain.pem --tls-key /certs/privkey.pem` in place of the --acme-* flags.
#
# HTTP/3 listens on UDP 8443 inside the container; `--http3-advertise-port 443` makes Alt-Svc
# announce the port clients actually reach.
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
# Contact briefs and the ACME cache (both mode 0600) live on a volume; everything else is
# read-only.
VOLUME ["/data"]
USER nonroot:nonroot
EXPOSE 8443/tcp 8443/udp 8080/tcp
STOPSIGNAL SIGTERM
# The certificate source (--acme-domain … or --tls-cert/--tls-key) comes from `docker run`.
ENTRYPOINT ["/usr/local/bin/mintaka-server", \
            "--dist", "/srv/dist", \
            "--data-dir", "/data", \
            "--listen", "127.0.0.1:9080", \
            "--tls-listen", "[::]:8443", \
            "--http3", \
            "--http3-advertise-port", "443", \
            "--redirect-http", "[::]:8080"]
