import {
  getParentRecordKey,
  insertEmptySetIfMissing,
  type EncounteredIds,
} from './cache';
import type { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
import { ReadFieldAggregateError } from './errors';

import {
  stableIdForFragmentReference,
  type ExtractParameters,
  type ExtractStartUpdate,
  type ExtractUpdatableData,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import {
  assertLink,
  type IsographEnvironment,
  type StoreLink,
  type WithErrors,
  type WithErrorsData,
} from './IsographEnvironment';
import { logMessage } from './logging';
import {
  addStartUpdateStoreLayer,
  getMutableStoreRecordProxy,
  getOrInsertRecord,
  type StartUpdateStoreLayer,
  type StoreLayer,
} from './optimisticProxy';
import { readPromise, type PromiseWrapper } from './PromiseWrapper';
import {
  readImperativelyLoadedField,
  readLinkedFieldData,
  readLoadablySelectedFieldData,
  readResolverFieldData,
  readScalarFieldData,
  type NetworkRequestReaderOptions,
  type ReadDataResultSuccess,
  type ReadFieldErrors,
} from './read';
import type { ReaderAst } from './reader';
import { callSubscriptions } from './subscribe';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (environment.eagerReaderCache[
    stableIdForFragmentReference(fragmentReference)
  ] ??= createStartUpdate(
    environment,
    fragmentReference,
    networkRequestOptions,
  ));
}

export function createStartUpdate<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (updater) => {
    let mutableUpdatedIds: EncounteredIds = new Map();

    const startUpdate: StartUpdateStoreLayer['startUpdate'] = (storeLayer) => {
      mutableUpdatedIds.clear();
      let updatableData = createUpdatableProxy(
        environment,
        storeLayer,
        fragmentReference,
        networkRequestOptions,
        mutableUpdatedIds,
      );

      try {
        updater({ updatableData });
      } catch (e) {
        logMessage(environment, () => ({
          kind: 'StartUpdateError',
          error: e,
        }));
        throw e;
      }
    };

    environment.store = addStartUpdateStoreLayer(
      environment.store,
      startUpdate,
    );

    logMessage(environment, () => ({
      kind: 'StartUpdateComplete',
      updatedIds: mutableUpdatedIds,
    }));

    callSubscriptions(environment, mutableUpdatedIds);
  };
}

export function createUpdatableProxy<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  storeLayer: StoreLayer,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableUpdatedIds: EncounteredIds,
): ExtractUpdatableData<TReadFromStore> {
  const readerWithRefetchQueries = readPromise(
    fragmentReference.readerWithRefetchQueries,
  );

  return readUpdatableData(
    environment,
    storeLayer,
    readerWithRefetchQueries.readerArtifact.readerAst,
    fragmentReference.root,
    fragmentReference.variables ?? {},
    readerWithRefetchQueries.nestedRefetchQueries,
    fragmentReference.networkRequest,
    networkRequestOptions,
    {
      lastInvalidated: 0,
    },
    mutableUpdatedIds,
  ).item.value;
}

type MutableInvalidationState = {
  lastInvalidated: number;
};

function defineCachedProperty<T>(
  target: T,
  property: PropertyKey,
  mutableState: MutableInvalidationState,
  get: () => any,
  set?: (v: any) => void,
) {
  let value:
    | { kind: 'Set'; value: T; validatedAt: number }
    | {
        kind: 'NotSet';
      } = {
    kind: 'NotSet',
  };

  Object.defineProperty(target, property, {
    configurable: false,
    enumerable: true,
    get: () => {
      if (
        value.kind === 'NotSet' ||
        value.validatedAt < mutableState.lastInvalidated
      ) {
        value = {
          kind: 'Set',
          value: get(),
          validatedAt: mutableState.lastInvalidated,
        };
      }
      return value.value;
    },
    ...(set != null && {
      set: (newValue) => {
        set(newValue);
        mutableState.lastInvalidated++;
      },
    }),
  });
}

