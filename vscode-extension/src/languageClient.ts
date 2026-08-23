import * as path from 'path';
import type { LanguageClientOptions } from 'vscode-languageclient';
import { RevealOutputChannelOn } from 'vscode-languageclient';
import type { ServerOptions } from 'vscode-languageclient/node';
import { LanguageClient } from 'vscode-languageclient/node';
import { getConfig } from './config';
import type { IsographExtensionContext } from './context';

export async function createAndStartLanguageClient(
  context: IsographExtensionContext,
): Promise<void> {
  const config = getConfig();

  context.primaryOutputChannel.appendLine(
    `Using isograph binary: ${context.isographBinaryExecutionOptions.binaryPath}`,
  );

  const args = ['lsp'];

  if (config.pathToConfig != null) {
    args.push('--config');
    args.push(config.pathToConfig);
  }

  const serverOptions: ServerOptions = {
    options: {
      cwd: context.isographBinaryExecutionOptions.rootPath,
    },
    command: path.resolve(
      context.isographBinaryExecutionOptions.rootPath,
      context.isographBinaryExecutionOptions.binaryPath,
    ),
    args,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'javascript' },
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'javascriptreact' },
    ],
    outputChannel: context.lspOutputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Never,
  };

  const client = new LanguageClient(
    'IsographLanguageClient',
    'Isograph Language Client',
    serverOptions,
    clientOptions,
  );

  context.primaryOutputChannel.appendLine(
    `Starting the Isograph Language Server with these options: ${JSON.stringify(
      serverOptions,
    )}`,
  );

  await client.start();
  context.client = client;
}
