import {
  callSubscriptions,
  getParentRecordKey,
  insertEmptySetIfMissing,
  type EncounteredIds,
} from './cache';
import type { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
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
} from './IsographEnvironment';
import { logMessage } from './logging';
import { readPromise, type PromiseWrapper } from './PromiseWrapper';
import {
  readImperativelyLoadedField,
  readLinkedFieldData,
  readLoadablySelectedFieldData,
  readResolverFieldData,
  readScalarFieldData,
  type NetworkRequestReaderOptions,
  type ReadDataResultSuccess,
} from './read';
import type { ReaderAst } from './reader';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  eagerResolverName: string,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (environment.eagerReaderCache[
    stableIdForFragmentReference(fragmentReference, eagerResolverName)
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

    let data = createUpdatableProxy(
      environment,
      fragmentReference,
      networkRequestOptions,
      mutableUpdatedIds,
    );

    try {
      updater(data);
    } catch (e) {
      logMessage(environment, () => ({
        kind: 'StartUpdateError',
        error: e,
      }));
      throw e;
    } finally {
      logMessage(environment, () => ({
        kind: 'StartUpdateComplete',
        updatedIds: mutableUpdatedIds,
      }));
      callSubscriptions(environment, mutableUpdatedIds);
    }
  };
}

export function createUpdatableProxy<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableUpdatedIds: EncounteredIds,
): ExtractUpdatableData<TReadFromStore> {
  const readerWithRefetchQueries = readPromise(
    fragmentReference.readerWithRefetchQueries,
  );

  return readUpdatableData(
    environment,
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
  ).data;
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
    ...(set && {
      set: (newValue) => {
        set(newValue);
        mutableState.lastInvalidated++;
      },
    }),
  });
}

function readUpdatableData<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: StoreLink,
  variables: ExtractParameters<TReadFromStore>,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableState: MutableInvalidationState,
  mutableUpdatedIds: EncounteredIds,
): ReadDataResultSuccess<ExtractUpdatableData<TReadFromStore>> {
  let storeRecord = environment.store[root.__typename]?.[root.__link];
  if (storeRecord == null) {
    return {
      kind: 'Success',
      data: null as any,
    };
  }

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
            );
            if (data.kind === 'MissingData') {
              throw new Error(data.reason);
            }
            return data.data;
          },
          field.isUpdatable
            ? (newValue) => {
                storeRecord[storeRecordName] = newValue;
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
                  ast,
                  root,
                  variables,
                  nestedRefetchQueries,
                  networkRequest,
                  networkRequestOptions,
                  mutableState,
                  mutableUpdatedIds,
                ),
            );
            if (data.kind === 'MissingData') {
              throw new Error(data.reason);
            }
            return data.data;
          },
          'isUpdatable' in field && field.isUpdatable
            ? (newValue) => {
                if (Array.isArray(newValue)) {
                  storeRecord[storeRecordName] = newValue.map((node) =>
                    assertLink(node?.__link),
                  );
                } else {
                  storeRecord[storeRecordName] = assertLink(newValue?.__link);
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
          );
          if (data.kind === 'MissingData') {
            throw new Error(data.reason);
          }
          return data.data;
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
          );
          if (data.kind === 'MissingData') {
            throw new Error(data.reason);
          }
          return data.data;
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
          );
          if (data.kind === 'MissingData') {
            throw new Error(data.reason);
          }
          return data.data;
        });
        break;
      }
      case 'Link': {
        target[field.alias] = root;
        break;
      }
      default: {
        field satisfies never;
        throw new Error('Unexpected case.');
      }
    }
  }

  return {
    kind: 'Success',
    data: target as any,
  };
}
