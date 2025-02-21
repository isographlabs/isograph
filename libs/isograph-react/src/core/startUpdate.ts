import {
  callSubscriptions,
  getParentRecordKey,
  type EncounteredIds,
  type NetworkResponseScalarValue,
} from './cache';
import {
  stableIdForFragmentReference,
  type ExtractParameters,
  type ExtractStartUpdate,
  type ExtractStartUpdateUpdatableData,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import {
  assertLink,
  type DataId,
  type IsographEnvironment,
  type Link,
  type StoreRecord,
  type TypeName,
} from './IsographEnvironment';
import {
  applyAndQueueStartUpdate,
  type MutableStorePatch,
} from './optimisitcUpdate';
import { readPromise } from './PromiseWrapper';
import { readButDoNotEvaluate, type NetworkRequestReaderOptions } from './read';
import type { ReaderAst } from './reader';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, any>,
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
    const readerWithRefetchQueries = readPromise(
      fragmentReference.readerWithRefetchQueries,
    );

    const apply = () => {
      const mutablePatch: StorePatch = {};
      const data = proxyUpdatableData(
        environment,
        readerWithRefetchQueries.readerArtifact.readerAst,
        fragmentReference.variables ?? {},
        readButDoNotEvaluate(
          environment,
          fragmentReference,
          networkRequestOptions,
        ).item,
        mutablePatch,
      ) as ExtractStartUpdateUpdatableData<ExtractStartUpdate<TReadFromStore>>;

      updater(data);

      return mutablePatch;
    };

    let mutableEncounteredIds: EncounteredIds = new Map();
    applyAndQueueStartUpdate(environment, apply, mutableEncounteredIds);

    callSubscriptions(environment, mutableEncounteredIds);
  };
}

export type StorePatch = {
  readonly [index: TypeName]:
    | {
        // we extend IsographStore to include `undefined`
        // `undefined` means record needs to be deleted
        readonly [index: DataId]: StoreRecord | null | undefined;
      }
    | null
    | undefined;
};

export const LINK = Symbol('link');

type ReadData = {
  [LINK]: Link;
  [key: PropertyKey]: unknown;
};

function isReadData(value: unknown): value is ReadData {
  console.log(value);
  return typeof value === 'object' && value !== null && LINK in value;
}

function proxyUpdatableData<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  variables: ExtractParameters<TReadFromStore>,
  readData: unknown,
  mutableUpdate: MutableStorePatch,
) {
  if (!isReadData(readData)) {
    throw new Error(
      'Expected readData to be a Record with a LINK property. ' +
        'This is indicative of a bug in Isograph.',
    );
  }

  const root = readData[LINK];

  let updatableData: Record<PropertyKey, typeof Reflect.set> = {};

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        if (field.isUpdatable) {
          const storeRecordName = getParentRecordKey(field, variables);
          const key = field.alias ?? field.fieldName;
          updatableData[key] = (
            target: object,
            propertyKey: PropertyKey,
            value: NetworkResponseScalarValue,
            receiver?: unknown,
          ) => {
            const recordsById = (mutableUpdate[root.__typename] ??= {});
            const newStoreRecord = (recordsById[root.__link] ??= {});
            newStoreRecord[storeRecordName] = value;

            return Reflect.set(target, propertyKey, value, receiver);
          };
        }
        break;
      }
      case 'Linked': {
        const key = field.alias ?? field.fieldName;

        if (Array.isArray(readData[key])) {
          for (let i = 0; i < readData[key].length; i++) {
            readData[key][i] = proxyUpdatableData(
              environment,
              field.selections,
              variables,
              readData[key][i],
              mutableUpdate,
            );
          }

          break;
        }

        const proxiedValue = proxyUpdatableData(
          environment,
          field.selections,
          variables,
          readData[key],
          mutableUpdate,
        );
        readData[key] = proxiedValue;
        break;
      }
      case 'Resolver':
      case 'Link':
      case 'ImperativelyLoadedField':
      case 'LoadablySelectedField':
        break;
      default: {
        // Ensure we have covered all variants
        let _: never = field;
        _;
        throw new Error('Unexpected case.');
      }
    }
  }

  return Object.keys(updatableData).length
    ? new Proxy(readData, {
        set(target, propertyKey, value, receiver) {
          const setter = updatableData[propertyKey];
          if (setter == undefined) {
            throw new Error(`Error: "${String(propertyKey)}" is read-only`);
          }
          return setter(target, propertyKey, value, receiver);
        },
      })
    : readData;
}
