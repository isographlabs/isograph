import { ConfigurationScope } from 'vscode';

export type Config = {
  rootDirectory: string | null;
  pathToIsograph: string | null;
  pathToConfig: string | null;
};

export function getConfig(scope?: ConfigurationScope): Config {
  return {
    rootDirectory: '.',
    pathToIsograph: null,
    pathToConfig: null,
  };
}
