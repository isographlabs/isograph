import * as path from 'path';
import type { FormattingOptions, TextEdit } from 'vscode';
import { WorkspaceEdit, workspace } from 'vscode';
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

  if (config.autoformatIsoLiterals) {
    workspace.onWillSaveTextDocument((event) => {
      event.waitUntil(
        (async () => {
          try {
            const textEdits: TextEdit[] = await client.sendRequest(
              'textDocument/formatting',
              {
                textDocument: TextDocumentIdentifier.create(
                  event.document.uri.toString(),
                ),
                options: {
                  tabSize: 2,
                  insertSpaces: true,
                } as FormattingOptions,
              },
            );
            const edit = new WorkspaceEdit();
            edit.set(event.document.uri, textEdits);
            await workspace.applyEdit(edit);
          } catch {
            // The client may not be ready yet; save still proceeds.
          }
        })(),
      );
    });
  }

  await client.start();
  context.client = client;
}
