import { getParentRecordKey } from './cache';
import type { NormalizationAstNodes } from './entrypoint';
import type { Variables } from './FragmentReference';
import type {
  IsographEnvironment,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';
import { getLink, isWithErrors } from './IsographEnvironment';
import { logMessage } from './logging';
import { getStoreRecordProxy } from './optimisticProxy';

export type ShouldFetch = RequiredShouldFetch | 'IfNecessary';
export type RequiredShouldFetch = 'Yes' | 'No';

export const DEFAULT_SHOULD_FETCH_VALUE: ShouldFetch = 'IfNecessary';

type FetchOptionsShared<TReadOutData> = {
  onComplete?: (data: TReadOutData) => void;
  onError?: () => void;
};

export interface FetchOptions<TReadOutData, TRawResponseType>
  extends FetchOptionsShared<TReadOutData> {
  shouldFetch?: ShouldFetch;
  optimisticNetworkResponse?: TRawResponseType;
}

export interface RequiredFetchOptions<TReadOutData>
  extends FetchOptionsShared<TReadOutData> {
  shouldFetch: RequiredShouldFetch;
}

export type CheckResult =
  | {
      kind: 'EnoughData';
    }
  | {
      kind: 'MissingData';
      record: StoreLink;
    };

export function check(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAstNodes,
  variables: Variables,
  root: StoreLink,
): CheckResult {
  const newStoreRecord = getStoreRecordProxy(environment.store, root);

  if (newStoreRecord == null) {
    return {
      kind: 'MissingData',
      record: root,
    };
  }

  const checkResult = checkFromRecord(
    environment,
    normalizationAst,
    variables,
    newStoreRecord,
    root,
  );
  logMessage(environment, () => ({
    kind: 'EnvironmentCheck',
    result: checkResult,
  }));
  return checkResult;
}

function checkFromRecord(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAstNodes,
  variables: Variables,
  record: StoreRecord,
  recordLink: StoreLink,
): CheckResult {
  normalizationAstLoop: for (const normalizationAstNode of normalizationAst) {
    switch (normalizationAstNode.kind) {
      case 'Scalar': {
        const parentRecordKey = getParentRecordKey(
          normalizationAstNode,
          variables,
        );
        const scalarValue = record[parentRecordKey];

        // null means the value is known to be missing, so it must
        // be exactly undefined
        if (scalarValue === undefined) {
          return {
            kind: 'MissingData',
            record: recordLink,
          };
        }
        continue normalizationAstLoop;
      }
      case 'Linked': {
        const parentRecordKey = getParentRecordKey(
          normalizationAstNode,
          variables,
        );

        let linkedValue = record[parentRecordKey];

        if (
          isWithErrors(linkedValue, normalizationAstNode.isFallible ?? false)
        ) {
          if (linkedValue.kind === 'Errors') {
            continue;
          }
          linkedValue = linkedValue.value;
        }

        if (linkedValue === undefined) {
          return {
            kind: 'MissingData',
            record: recordLink,
          };
        } else if (linkedValue == null) {
          continue;
        } else if (Array.isArray(linkedValue)) {
          arrayItemsLoop: for (const item of linkedValue) {
            const link = getLink(item);
            if (link == null) {
              throw new Error(
                'Unexpected non-link in the Isograph store. ' +
                  'This is indicative of a bug in Isograph.',
              );
            }

            const linkedRecord = getStoreRecordProxy(environment.store, link);

            if (linkedRecord === undefined) {
              return {
                kind: 'MissingData',
                record: link,
              };
            } else if (linkedRecord == null) {
              continue arrayItemsLoop;
            } else {
              // TODO in __DEV__ assert linkedRecord is an object
              const result = checkFromRecord(
                environment,
                normalizationAstNode.selections,
                variables,
                linkedRecord,
                link,
              );

              if (result.kind === 'MissingData') {
                return result;
              }
            }
          }
        } else {
          const link = getLink(linkedValue);
          if (link == null) {
            throw new Error(
              'Unexpected non-link in the Isograph store. ' +
                'This is indicative of a bug in Isograph.',
            );
          }

          const linkedRecord = getStoreRecordProxy(environment.store, link);

          if (linkedRecord === undefined) {
            return {
              kind: 'MissingData',
              record: link,
            };
          } else if (linkedRecord == null) {
            continue normalizationAstLoop;
          } else {
            // TODO in __DEV__ assert linkedRecord is an object
            const result = checkFromRecord(
              environment,
              normalizationAstNode.selections,
              variables,
              linkedRecord,
              link,
            );

            if (result.kind === 'MissingData') {
              return result;
            }
          }
        }

        continue normalizationAstLoop;
      }
      case 'InlineFragment': {
        const existingRecordTypename = record['__typename'];

        if (existingRecordTypename == null) {
          return {
            kind: 'MissingData',
            record: recordLink,
          };
        }

        if (existingRecordTypename === normalizationAstNode.type) {
          const result = checkFromRecord(
            environment,
            normalizationAstNode.selections,
            variables,
            record,
            recordLink,
          );

          if (result.kind === 'MissingData') {
            return result;
          }
        }

        continue normalizationAstLoop;
      }
    }
  }

  return {
    kind: 'EnoughData',
  };
}
