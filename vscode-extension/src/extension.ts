import * as path from 'path';
import type { ExtensionContext } from 'vscode';
import { window, workspace } from 'vscode';
import { getConfig } from './config';
import type { IsographExtensionContext } from './context';
import { createAndStartLanguageClient } from './languageClient';
import { findIsographBinaryWithWarnings } from './utils/findIsographBinary';

let isographExtensionContext: IsographExtensionContext | null = null;

function workspaceRoot(): string {
  return workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
}

export async function activate(extensionContext: ExtensionContext) {
  isographExtensionContext =
    await buildIsographExtensionContext(extensionContext);

  if (isographExtensionContext != null) {
    isographExtensionContext.primaryOutputChannel.appendLine(
      'Starting the Isograph extension...',
    );

    await createAndStartLanguageClient(isographExtensionContext);
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

  let rootPath = workspaceRoot();
  if (config.rootDirectory != null) {
    rootPath = path.join(rootPath, config.rootDirectory);
  }

  const binary = await findIsographBinaryWithWarnings(
    primaryOutputChannel,
    rootPath,
  );

  if (binary != null) {
    return {
      client: null,
      extensionContext,
      lspOutputChannel,
      primaryOutputChannel,
      isographBinaryExecutionOptions: {
        rootPath,
        binaryPath: binary.path,
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
