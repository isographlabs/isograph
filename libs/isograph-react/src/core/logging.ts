import type { CleanupFn } from '@isograph/disposable-types';
import type {
  EncounteredIds,
  NetworkResponseError,
  NetworkResponseObject,
} from './cache';
import type { CheckResult } from './check';
import type {
  IsographEntrypoint,
  NormalizationAstNodes,
  RefetchQueryNormalizationArtifact,
} from './entrypoint';

import type { FragmentReference, Variables } from './FragmentReference';
import type {
  IsographEnvironment,
  StoreLink,
  StoreRecord,
  WithErrors,
} from './IsographEnvironment';
import type { NonEmptyArray } from './NonEmptyArray';
import type { StoreLayer } from './optimisticProxy';
import type { ReadDataResult, ReadFieldErrors } from './read';
import type { Arguments } from './util';

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
      networkResponse: NetworkResponseObject | undefined;
      errors: NonEmptyArray<NetworkResponseError> | undefined;
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
      response: ReadDataResult<WithErrors<unknown, ReadFieldErrors>>;
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
