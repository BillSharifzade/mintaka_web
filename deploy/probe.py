"""Non-destructive probes of one Mintaka instance (docs/SECURITY-REVIEW.md). Usage: probe.py host port base [tls]
Prints one line per check: id, what was sent, what came back. Scoped to this site's own paths."""
import json, socket, ssl, sys, time, threading

host, port, base = sys.argv[1], int(sys.argv[2]), sys.argv[3].rstrip('/')
use_tls = len(sys.argv) > 4 and sys.argv[4] == 'tls'
UA = 'Mozilla/5.0 (mintaka pentest)'

def conn():
    s = socket.create_connection((host, port), timeout=15)
    if use_tls:
        ctx = ssl.create_default_context(); ctx.check_hostname = False; ctx.verify_mode = ssl.CERT_NONE
        ctx.set_alpn_protocols(['http/1.1'])
        # A name gets SNI (an edge routes by it); an address is our self-signed LAN preview.
        sni = 'localhost' if host.replace('.', '').isdigit() or ':' in host else host
        s = ctx.wrap_socket(s, server_hostname=sni)
    return s

def raw(data: bytes, read_timeout=6.0, keep=False):
    s = conn(); s.settimeout(read_timeout)
    s.sendall(data)
    out = b''
    t0 = time.time()
    try:
        while True:
            chunk = s.recv(65536)
            if not chunk: break
            out += chunk
            if not keep and b'\r\n\r\n' in out:
                head, _, body = out.partition(b'\r\n\r\n')
                cl = [l for l in head.split(b'\r\n') if l.lower().startswith(b'content-length:')]
                if cl and len(body) >= int(cl[0].split(b':')[1]): break
                if b'transfer-encoding: chunked' in head.lower() and body.endswith(b'0\r\n\r\n'): break
    except socket.timeout:
        pass
    except (ConnectionResetError, ssl.SSLError) as e:
        out += f'<<{type(e).__name__}>>'.encode()
    s.close()
    return out, time.time() - t0

def req(method, path, headers=None, body=b'', version='HTTP/1.1'):
    h = {'Host': host, 'User-Agent': UA, 'Connection': 'close'}
    h.update(headers or {})
    if body and 'Content-Length' not in h and 'Transfer-Encoding' not in h:
        h['Content-Length'] = str(len(body))
    lines = [f'{method} {path} {version}'] + [f'{k}: {v}' for k, v in h.items()]
    return ('\r\n'.join(lines) + '\r\n\r\n').encode() + body

def status(resp):
    line = resp.split(b'\r\n', 1)[0].decode(errors='replace')
    return line or '(no response)'

def header(resp, name):
    for l in resp.split(b'\r\n\r\n', 1)[0].split(b'\r\n')[1:]:
        k, _, v = l.partition(b':')
        if k.strip().lower() == name.lower().encode(): return v.strip().decode(errors='replace')
    return None

def body(resp):
    return resp.split(b'\r\n\r\n', 1)[1] if b'\r\n\r\n' in resp else b''

def report(cid, what, result):
    print(f'{cid:<6} {what:<58} {result}')

B = base
# 1. Information disclosure on an ordinary page
r, _ = raw(req('GET', f'{B}/en/'))
report('I1', 'GET page: Server / X-Powered-By headers', f'server={header(r, "server")} x-powered-by={header(r, "x-powered-by")}')
for h in ['content-security-policy', 'cross-origin-opener-policy', 'cross-origin-embedder-policy', 'cross-origin-resource-policy',
          'permissions-policy', 'referrer-policy', 'x-content-type-options', 'strict-transport-security', 'access-control-allow-origin', 'set-cookie']:
    v = header(r, h)
    report('I2', f'  {h}', (v[:110] + '…') if v and len(v) > 110 else v)

# 2. Methods
for m in ['TRACE', 'OPTIONS', 'PUT', 'DELETE', 'PATCH', 'CONNECT']:
    path = f'{B}/en/' if m != 'CONNECT' else f'{host}:{port}'
    r, _ = raw(req(m, path, {'X-Probe': 'reflect-me'}))
    report('M1', f'{m} {path}', f'{status(r)} allow={header(r, "allow")} reflected={b"reflect-me" in r}')

