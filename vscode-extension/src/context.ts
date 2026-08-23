import type { ExtensionContext, OutputChannel } from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';

export type IsographExtensionContext = {
  client: LanguageClient | null;
  lspOutputChannel: OutputChannel;
  extensionContext: ExtensionContext;
  primaryOutputChannel: OutputChannel;
  isographBinaryExecutionOptions: {
    rootPath: string;
    binaryPath: string;
  };
};
