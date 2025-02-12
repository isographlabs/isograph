import {
  callSubscriptions,
  getParentRecordKey,
  insertIfNotExists,
  type EncounteredIds,
} from './cache';
import {
  stableIdForFragmentReference,
  type ExtractParameters,
  type ExtractStartUpdate,
  type ExtractStartUpdateUpdatableData,
  type FragmentReference,
  type UnknownTReadFromStore,
  type Variables,
} from './FragmentReference';
import {
  assertLink,
  type DataTypeValue,
  type IsographEnvironment,
  type Link,
} from './IsographEnvironment';
import { logMessage } from './logging';
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

export type Update = {
  root: Link;
  variables: Variables;
  storeRecordName: string;
  newValue: DataTypeValue;
  oldValue: DataTypeValue;
};

export function createStartUpdate<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (updater) => {
    const mutableUpdates: Update[] = [];
    const readerWithRefetchQueries = readPromise(
      fragmentReference.readerWithRefetchQueries,
    );

    const data = proxyUpdatableData(
      environment,
      readerWithRefetchQueries.readerArtifact.readerAst,
      fragmentReference.root,
      fragmentReference.variables ?? {},
      readButDoNotEvaluate(
        environment,
        fragmentReference,
        networkRequestOptions,
      ).item,
      mutableUpdates,
    ) as ExtractStartUpdateUpdatableData<ExtractStartUpdate<TReadFromStore>>;

    updater(data);

    let mutableEncounteredIds: EncounteredIds = new Map();
    applyUpdates(environment, mutableUpdates, mutableEncounteredIds);
    callSubscriptions(environment, mutableEncounteredIds);
  };
}

function applyUpdates(
  environment: IsographEnvironment,
  updates: Update[],
  mutableEncounteredIds: EncounteredIds,
) {
  for (const update of updates) {
    const storeRecord = ((environment.store[update.root.__typename] ??= {})[
      update.root.__link
    ] ??= {});

    storeRecord[update.storeRecordName] = update.newValue;
    logMessage(environment, {
      kind: 'AppliedUpdate',
      update,
    });

    let encounteredRecordsIds = insertIfNotExists(
      mutableEncounteredIds,
      update.root.__typename,
    );
    encounteredRecordsIds.add(update.root.__link);
  }
}

function proxyUpdatableData<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: Link,
  variables: ExtractParameters<TReadFromStore>,
  readData: object,
  mutableUpdates: Update[],
) {
  let updatableData: Record<PropertyKey, typeof Reflect.set> = {};

  let storeRecord = environment.store[root.__typename]?.[root.__link];
  if (storeRecord === undefined) {
    throw new Error('Expected record for root ' + root.__link);
  }

  if (storeRecord === null) {
    return null;
  }

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        if (field.isUpdatable) {
          const storeRecordName = getParentRecordKey(field, variables);
          const key = field.alias ?? field.fieldName;
          updatableData[key] = (
            target: object,
            propertyKey: PropertyKey,
            value: DataTypeValue,
            receiver?: unknown,
          ) => {
            mutableUpdates.push({
              storeRecordName,
              newValue: value,
              oldValue: Reflect.get(target, propertyKey, receiver),
              root,
              variables,
            });
            return Reflect.set(target, propertyKey, value, receiver);
          };
        }
        break;
      }
      case 'Linked': {
        const key = field.alias ?? field.fieldName;
        const storeRecordName = getParentRecordKey(field, variables);
        const storeValue = storeRecord[storeRecordName];
        if (Array.isArray(storeValue)) {
          const results = [];
          for (let i = 0; i < storeValue.length; i++) {
            const link = assertLink(storeValue[i]);
            if (link === undefined) {
              throw new Error(
                'Expected link for ' +
                  storeRecordName +
                  ' on root ' +
                  root.__link +
                  '. Link is ' +
                  JSON.stringify(storeValue[i]),
              );
            } else if (link === null) {
              results.push(null);
              continue;
            }

            // @ts-expect-error
            const value = readData[key][i];

            const result = proxyUpdatableData(
              environment,
              field.selections,
              link,
              variables,
              value,
              mutableUpdates,
            );

            results.push(result);
          }
          // @ts-expect-error
          readData[key] = results;
          break;
        }
        let link = assertLink(storeValue);

        if (link === undefined) {
          const missingFieldHandler = environment.missingFieldHandler;

          const altLink = missingFieldHandler?.(
            storeRecord,
            root,
            field.fieldName,
            field.arguments,
            variables,
          );

          if (altLink === undefined) {
            throw new Error(
              'Expected link for ' +
                storeRecordName +
                ' on root ' +
                root.__link +
                '. Link is ' +
                JSON.stringify(storeValue),
            );
          } else {
            link = altLink;
          }
        }

        if (link === null) {
          break;
        }

        // @ts-expect-error
        const value = readData[key];

        const mergedValue = proxyUpdatableData(
          environment,
          field.selections,
          link,
          variables,
          value,
          mutableUpdates,
        );
        // @ts-expect-error
        readData[key] = mergedValue;
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