# 3. Paths that must not reach anything but the route table
for p in ['/en/../_proof', '/%2e%2e/%2e%2e/etc/passwd', '//en/', '/en//', '/EN/', '/en/%00', '/assets/../manifest.json',
          '/manifest.json', '/.git/config', '/dist/', '/en/index.html', '/en/index.html.br', '/text/en/index.ansi',
          '/data/contact/', '/ctf.key', '/.env', '/en/%2e%2e/%2e%2e/manifest.json', '/en/é', '/assets/', '/' + 'a' * 300]:
    r, _ = raw(req('GET', B + p))
    b = body(r)
    leak = any(x in b for x in [b'"routes"', b'root:x:', b'[core]', b'AGE-SECRET', b'BEGIN'])
    report('P1', f'GET {B}{p[:50]}', f'{status(r)} len={len(b)} leak={leak} echoes-path={p.encode()[:40] in b and len(p) > 3}')
# Outside the prefix — only against our own upstream: through a shared host these paths belong to
# other applications, which are out of scope.
if B and host.replace('.', '').isdigit():
    for p in ['/', '/en/', '/_proof', '/api/contact', B + 'x/en/']:
        r, _ = raw(req('GET', p))
        report('P2', f'GET {p} (outside the prefix)', f'{status(r)} body={body(r)[:20]!r}')

# 4. HTTP/1.1 parsing / smuggling probes
smuggle = {
    'CL+TE': b'POST ' + B.encode() + b'/_rum HTTP/1.1\r\nHost: x\r\nContent-Length: 4\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\n\r\n0\r\n\r\nGET ' + B.encode() + b'/_proof HTTP/1.1\r\nHost: x\r\n\r\n',
    'dup CL': b'POST ' + B.encode() + b'/_rum HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\nContent-Length: 40\r\n\r\n{}',
    'TE obfuscated': b'POST ' + B.encode() + b'/_rum HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: xchunked\r\nContent-Length: 2\r\n\r\n{}',
    'TE chunked,identity': b'POST ' + B.encode() + b'/_rum HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked, identity\r\n\r\n2\r\n{}\r\n0\r\n\r\n',
    'space before colon': b'GET ' + B.encode() + b'/en/ HTTP/1.1\r\nHost: x\r\nContent-Length : 0\r\n\r\n',
    'obs-fold': b'GET ' + B.encode() + b'/en/ HTTP/1.1\r\nHost: x\r\nX-A: a\r\n b\r\n\r\n',
    'bare LF': b'GET ' + B.encode() + b'/en/ HTTP/1.1\nHost: x\n\n',
    'no Host (1.1)': b'GET ' + B.encode() + b'/en/ HTTP/1.1\r\nUser-Agent: x\r\n\r\n',
    'absolute-form other host': b'GET http://evil.example' + B.encode() + b'/_proof HTTP/1.1\r\nHost: evil.example\r\n\r\n',
    'bad chunk size': b'POST ' + B.encode() + b'/_rum HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n{}\r\n0\r\n\r\n',
    'HTTP/0.9': b'GET ' + B.encode() + b'/en/\r\n',
    'HTTP/2 preface on h1': b'PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n',
}
for name, data in smuggle.items():
    r, _ = raw(data, keep=True, read_timeout=3)
    statuses = [l.decode(errors='replace') for l in r.split(b'\r\n') if l.startswith(b'HTTP/')]
    report('S1', name, ' | '.join(statuses) or ('(closed) ' + repr(r[:30])))

# 5. Size limits
r, _ = raw(req('GET', B + '/en/?' + 'q' * 100_000))
report('L1', 'URL of 100 kB', status(r))
r, _ = raw(req('GET', B + '/en/', {f'X-H{i}': 'v' for i in range(200)}))
report('L2', '200 headers', status(r))
r, _ = raw(req('GET', B + '/en/', {'X-Big': 'v' * 70_000}))
report('L3', 'one 70 kB header', status(r))
for path, size, ctype in [('/api/contact', 33 * 1024, 'application/json'), ('/_rum', 3 * 1024, 'text/plain'), ('/api/ctf/start', 2 * 1024, 'application/json')]:
    r, _ = raw(req('POST', B + path, {'Content-Type': ctype, 'Sec-Fetch-Site': 'same-origin'}, b'{' + b' ' * (size - 2) + b'}'))
    report('L4', f'POST {path} {size // 1024} KiB', status(r))
# Chunked body far over the limit: refused without reading it all?
s = conn(); s.settimeout(5)
s.sendall(f'POST {B}/api/contact HTTP/1.1\r\nHost: x\r\nUser-Agent: {UA}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n'.encode())
sent = 0; resp = b''
try:
    for _ in range(64):
        s.sendall(b'4000\r\n' + b' ' * 0x4000 + b'\r\n'); sent += 0x4000
        s.settimeout(0.05)
        try:
            resp += s.recv(4096)
            if resp: break
        except (socket.timeout, BlockingIOError):
            pass