function readUpdatableData<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  storeLayer: StoreLayer,
  ast: ReaderAst<TReadFromStore>,
  root: StoreLink,
  variables: ExtractParameters<TReadFromStore>,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableState: MutableInvalidationState,
  mutableUpdatedIds: EncounteredIds,
): ReadDataResultSuccess<WithErrorsData<ExtractUpdatableData<TReadFromStore>>> {
  const storeRecord = getMutableStoreRecordProxy(storeLayer, root);
  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        const storeRecordName = getParentRecordKey(field, variables);

        defineCachedProperty(
          target,
          field.alias ?? field.fieldName,
          mutableState,
          () => {
            const data = readScalarFieldData(
              field,
              storeRecord,
              root,
              variables,
              [],
            );
            switch (data.kind) {
              case 'MissingData':
                throw new Error(data.reason);
              case 'Success':
                return readDataOrThrowOnError(data.item);
            }
          },
          field.isUpdatable
            ? (newValue) => {
                const storeRecord = getOrInsertRecord(storeLayer.data, root);
                storeRecord[storeRecordName] = {
                  kind: 'Data',
                  value: newValue,
                };
                const updatedIds = insertEmptySetIfMissing(
                  mutableUpdatedIds,
                  root.__typename,
                );
                updatedIds.add(root.__link);
              }
            : undefined,
        );
        break;
      }
      case 'Linked': {
        const storeRecordName = getParentRecordKey(field, variables);
        defineCachedProperty(
          target,
          field.alias ?? field.fieldName,
          mutableState,
          () => {
            const data = readLinkedFieldData(
              environment,
              field,
              storeRecord,
              root,
              variables,
              nestedRefetchQueries,
              networkRequest,
              networkRequestOptions,
              (ast, root) =>
                readUpdatableData(
                  environment,
                  storeLayer,
                  ast,
                  root,
                  variables,
                  nestedRefetchQueries,
                  networkRequest,
                  networkRequestOptions,
                  mutableState,
                  mutableUpdatedIds,
                ),
              [],
            );
            if (data.kind === 'MissingData') {
              throw new Error(data.reason);
            }
            return readDataOrThrowOnError(data.item);
          },
          'isUpdatable' in field && field.isUpdatable
            ? (newValue) => {
                const storeRecord = getOrInsertRecord(storeLayer.data, root);
                if (Array.isArray(newValue)) {
                  storeRecord[storeRecordName] = {
                    kind: 'Data',
                    value: newValue.map((node) => assertLink(node?.__link)),
                  };
                } else {
                  storeRecord[storeRecordName] = {
                    kind: 'Data',
                    value: assertLink(newValue?.__link),
                  };
                }
                const updatedIds = insertEmptySetIfMissing(
                  mutableUpdatedIds,
                  root.__typename,
                );
                updatedIds.add(root.__link);
              }
            : undefined,
        );
        break;
      }
      case 'ImperativelyLoadedField': {
        defineCachedProperty(target, field.alias, mutableState, () => {
          const data = readImperativelyLoadedField(
            environment,
            field,
            root,
            variables,
            nestedRefetchQueries,
            networkRequest,
            networkRequestOptions,
            new Map(),
            [],
          );
          switch (data.kind) {
            case 'MissingData':
              throw new Error(data.reason);
            case 'Success':
              return readDataOrThrowOnError(data.item);
          }
        });
        break;
      }
      case 'Resolver': {
        defineCachedProperty(target, field.alias, mutableState, () => {
          const data = readResolverFieldData(
            environment,
            field,
            root,
            variables,
            nestedRefetchQueries,
            networkRequest,
            networkRequestOptions,
            new Map(),
            [],
          );
          switch (data.kind) {
            case 'MissingData':
              throw new Error(data.reason);

            case 'Success':
              return readDataOrThrowOnError(data.item);
          }
        });
        break;
      }
      case 'LoadablySelectedField': {
        defineCachedProperty(target, field.alias, mutableState, () => {
          const data = readLoadablySelectedFieldData(
            environment,
            field,
            root,
            variables,
            networkRequest,
            networkRequestOptions,
            new Map(),
            [],
          );

          switch (data.kind) {
            case 'MissingData':
              throw new Error(data.reason);
            case 'Success':
              return readDataOrThrowOnError(data.item);
          }
        });
        break;
      }
      case 'Link': {
        target[field.alias] = root;
        break;
      }
    }
  }

  return {
    kind: 'Success',
    item: {
      kind: 'Data',
      value: target as any,
    },
  };
}

function readDataOrThrowOnError<T>(result: WithErrors<T, ReadFieldErrors>) {
  switch (result.kind) {
    case 'Errors':
      throw new ReadFieldAggregateError(result.errors);
    case 'Data': {
      return result.value;
    }
  }
}
