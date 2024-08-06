import { workspace, ConfigurationScope } from 'vscode';

export type Config = {
  rootDirectory: string | null;
  pathToIsograph: string | null;
  pathToConfig: string | null;
};

export function getConfig(scope?: ConfigurationScope): Config {
  const configuration = workspace.getConfiguration('isograph', scope);
  return {
    rootDirectory: configuration.rootDirectory,
    pathToIsograph: configuration.pathToIsograph,
    pathToConfig: configuration.pathToConfig,
  };
}
