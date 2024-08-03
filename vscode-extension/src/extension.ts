import { ExtensionContext, window, workspace } from 'vscode';
import { getConfig } from './config';
import { IsographExtensionContext } from './context';
import { findIsographBinaryWithWarnings } from './utils/findIsographBinary';
import path = require('path');
import { createAndStartLanguageClient } from './languageClient';

let isographExtensionContext: IsographExtensionContext | null = null;

export async function activate(extensionContext: ExtensionContext) {
  const config = getConfig();

  isographExtensionContext =
    await buildIsographExtensionContext(extensionContext);

  if (isographExtensionContext) {
    isographExtensionContext.primaryOutputChannel.appendLine(
      'Starting the Isograph extension...',
    );

    createAndStartLanguageClient(isographExtensionContext);
  }
}

async function buildIsographExtensionContext(
  extensionContext: ExtensionContext,
): Promise<IsographExtensionContext | null> {
  const config = getConfig();

  const primaryOutputChannel = window.createOutputChannel('Isograph');
  const lspOutputChannel = window.createOutputChannel('Isograph LSP Logs');

  extensionContext.subscriptions.push(lspOutputChannel);
  extensionContext.subscriptions.push(primaryOutputChannel);

  let rootPath = workspace.rootPath || process.cwd();
  if (config.rootDirectory) {
    rootPath = path.join(rootPath, config.rootDirectory);
  }

  const binary = await findIsographBinaryWithWarnings(primaryOutputChannel);

  if (binary) {
    return {
      client: null,
      extensionContext,
      lspOutputChannel,
      primaryOutputChannel,
      compilerTerminal: null,
      isographBinaryExecutionOptions: {
        rootPath,
        binaryPath: binary.path,
        binaryVersion: binary.version,
      },
    };
  }

  primaryOutputChannel.appendLine(
    'Stopping execution of the Isograph VSCode extension since we could not find a valid compiler binary.',
  );

  return null;
}

export function deactivate(): Thenable<void> | undefined {
  isographExtensionContext?.primaryOutputChannel.dispose();

  return isographExtensionContext?.client?.stop();
}
