// The DevTools greeting: Orion as an hourglass, Betelgeuse in rust and Mintaka in blue, and a
// small frozen `window.mintaka` API for the curious — including the console challenge (ctf.ts,
// loaded on first use).

import { t } from '../shared/i18n.ts';
import { emit } from '../shared/bus.ts';
import { getProof, type Proof } from '../shared/proof.ts';

// Seen from the northern hemisphere: east on the left. `%c` switches colour (see greet()).
const ORION = [
  '                 ·  Meissa',
  ' %cBetelgeuse ●%c---------* Bellatrix',
  '             \\       /',
  '              \\     /',
  '       Alnitak * * %c● Mintaka%c',
  '              /     \\',
  '             /       \\',
  '      Saiph *---------* Rigel',
];

let build: string | undefined;

const challenge = () => import('./ctf.ts');

const T = (key: string): string => t('console-' + key).replace(/%/g, '%%');

export function greet(): void {
  // Own property only: an element with id="mintaka" also makes `'mintaka' in window` true.
  if (Object.hasOwn(window, 'mintaka')) return;
  const token = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const rust = `color: ${token('--betelgeuse')}; font-weight: bold`;
  const blue = `color: ${token('--mintaka')}; font-weight: bold`;
  const [before = '', after = ''] = T('hi').split('{cmd}');
  console.log(`${ORION.join('\n')}\n\n${before}%cmintaka.help()%c${after}`, rust, '', blue, '', blue, '');

  getProof().then(
    (p) => (build = p.server.build),
    () => {},
  );
  const api = {
    /** List what `window.mintaka` can do. */
    help(): void {
      console.log(
        [
          ['mintaka.help()', T('help-help')],
          ['mintaka.proof()', T('help-proof')],
          ['mintaka.terminal()', T('help-terminal')],
          ['mintaka.version', T('help-version')],
          ['mintaka.ctf()', T('help-ctf')],
          ['mintaka.flag(answer)', T('help-flag')],
          ['mintaka.apply()', T('help-apply')],
        ]
          .map(([k, v]) => `${k!.padEnd(22)}${v}`)
          .join('\n'),
      );
    },
    /** Fetch /_proof fresh, log it, and resolve with it. */
    proof(): Promise<Proof> {
      return getProof({ fresh: true }).then((p) => {
        build = p.server.build;
        console.log(p);
        return p;
      });
    },
    /** Open the terminal (same as pressing ~). */
    terminal(): void {
      emit('terminal:open', undefined);
    },
    /** This build of the site, as /_proof reports it. */
    get version(): string {
      return build ?? 'unknown';
    },
    /** Start the console challenge, or show where your run stands. */
    ctf: (): Promise<void> => challenge().then((m) => m.ctf()),
    /** This stage's puzzle data, for your own code. */
    puzzle: () => challenge().then((m) => m.puzzle()),
    /** Check an answer. */
    flag: (answer: unknown): Promise<boolean> => challenge().then((m) => m.flag(answer)),
    /** A hint for this stage. */
    hint: (): Promise<void> => challenge().then((m) => m.hint()),
    /** Forget this run and start a new one. */
    restart: (): Promise<void> => challenge().then((m) => m.restart()),
    /** The application form, once all four stages are solved. */
    apply: (): Promise<void> => challenge().then((m) => m.apply()),
  };
  Object.defineProperty(window, 'mintaka', { value: Object.freeze(api), enumerable: true });

  // Someone who finished the challenge earlier finds the application form waiting.
  if (hasRun()) void challenge().then((m) => m.resume());
}

function hasRun(): boolean {
  try {
    return localStorage.getItem('mintaka-ctf') !== null;
  } catch {
    return false;
  }
}
