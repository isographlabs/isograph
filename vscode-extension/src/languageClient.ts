import * as path from 'path';
import type { FormattingOptions, TextEdit } from 'vscode';
import { workspace } from 'vscode';
import type { LanguageClientOptions } from 'vscode-languageclient';
import {
  RevealOutputChannelOn,
  TextDocumentIdentifier,
} from 'vscode-languageclient';
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

  if (config.autoformatIsoLiterals) {
    workspace.onWillSaveTextDocument((event) => {
      event.waitUntil(
        (async (): Promise<TextEdit[]> => {
          try {
            const textEdits: TextEdit[] | null | undefined =
              await client.sendRequest('textDocument/formatting', {
                textDocument: TextDocumentIdentifier.create(
                  event.document.uri.toString(),
                ),
                options: {
                  tabSize: 2,
                  insertSpaces: true,
                } as FormattingOptions,
              });
            return textEdits ?? [];
          } catch {
            return [];
          }
        })(),
      );
    });
  }
}
