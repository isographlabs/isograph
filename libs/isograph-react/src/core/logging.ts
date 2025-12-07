import { CleanupFn } from '@isograph/disposable-types';
import { NetworkResponseObject, type EncounteredIds } from './cache';
import { CheckResult } from './check';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifact,
  type NormalizationAstNodes,
} from './entrypoint';
import { FragmentReference, Variables } from './FragmentReference';
import {
  IsographEnvironment,
  StoreRecord,
  type StoreLink,
} from './IsographEnvironment';
import { ReadDataResult } from './read';
import { Arguments } from './util';
import type { StoreLayer } from './optimisticProxy';

/**
 * Note: these types are unstable. We will add and remove items from this enum
 * and add and remove fields. Please do not rely on the specifics here (for now).
 *
 * Goals include:
 * - convenient debugging for Isograph developers
 * - eventual support for the Isograph devtools
 *
 * In some cases (e.g. in `AfterNormalization`), we include large objects and thus
 * prevent them from getting garbage collected (if the log message is printed).
 * Especially in cases like that, we intend to remove those!
 */
export type LogMessage =
  | {
      kind: 'AboutToNormalize';
      normalizationAst: NormalizationAstNodes;
      networkResponse: NetworkResponseObject;
      variables: Variables;
    }
  | {
      kind: 'AfterNormalization';
      store: StoreLayer;
      encounteredIds: EncounteredIds;
    }
  | {
      kind: 'DeepEqualityCheck';
      fragmentReference: FragmentReference<any, any>;
      old: object;
      new: object;
      deeplyEqual: boolean;
    }
  | {
      kind: 'ComponentRerendered';
      componentName: string;
      rootLink: StoreLink;
    }
  | {
      kind: 'MakeNetworkRequest';
      artifact:
        | RefetchQueryNormalizationArtifact
        | IsographEntrypoint<any, any, any, any>;
      variables: Variables;
      networkRequestId: string;
    }
  | {
      kind: 'ReceivedNetworkResponse';
      // TODO should be object
      networkResponse: any;
      networkRequestId: string;
    }
  | {
      kind: 'ReceivedNetworkError';
      error: any;
      networkRequestId: string;
    }
  | {
      kind: 'MissingFieldHandlerCalled';
      root: StoreLink;
      storeRecord: StoreRecord;
      fieldName: string;
      arguments: Arguments | null;
      variables: Variables;
    }
  | {
      kind: 'DoneReading';
      response: ReadDataResult<any>;
      fieldName: string;
      root: StoreLink;
    }
  | {
      kind: 'NonEntrypointReceived';
      entrypoint: any;
    }
  | {
      kind: 'EnvironmentCheck';
      result: CheckResult;
    }
  | {
      kind: 'EnvironmentCreated';
    }
  | {
      kind: 'StartUpdateError';
      error: any;
    }
  | {
      kind: 'StartUpdateComplete';
      updatedIds: EncounteredIds;
    }
  | {
      kind: 'ErrorEncounteredInWithErrorHandling';
      error: any;
    };

export type LogFunction = (logMessage: LogMessage) => void;

// wrapped so that items in the loggers set are unique.
export type WrappedLogFunction = {
  log: LogFunction;
};

export function logMessage(
  environment: IsographEnvironment,
  getMessage: () => LogMessage,
) {
  if (environment.loggers.size > 0) {
    const message = getMessage();
    for (const logger of environment.loggers) {
      try {
        logger.log(message);
      } catch {}
    }
  }
}

export function registerLogger(
  environment: IsographEnvironment,
  log: LogFunction,
): CleanupFn {
  const wrapped = { log };
  environment.loggers.add(wrapped);
  return () => {
    environment.loggers.delete(wrapped);
  };
}
