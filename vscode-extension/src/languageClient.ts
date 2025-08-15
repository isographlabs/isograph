import * as path from 'path';
import {
  FormattingOptions,
  Range,
  TextEdit,
  window,
  workspace,
  WorkspaceEdit,
} from 'vscode';
import {
  LanguageClientOptions,
  RevealOutputChannelOn,
  TextDocumentIdentifier,
} from 'vscode-languageclient';
import { LanguageClient, ServerOptions } from 'vscode-languageclient/node';
import { getConfig } from './config';
import { IsographExtensionContext } from './context';

export function createAndStartLanguageClient(
  context: IsographExtensionContext,
) {
  const config = getConfig();

  context.primaryOutputChannel.appendLine(
    `Using isograph binary: ${context.isographBinaryExecutionOptions.binaryPath}`,
  );

  const args = ['lsp'];

  if (config.pathToConfig) {
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

  // Options to control the language client
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

    // Since we use stderr for debug logs, the "Something went wrong" popup
    // in VSCode shows up a lot. This tells vscode not to show it in any case.
    revealOutputChannelOn: RevealOutputChannelOn.Never,

    initializationFailedHandler: (error) => {
      context?.primaryOutputChannel.appendLine(
        `initializationFailedHandler ${error}`,
      );

      return true;
    },
  };

  // Create the language client and start the client.
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

  // VSCode does not automatically ask the Isograph language server to format tsx (etc)
  // documents, opting to use the built-in formatter. This is a hack that allows us
  // to get the language server to format documents.
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

  // Start the client. This will also launch the server
  client.start();
  context.client = client;
}

type DidNotError = boolean;

export async function killLanguageClient(
  context: IsographExtensionContext,
): Promise<DidNotError> {
  if (!context.client) {
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
