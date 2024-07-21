import {ConfigurationScope} from 'vscode';

export type Config = {
  rootDirectory: string | null;
  pathToIsograph: string | null;
  pathToConfig: string | null;
};

export function getConfig(scope?: ConfigurationScope): Config {
  return {
    rootDirectory: '.',
    pathToIsograph:
      '/home/edmondo/Development/isograph/target/debug/isograph_cli',
    pathToConfig:
      '/home/edmondo/Development/isograph/demos/pet-demo/isograph.config.json',
  };
}
