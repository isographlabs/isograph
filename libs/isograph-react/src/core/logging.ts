import { CleanupFn } from '@isograph/disposable-types';
import {
  IsographEnvironment,
  IsographStore,
  StoreRecord,
  type Link,
} from './IsographEnvironment';
import {
  IsographEntrypoint,
  NormalizationAst,
  RefetchQueryNormalizationArtifact,
} from './entrypoint';
import { FragmentReference, Variables } from './FragmentReference';
import { NetworkResponseObject, type EncounteredIds } from './cache';
import { Arguments } from './util';
import { ReadDataResult } from './read';
import { CheckResult } from './check';

export type LogMessage =
  | {
      kind: 'GettingSuspenseCacheItem';
      index: string;
      availableCacheItems: ReadonlyArray<string>;
      found: boolean;
    }
  | {
      kind: 'AboutToNormalize';
      normalizationAst: NormalizationAst;
      networkResponse: NetworkResponseObject;
      variables: Variables;
    }
  | {
      kind: 'AfterNormalization';
      store: IsographStore;
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
      rootLink: Link;
    }
  | {
      kind: 'MakeNetworkRequest';
      artifact:
        | RefetchQueryNormalizationArtifact
        | IsographEntrypoint<any, any>;
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
      root: Link;
      storeRecord: StoreRecord;
      fieldName: string;
      arguments: Arguments | null;
      variables: Variables;
    }
  | {
      kind: 'DoneReading';
      response: ReadDataResult<any>;
    }
  | {
      kind: 'NonEntrypointReceived';
      entrypoint: any;
    }
  | {
      kind: 'EnvironmentCheck';
      result: CheckResult;
    };

export type LogFunction = (logMessage: LogMessage) => void;

// wrapped so that items in the loggers set are unique.
export type WrappedLogFunction = {
  log: LogFunction;
};

export function logMessage(
  environment: IsographEnvironment,
  message: LogMessage,
) {
  for (const logger of environment.loggers) {
    try {
      logger.log(message);
    } catch {}
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
