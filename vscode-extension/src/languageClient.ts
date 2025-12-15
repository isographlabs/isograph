import * as path from 'path';
import type { FormattingOptions, TextEdit } from 'vscode';
import { Range, window, workspace, WorkspaceEdit } from 'vscode';
import type { LanguageClientOptions } from 'vscode-languageclient';
import {
  RevealOutputChannelOn,
  TextDocumentIdentifier,
} from 'vscode-languageclient';
import type { ServerOptions } from 'vscode-languageclient/node';
import { LanguageClient } from 'vscode-languageclient/node';
import { getConfig } from './config';
import type { IsographExtensionContext } from './context';

export function createAndStartLanguageClient(
  context: IsographExtensionContext,
) {
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
    markdown: {
      isTrusted: true,
    },
    documentSelector: [
      { scheme: 'file', language: 'javascript' },
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'javascriptreact' },
    ],

    outputChannel: context.lspOutputChannel,

    revealOutputChannelOn: RevealOutputChannelOn.Never,

    initializationFailedHandler: (error) => {
      context?.primaryOutputChannel.appendLine(
        `initializationFailedHandler ${error}`,
      );

      return true;
    },
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
    workspace.onWillSaveTextDocument(async (event) => {
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
          } catch {}
        })(),
      );
    });
  }

  client.start();
  context.client = client;
}

type DidNotError = boolean;

export async function killLanguageClient(
  context: IsographExtensionContext,
): Promise<DidNotError> {
  if (context.client == null) {
    return true;
  }

  return context.client
    .stop()
    .then(() => {
      context.primaryOutputChannel.appendLine(
        'Successfully stopped existing isograph lsp client',
      );

      context.client = null;

      return true;
    })
    .catch(() => {
      window.showErrorMessage(
        'An error occurred while trying to stop the Isograph LSP Client. Try restarting VSCode.',
      );

      return false;
    });
}
