/** Set by esbuild (`--define:MINTAKA_DEV=…` in xtask): true in development builds only. */
declare const MINTAKA_DEV: boolean;
/** Set by esbuild (`--define:MINTAKA_BASE=…` in xtask): the site's URL prefix. Read it through web/shared/base.ts. */
declare const MINTAKA_BASE: string | undefined;
