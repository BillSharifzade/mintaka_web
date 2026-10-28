#!/bin/sh
# Regenerates tests/fixtures/dist: a tiny site in the shape the generator writes (docs/CONTRACTS.md §3),
# with real brotli/gzip siblings and a real CSP hash for the inline style. Needs brotli, gzip, openssl.
set -eu
cd "$(dirname "$0")"
rm -rf dist
mkdir -p dist/en/sent dist/ru dist/assets dist/.well-known dist/text/en dist/text/ru

style='body{margin:0;background:#05060a;color:#f4f5f7}'
hash="sha256-$(printf '%s' "$style" | openssl dgst -sha256 -binary | base64)"

page() { # lang title body
  printf '<!doctype html>\n<html lang="%s"><head><meta charset="utf-8"><title>%s</title>\n<link rel="canonical" href="/%s/"><style>%s</style></head>\n<body><main id="main"><h1>%s</h1>\n%s\n</main><script type="module" src="/assets/main.3F2A1B9C.js"></script></body></html>\n' \
    "$1" "$2" "$1" "$style" "$2" "$3"
}

page en "Software at the edge of physics." '<p>Fast by proof. Secure by attack. Scaled to the metal. Intuitive by experiment.</p><p>Fixture page: the server only moves these bytes around; padding follows so compression is visible. Mintaka, Alnitak and Alnilam form the belt of Orion. Mintaka, Alnitak and Alnilam form the belt of Orion. Mintaka, Alnitak and Alnilam form the belt of Orion.</p>' > dist/en/index.html
page ru "Программы на пределе физики." '<p>Быстро — доказано. Тестовая страница.</p>' > dist/ru/index.html
page en "Your message is on its way." '<p id="e-name-required">Add your name.</p><p id="e-rate-limited">Too many messages from your network.</p>' > dist/en/sent/index.html
page en "This page drifted out of orbit." '<p>The closest star: <!--mintaka:suggest--></p><p><a href="/en/">Navigate by the stars</a></p>' > dist/en/404.html
page ru "Страница сошла с орбиты." '<p>Ближайшая звезда: <!--mintaka:suggest--></p>' > dist/ru/404.html

printf '// fixture bundle\nexport const answer = 42;\nconsole.log("mintaka fixture", answer, answer, answer, answer, answer, answer);\n' > dist/assets/main.3F2A1B9C.js
printf '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="8" cy="8" r="3" fill="#3d7bff"/></svg>\n' > dist/favicon.svg
printf '# Orion, in miniature: * * *\nUser-agent: *\nAllow: /\n' > dist/robots.txt
printf 'Contact: mailto:security@mintaka.tj\nExpires: 2027-10-27T00:00:00Z\nPreferred-Languages: en, ru, tg\n' > dist/.well-known/security.txt

esc=$(printf '\033')
printf '%s[38;2;61;123;255mmintaka%s[0m %s[38;2;201;206;216m- software at the edge of physics%s[0m\n\n  served in {{server_time}} · {{tls}} · {{http}} · build {{build}} · 0 cookies\n' "$esc" "$esc" "$esc" "$esc" > dist/text/en/index.ansi
printf '  efficiency   fast by proof\n  security     secure by attack\n  (served in {{server_time}} over {{http}})\n' > dist/text/en/pillars.ansi
printf '  curl -d name=You -d email=you@example.com https://mintaka.tj/api/contact\n  {{tls}}\n' > dist/text/en/contact.ansi
printf '  mintaka - программы на пределе физики · {{server_time}} · {{tls}}\n' > dist/text/ru/index.ansi

for f in dist/en/index.html dist/assets/main.3F2A1B9C.js; do brotli -q 11 -k -f "$f"; gzip -9 -k -n -f "$f"; done
gzip -9 -k -n -f dist/ru/index.html

cat > dist/manifest.json <<EOF
{
  "version": 1,
  "build": "2026-10-27.f1x7ure0",
  "origin": "https://mintaka.tj",
  "default_lang": "en",
  "langs": ["en", "ru"],
  "csp": { "style_src": ["$hash"], "script_src": [] },
  "routes": [
    { "path": "/en/", "file": "en/index.html", "content_type": "text/html; charset=utf-8", "cache": "document", "encodings": ["br", "gzip"], "lang": "en", "kind": "page" },
    { "path": "/ru/", "file": "ru/index.html", "content_type": "text/html; charset=utf-8", "cache": "document", "encodings": ["gzip"], "lang": "ru", "kind": "page" },
    { "path": "/en/sent/", "file": "en/sent/index.html", "content_type": "text/html; charset=utf-8", "cache": "document", "lang": "en", "kind": "page" },
    { "path": "/assets/main.3F2A1B9C.js", "file": "assets/main.3F2A1B9C.js", "content_type": "text/javascript; charset=utf-8", "cache": "immutable", "encodings": ["br", "gzip"], "kind": "asset" },
    { "path": "/favicon.svg", "file": "favicon.svg", "content_type": "image/svg+xml", "cache": "short", "kind": "file" },
    { "path": "/robots.txt", "file": "robots.txt", "content_type": "text/plain; charset=utf-8", "cache": "short", "kind": "file" },
    { "path": "/.well-known/security.txt", "file": ".well-known/security.txt", "content_type": "text/plain; charset=utf-8", "cache": "short", "kind": "file" }
  ],
  "not_found": { "en": "en/404.html", "ru": "ru/404.html" },
  "text": {
    "en": { "/": "text/en/index.ansi", "/pillars": "text/en/pillars.ansi", "/contact": "text/en/contact.ansi" },
    "ru": { "/": "text/ru/index.ansi" }
  }
}
EOF
echo "fixture written; style hash $hash"
