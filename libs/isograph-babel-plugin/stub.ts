import type cosmiconfig from 'cosmiconfig';
declare module 'cosmiconfig' {
  export const loadJson: cosmiconfig.LoaderEntry;
}

export {};
