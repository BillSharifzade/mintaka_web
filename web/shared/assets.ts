// Hashed URLs of assets that code needs by name (workers, WebAssembly, data files). The
// generator writes them into `<script type="application/json" id="mintaka-assets">`.
//
// Names are the entry's file stem: `web/instruments/pow.worker.ts` → "pow.worker".

let map: Record<string, string> | null = null;

export function assetUrl(name: string): string {
  if (!map) {
    const el = document.getElementById('mintaka-assets');
    try {
      map = el?.textContent ? (JSON.parse(el.textContent) as Record<string, string>) : {};
    } catch {
      map = {};
    }
  }
  const url = map[name];
  if (!url) throw new Error(`[mintaka] unknown asset "${name}"`);
  return url;
}
