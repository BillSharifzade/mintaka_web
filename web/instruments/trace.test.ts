import { assert, near, test } from './testkit.ts';
import { hops, type Hop, type Timing } from './trace.ts';

const timing = (t: Partial<Timing>): Timing => ({
  fetchStart: 0,
  domainLookupStart: 0,
  domainLookupEnd: 0,
  connectStart: 0,
  secureConnectionStart: 0,
  connectEnd: 0,
  requestStart: 0,
  responseStart: 0,
  responseEnd: 0,
  nextHopProtocol: 'h2',
  ...t,
});

const byId = (list: Hop[]) => Object.fromEntries(list.map((x) => [x.id, x]));

test('a new TLS connection: every hop, network and server split', () => {
  const tr = hops(
    timing({
      fetchStart: 10,
      domainLookupStart: 12,
      domainLookupEnd: 20,
      connectStart: 20,
      secureConnectionStart: 30,
      connectEnd: 45,
      requestStart: 46,
      responseStart: 96,
      responseEnd: 100,
    }),
    0.5,
    true,
  );
  assert.deepEqual(tr.hops.map((x) => x.id), ['dns', 'tcp', 'tls', 'net', 'app', 'down']);
  const h = byId(tr.hops);
  near(h.dns!.ms, 8);
  near(h.tcp!.ms, 10);
  near(h.tls!.ms, 15);
  near(h.net!.ms, 49.5, 1e-9, 'network = first byte − app');
  near(h.app!.ms, 0.5);
  near(h.down!.ms, 4);
  // Positions are relative to fetchStart; the server's share sits in the middle of the round trip.
  near(h.dns!.from, 2);
  near(h.tls!.to, 35);
  near(h.net!.from, 36);
  near(h.net!.to, 86, 1e-9, 'the network bar spans request → first byte');
  near(h.app!.from, 60.75);
  near(h.app!.to, 61.25);
  near(tr.total, 90);
  near(tr.gap, 3, 1e-9, '2 ms before DNS + 1 ms between connect and request');
  assert.equal(tr.reused, false);
  assert.ok(tr.hops.every((x) => x.zero === undefined));
});

test('a reused connection: zero-length hops are kept and say why', () => {
  const tr = hops(
    timing({
      fetchStart: 1000,
      domainLookupStart: 1000,
      domainLookupEnd: 1000,
      connectStart: 1000,
      connectEnd: 1000,
      requestStart: 1000.2,
      responseStart: 1001,
      responseEnd: 1001.05,
    }),
    0.021,
    true,
  );
  const h = byId(tr.hops);
  assert.equal(tr.reused, true);
  for (const id of ['dns', 'tcp', 'tls']) {
    assert.equal(h[id]!.ms, 0, id);
    assert.equal(h[id]!.zero, 'reused', id);
  }
  near(h.net!.ms, 0.779);
  near(h.app!.ms, 0.021);
  near(h.down!.ms, 0.05);
  near(tr.total, 1.05);
  near(tr.gap, 0.2);
});

test('plain HTTP: no TLS hop time, labelled as development', () => {
  const tr = hops(
    timing({ domainLookupEnd: 1, connectStart: 1, connectEnd: 3, requestStart: 3, responseStart: 4, responseEnd: 5 }),
    0.1,
    false,
  );
  const h = byId(tr.hops);
  near(h.tcp!.ms, 2, 1e-9, 'without TLS the connection runs to connectEnd');
  assert.equal(h.tls!.ms, 0);
  assert.equal(h.tls!.zero, 'plain');
});

test('DNS answered from a cache on a new connection', () => {
  const tr = hops(
    timing({ domainLookupStart: 5, domainLookupEnd: 5, connectStart: 5, secureConnectionStart: 9, connectEnd: 20 }),
    0,
    true,
  );
  const h = byId(tr.hops);
  assert.equal(tr.reused, false);
  assert.equal(h.dns!.zero, 'cached');
  near(h.tcp!.ms, 4);
  near(h.tls!.ms, 11);
});

test('without Server-Timing, request → first byte stays one hop', () => {
  const tr = hops(timing({ requestStart: 10, responseStart: 30, responseEnd: 31 }), undefined, true);
  assert.deepEqual(tr.hops.map((x) => x.id), ['dns', 'tcp', 'tls', 'ttfb', 'down']);
  near(byId(tr.hops).ttfb!.ms, 20);
});

test('server time never exceeds the time to first byte', () => {
  const h = byId(hops(timing({ requestStart: 10, responseStart: 11, responseEnd: 11 }), 5, true).hops);
  near(h.app!.ms, 1);
  near(h.net!.ms, 0);
  assert.equal(h.down!.ms, 0, 'an empty body downloads in no time');
});

test('QUIC: the handshake carries TLS', () => {
  const h = byId(
    hops(timing({ nextHopProtocol: 'h3', connectStart: 2, secureConnectionStart: 2, connectEnd: 30 }), 0, true).hops,
  );
  assert.equal(h.tcp!.ms, 0);
  assert.equal(h.tcp!.zero, 'quic');
  near(h.tls!.ms, 28);
});

test('a secureConnectionStart outside the connection is clamped into it', () => {
  const h = byId(hops(timing({ connectStart: 10, secureConnectionStart: 4, connectEnd: 20 }), 0, true).hops);
  near(h.tcp!.ms, 0);
  near(h.tls!.ms, 10);
});