except (BrokenPipeError, ConnectionResetError, ssl.SSLError):
    pass
s.close()
report('L5', 'chunked 1 MiB body to /api/contact', f'answered after {sent // 1024} KiB sent: {status(resp)}')

# 6. Content types and parser abuse
r, _ = raw(req('POST', B + '/api/contact', {'Content-Type': 'text/xml', 'Sec-Fetch-Site': 'same-origin'}, b'<a/>'))
report('C1', 'contact as text/xml', status(r))
deep = b'[' * 5000 + b']' * 5000
r, _ = raw(req('POST', B + '/api/contact', {'Content-Type': 'application/json', 'Sec-Fetch-Site': 'same-origin'}, deep))
report('C2', 'contact JSON nested 5000 deep', f'{status(r)} {body(r)[:60]!r}')
r, _ = raw(req('POST', B + '/api/ctf/flag', {'Content-Type': 'application/json'}, json.dumps({'run': 'r1.' + 'A' * 40, 'flag': '1'}).encode()))
report('C3', 'CTF forged run token', f'{status(r)} {body(r)[:60]!r}')

# 7. Cross-site requests (CSRF) — what a hostile page could make a visitor's browser send
form = b'lang=en&name=Csrf&email=csrf%40example.com&message=hello'
r, _ = raw(req('POST', B + '/api/contact', {'Content-Type': 'application/x-www-form-urlencoded', 'Origin': 'https://evil.example', 'Sec-Fetch-Site': 'cross-site', 'Sec-Fetch-Mode': 'navigate'}, form))
report('X1', 'cross-site no-JS form POST to /api/contact', f'{status(r)} location={header(r, "location")}')
r, _ = raw(req('OPTIONS', B + '/api/contact', {'Origin': 'https://evil.example', 'Access-Control-Request-Method': 'POST', 'Access-Control-Request-Headers': 'content-type'}))
report('X2', 'CORS preflight from evil.example', f'{status(r)} acao={header(r, "access-control-allow-origin")}')
r, _ = raw(req('POST', B + '/_rum', {'Content-Type': 'text/plain', 'Sec-Fetch-Site': 'cross-site'}, b'{"route":"/en/","view":true}'))
report('X3', 'cross-site beacon to /_rum', status(r))

# 8. Open redirects
for p in ['//evil.example', '//evil.example/', '/\\evil.example', '/en/..//evil.example']:
    r, _ = raw(req('GET', B + p))
    report('R1', f'GET {B}{p}', f'{status(r)} location={header(r, "location")}')

# 9. Cache headers on dynamic endpoints
for p in ['/_proof', '/_ping', '/_rum', '/api/pow']:
    r, _ = raw(req('GET', B + p))
    report('H1', f'GET {p} cache-control', f'{status(r)} cc={header(r, "cache-control")} vary={header(r, "vary")}')
r, _ = raw(req('GET', B + '/', {'Accept-Language': 'ru'}))
report('H2', 'GET / negotiated: Vary', header(r, 'vary'))
r, _ = raw(req('GET', B + '/en/', {'X-Forwarded-Host': 'evil.example', 'X-Original-URL': '/admin', 'X-Rewrite-URL': '/admin'}))
report('H3', 'unkeyed-header poisoning probes', f'{status(r)} evil-in-body={b"evil.example" in body(r)}')

# 10. Slow clients
def slow(results, i):
    s = conn(); s.settimeout(30)
    t0 = time.time()
    try:
        s.sendall(f'GET {B}/en/ HTTP/1.1\r\nHost: x\r\n'.encode())
        while time.time() - t0 < 25:
            s.sendall(b'X-Slow: a\r\n'); time.sleep(1)
        results[i] = 'still open after 25 s'
    except (BrokenPipeError, ConnectionResetError, ssl.SSLError, OSError):
        results[i] = f'closed after {time.time() - t0:.1f} s'
    s.close()
def idle(results, i):
    s = conn(); s.settimeout(30); t0 = time.time()
    try:
        d = s.recv(1)
        results[i] = f'idle closed after {time.time() - t0:.1f} s ({d!r})'
    except Exception as e:
        results[i] = f'idle: {type(e).__name__} after {time.time() - t0:.1f} s'
    s.close()
res = [None] * 4
ts = [threading.Thread(target=slow, args=(res, i)) for i in range(3)] + [threading.Thread(target=idle, args=(res, 3))]
for t in ts: t.start()
for t in ts: t.join()
report('D1', 'slowloris (3 connections, 1 header/s)', '; '.join(res[:3]))
report('D2', 'connection that never sends', res[3])
