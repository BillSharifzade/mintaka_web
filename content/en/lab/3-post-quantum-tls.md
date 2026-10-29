+++
slug = "post-quantum-tls-with-rustls"
title = "Post-quantum TLS in production with rustls"
pillar = "security"
date = 2026-10-29
summary = "Our server offers X25519MLKEM768, the hybrid of X25519 and ML-KEM-768, through rustls and aws-lc-rs. It cannot force it: in TLS 1.3 rustls takes the client's order of preference, so a client that prefers X25519 gets X25519. Measured on a laptop, the hybrid adds exactly 1,184 bytes to the ClientHello, 1,088 bytes to the ServerHello, and about 0.04 ms to a handshake."
repo = "crates/server/src/tls.rs"
+++

## The problem

TLS 1.3 agrees on session keys with an elliptic-curve Diffie–Hellman exchange, usually X25519. A large enough quantum computer running Shor's algorithm would recover those keys from a recorded handshake. Nobody has such a computer today, but traffic can be recorded today and decrypted later, so key exchange is the part of TLS that has to change before the computer exists. Certificates and signatures are a different problem: forging one is only useful while the connection is live.

The standard answer is a hybrid: do X25519 and ML-KEM-768 (the lattice-based key encapsulation NIST standardised as FIPS 203) in the same handshake and derive the keys from both secrets, so an attacker has to break both. The TLS group for it is X25519MLKEM768. This post describes how our server offers it, what it cannot do about clients, and what the hybrid costs, measured.

This is the server we will run in production; the site is not deployed yet. The measurements below come from a laptop, against the same binary with a self-signed certificate.

## Method

### What the server offers

[`tls.rs`](repo:crates/server/src/tls.rs) builds the rustls configuration on the aws-lc-rs provider: TLS 1.3 only (TLS 1.2 behind a flag), ALPN `h2` then `http/1.1`, no 0-RTT, and exactly three key-exchange groups: X25519MLKEM768, X25519 and secp256r1.

### Who decides

Two corrections came out of building this, both recorded in our milestone notes.

The server cannot force post-quantum key exchange. In TLS 1.3, rustls 0.23.46 walks the client's `supported_groups` list in the client's order and takes the first group it also supports; the server's own list only decides what it supports, not the order. If the client sent no key share for that group, rustls answers with a HelloRetryRequest asking for one, which costs a round trip. A client that lists X25519 first gets X25519, and the page says so instead of pretending. Our milestone notes record that current Chrome, Firefox and Safari list X25519MLKEM768 first, so they get it.

The `prefer-post-quantum` feature is not something to switch on: it is a default feature of rustls 0.23, and all it does is put X25519MLKEM768 first in rustls's list of default groups. For this server it changes nothing, because `provider()` replaces that list and the client's order wins anyway. It matters for rustls as a client, where the first group decides which key share goes into the ClientHello.

### How the handshake is timed

The connection layer reads the complete ClientHello first, then starts a clock and finishes the handshake. The interval covers our flight (ServerHello to Finished), one round trip, the client's processing and its Finished. The negotiated group, a `post_quantum` flag and the duration are kept with the connection; they appear in `/_proof` and, on the first response of the connection, as `tls;dur=…` in `Server-Timing`, so a browser can read them.

## The mathematics

The sizes are fixed by the standards. FIPS 203 (Table 3) gives ML-KEM-768 an encapsulation key of 1,184 bytes, a ciphertext of 1,088 bytes and a shared secret of 32 bytes. RFC 10024 (August 2026, formerly the IETF draft draft-ietf-tls-ecdhe-mlkem), sections 4.1 to 4.3, defines the TLS group with codepoint 4588 (0x11EC) and puts the ML-KEM part first:

$$
|\text{client share}| = 1184 + 32 = 1216 \text{ bytes}
$$

$$
|\text{server share}| = 1088 + 32 = 1120 \text{ bytes}
$$

The shared secret is the ML-KEM secret followed by the X25519 secret, 32 + 32 = 64 bytes, and it enters the TLS 1.3 key schedule where an X25519 secret alone would. Compared with X25519 alone, which sends 32 bytes each way, the client sends 1,184 more bytes and the server 1,088 more. Those are the numbers to look for on the wire.

## The code

The provider the server is built on, with the three groups in one place:

```rust include="crates/server/src/tls.rs" from="/// The aws-lc-rs provider restricted"
```

Where the handshake clock starts and stops, in [`conn.rs`](repo:crates/server/src/conn.rs):

```rust include="crates/server/src/conn.rs" from="let handshake = async {"
```

And how a group is called post-quantum, for `/_proof` and curl mode:

```rust include="crates/server/src/tls.rs" from="/// True for the ML-KEM groups, pure or hybrid."
```

## The benchmark

