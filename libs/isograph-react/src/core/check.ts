import { getParentRecordKey } from './cache';
import { NormalizationAst } from './entrypoint';
import { Variables } from './FragmentReference';
import {
  getLink,
  IsographEnvironment,
  Link,
  ROOT_ID,
  StoreRecord,
} from './IsographEnvironment';
import { logMessage } from './logging';

export type ShouldFetch = 'Yes' | 'No' | 'IfNecessary';

export const DEFAULT_SHOULD_FETCH_VALUE: ShouldFetch = 'IfNecessary';

export type FetchOptions = {
  shouldFetch?: ShouldFetch;
};

export type CheckResult =
  | {
      kind: 'EnoughData';
    }
  | {
      kind: 'MissingData';
      record: Link;
    };

export function check(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAst,
  variables: Variables,
): CheckResult {
  const checkResult = checkFromRecord(
    environment,
    normalizationAst,
    variables,
    environment.store[ROOT_ID],
    { __link: ROOT_ID },
  );
  logMessage(environment, {
    kind: 'EnvironmentCheck',
    result: checkResult,
  });
  return checkResult;
}

function checkFromRecord(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAst,
  variables: Variables,
  record: StoreRecord,
  recordLink: Link,
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

        const linkedValue = record[parentRecordKey];

        if (linkedValue === undefined) {
          return {
            kind: 'MissingData',
            record: recordLink,
          };
        } else if (linkedValue === null) {
          continue;
        } else if (Array.isArray(linkedValue)) {
          arrayItemsLoop: for (const item of linkedValue) {
            const link = getLink(item);
            if (link === null) {
              throw new Error(
                'Unexpected non-link in the Isograph store. ' +
                  'This is indicative of a bug in Isograph.',
              );
            }

            const linkedRecord = environment.store[link.__link];

            if (linkedRecord === undefined) {
              return {
                kind: 'MissingData',
                record: link,
              };
            } else if (linkedRecord === null) {
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
          if (link === null) {
            throw new Error(
              'Unexpected non-link in the Isograph store. ' +
                'This is indicative of a bug in Isograph.',
            );
          }

          const linkedRecord = environment.store[link.__link];

          if (linkedRecord === undefined) {
            return {
              kind: 'MissingData',
              record: link,
            };
          } else if (linkedRecord === null) {
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

        if (
          existingRecordTypename == null ||
          existingRecordTypename !== normalizationAstNode.type
        ) {
          return {
            kind: 'MissingData',
            record: recordLink,
          };
        }

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

        continue normalizationAstLoop;
      }
      default: {
        let _: never = normalizationAstNode;
        _;
        throw new Error(
          'Unexpected case. This is indicative of a bug in Isograph.',
        );
      }
    }
  }

  return {
    kind: 'EnoughData',
  };
}
