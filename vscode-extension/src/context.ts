import { ExtensionContext, OutputChannel, Terminal } from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export type IsographExtensionContext = {
  client: LanguageClient | null;
  lspOutputChannel: OutputChannel;
  extensionContext: ExtensionContext;
  primaryOutputChannel: OutputChannel;
  compilerTerminal: Terminal | null;
  isographBinaryExecutionOptions: {
    rootPath: string;
    binaryPath: string;
    binaryVersion?: string;
  };
};