Measured on 2026-10-29 on a laptop with a 13th-generation Intel Core i7-13620H, Arch Linux (kernel 7.2.11-zen1), rustc 1.99.1, the release build of the server (rustls 0.23.46, aws-lc-rs), over loopback. The server ran with `--tls-self-signed`, which generates an ECDSA P-256 certificate at startup, and the client was on the same machine.

### Bytes

We made one handshake per client offer with OpenSSL 3.6.5, reading message lengths with `-msg` and extensions with `-trace`:

```sh title="One handshake, X25519MLKEM768 only"
echo | openssl s_client -connect 127.0.0.1:18843 -servername localhost \
    -groups X25519MLKEM768 -msg -trace
```

| Client offer | ClientHello | Key share | ServerHello | Read | Written | Group |
|---|---:|---:|---:|---:|---:|---|
| X25519MLKEM768 only | 1,490 B | 1,222 B | 1,210 B | 1,775 B | 1,575 B | X25519MLKEM768 |
| X25519 only | 312 B | 38 B | 122 B | 688 B | 397 B | X25519 |
| OpenSSL's default, both shares | 1,546 B | 1,258 B | 1,210 B | 1,777 B | 1,631 B | X25519MLKEM768 |
| X25519 first, then X25519MLKEM768 | 314 B | 38 B | 122 B | 688 B | 399 B | X25519 |
| X25519MLKEM768 first, only an X25519 share | 314 + 1,498 B | 38, then 1,222 B | 88 + 1,210 B | 1,870 B | 1,902 B | X25519MLKEM768 |

Table: Handshake messages for five client offers. Read and written are OpenSSL's totals for the whole handshake, record headers included.

The key share extension grows by 1,184 bytes, exactly the encapsulation key. The ClientHello grows by 1,178, because OpenSSL leaves out its 6-byte `ec_point_formats` extension when it offers no classic elliptic curve. The ServerHello grows by 1,088 bytes, exactly the ciphertext. The fourth row is the correction above, measured: our server lists X25519MLKEM768 first, the client listed X25519 first, and X25519 won. The last row is the other case: the client prefers the hybrid but only sent an X25519 share, so the server asked again (the 88-byte HelloRetryRequest) and got the hybrid one round trip later. The default of OpenSSL 3.6 sends both shares and got the hybrid; curl 8.23, built with that OpenSSL, negotiated X25519MLKEM768 with its defaults too.

### Time

For handshake time, process start-up must stay out of the loop, so we wrote a small client in a scratch directory (not in the repository) with rustls 0.23.46 and aws-lc-rs. It pins the server's certificate but still verifies the handshake signature, turns resumption off, and alternates the two groups for 5,000 fresh connections each after 50 warm-up rounds. For each connection it records its own handshake time and the server's `tls;dur`.

| Group | Client median | Client p99 | Server median | Server p99 |
|---|---:|---:|---:|---:|
| X25519MLKEM768 | 0.244 ms | 1.022 ms | 0.264 ms | 1.115 ms |
| X25519 | 0.203 ms | 0.905 ms | 0.220 ms | 0.989 ms |

Table: 5,000 TLS 1.3 handshakes per group over loopback. The median paired difference was 0.042 ms on the client and 0.044 ms on the server; a first run of 2,000 per group gave 0.041 ms and 0.043 ms.

The same client counted 1,511 bytes written with the hybrid against 327 without (exactly 1,184 more) and 1,791 read against 703, which is the ServerHello's 1,088 (in the first run it was 1,086: an ECDSA signature's encoding varies by a byte or two between connections).

We also timed 300 handshakes per offer with curl and `--curves`. There the server's median `tls;dur` was 1.288 ms with the hybrid, 1.171 ms with X25519 and 1.520 ms with the HelloRetryRequest, about five times the server-side time measured with the rustls client. Each curl process makes exactly one handshake, so most of that is probably per-process start-up in the client, which the server's clock includes because it waits for the client's Finished. The differences point the same way: the hybrid 0.12 ms above X25519, and the HelloRetryRequest another 0.23 ms above the hybrid, even though a loopback round trip is tens of microseconds.

## What it means

On this machine the hybrid costs about 0.04 ms and 2.3 kB per new connection. Next to a network round trip it is small: light in fibre needs at least 47.1 ms between Dushanbe and Frankfurt. Two things are worth watching instead. A 1,546-byte ClientHello no longer fits one 1,500-byte Ethernet packet, and some middleboxes fail on that ([tldr.fail](https://tldr.fail/) collects them; rustls's own manual warns about it). And a client that lists the hybrid first without sending its share pays a full extra round trip, which on a real network is far more than the 0.04 ms.

What is still classical: the certificate and its ECDSA signature, and anything negotiated over TLS 1.2, which the server only allows behind `--tls12`. Hybrid key exchange protects recorded traffic; authentication can change when post-quantum certificates do.
