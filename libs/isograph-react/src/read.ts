import { getParentRecordKey, onNextChange } from './cache';
import { getOrCreateCachedComponent } from './componentCache';
import { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
import { FragmentReference } from './FragmentReference';
import {
  assertLink,
  DataId,
  defaultMissingFieldHandler,
  IsographEnvironment,
} from './IsographEnvironment';
import { ReaderAst } from './reader';

export type WithEncounteredRecords<T> = {
  readonly encounteredRecords: Set<DataId>;
  readonly item: T;
};

export function readButDoNotEvaluate<TReadFromStore extends Object>(
  environment: IsographEnvironment,
  reference: FragmentReference<TReadFromStore, unknown>,
): WithEncounteredRecords<TReadFromStore> {
  const mutableEncounteredRecords = new Set<DataId>();
  const response = readData(
    environment,
    reference.readerArtifact.readerAst,
    reference.root,
    reference.variables ?? {},
    reference.nestedRefetchQueries,
    mutableEncounteredRecords,
  );
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('done reading', { response });
  }
  if (response.kind === 'MissingData') {
    throw onNextChange(environment);
  } else {
    return {
      encounteredRecords: mutableEncounteredRecords,
      item: response.data,
    };
  }
}

type ReadDataResult<TReadFromStore> =
  | {
      readonly kind: 'Success';
      readonly data: TReadFromStore;
      readonly encounteredRecords: Set<DataId>;
    }
  | {
      readonly kind: 'MissingData';
      readonly reason: string;
      readonly nestedReason?: ReadDataResult<unknown>;
    };

function readData<TReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: DataId,
  variables: { [index: string]: string },
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredRecords: Set<DataId>,
): ReadDataResult<TReadFromStore> {
  mutableEncounteredRecords.add(root);
  let storeRecord = environment.store[root];
  if (storeRecord === undefined) {
    return {
      kind: 'MissingData',
      reason: 'No record for root ' + root,
    };
  }

  if (storeRecord === null) {
    return {
      kind: 'Success',
      data: null as any,
      encounteredRecords: mutableEncounteredRecords,
    };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        // TODO consider making scalars into discriminated unions. This probably has
        // to happen for when we handle errors.
        if (value === undefined) {
          return {
            kind: 'MissingData',
            reason: 'No value for ' + storeRecordName + ' on root ' + root,
          };
        }
        target[field.alias ?? field.fieldName] = value;
        break;
      }
      case 'Linked': {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        if (Array.isArray(value)) {
          const results = [];
          for (const item of value) {
            const link = assertLink(item);
            if (link === undefined) {
              return {
                kind: 'MissingData',
                reason:
                  'No link for ' +
                  storeRecordName +
                  ' on root ' +
                  root +
                  '. Link is ' +
                  JSON.stringify(item),
              };
            } else if (link === null) {
              results.push(null);
              continue;
            }
            const result = readData(
              environment,
              field.selections,
              link.__link,
              variables,
              nestedRefetchQueries,
              mutableEncounteredRecords,
            );
            if (result.kind === 'MissingData') {
              return {
                kind: 'MissingData',
                reason:
                  'Missing data for ' +
                  storeRecordName +
                  ' on root ' +
                  root +
                  '. Link is ' +
                  JSON.stringify(item),
                nestedReason: result,
              };
            }
            results.push(result.data);
          }
          target[field.alias ?? field.fieldName] = results;
          break;
        }
        let link = assertLink(value);
        if (link === undefined) {
          // TODO make this configurable, and also generated and derived from the schema
          const missingFieldHandler =
            environment.missingFieldHandler ?? defaultMissingFieldHandler;
          const altLink = missingFieldHandler(
            storeRecord,
            root,
            field.fieldName,
            field.arguments,
            variables,
          );
          if (altLink === undefined) {
            return {
              kind: 'MissingData',
              reason:
                'No link for ' +
                storeRecordName +
                ' on root ' +
                root +
                '. Link is ' +
                JSON.stringify(value),
            };
          } else {
            link = altLink;
          }
        } else if (link === null) {
          target[field.alias ?? field.fieldName] = null;
          break;
        }
        const targetId = link.__link;
        const data = readData(
          environment,
          field.selections,
          targetId,
          variables,
          nestedRefetchQueries,
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + storeRecordName + ' on root ' + root,
            nestedReason: data,
          };
        }
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case 'RefetchField': {
        const data = readData(
          environment,
          field.readerArtifact.readerAst,
          root,
          variables,
          // Refetch fields just read the id, and don't need refetch query artifacts
          [],
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + field.alias + ' on root ' + root,
            nestedReason: data,
          };
        } else {
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error('refetchQuery is null in RefetchField');
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            environment,
            // resolvers for refetch fields take 3 args, and this is not reflected in types
            refetchQueryArtifact,
            {
              // @ts-expect-error
              ...data.data,
              ...filterVariables(variables, allowedVariables),
            },
          );
        }
        break;
      }
      case 'MutationField': {
        const data = readData(
          environment,
          field.readerArtifact.readerAst,
          root,
          variables,
          // Mutation don't need refetch query artifacts
          [],
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + field.alias + ' on root ' + root,
            nestedReason: data,
          };
        } else {
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error('refetchQuery is null in MutationField');
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            environment,
            refetchQueryArtifact,
            data.data,
            filterVariables(variables, allowedVariables),
          );
        }
        break;
      }
      case 'Resolver': {
        const usedRefetchQueries = field.usedRefetchQueries;
        const resolverRefetchQueries = usedRefetchQueries.map(
          (index) => nestedRefetchQueries[index],
        );

        const kind = field.readerArtifact.kind;
        if (kind === 'EagerReaderArtifact') {
          const data = readData(
            environment,
            field.readerArtifact.readerAst,
            root,
            variables,
            resolverRefetchQueries,
            mutableEncounteredRecords,
          );
          if (data.kind === 'MissingData') {
            return {
              kind: 'MissingData',
              reason: 'Missing data for ' + field.alias + ' on root ' + root,
              nestedReason: data,
            };
          } else {
            target[field.alias] = field.readerArtifact.resolver(data.data);
          }
        } else if (kind === 'ComponentReaderArtifact') {
          target[field.alias] = getOrCreateCachedComponent(
            environment,
            field.readerArtifact.componentName,
            {
              kind: 'FragmentReference',
              readerArtifact: field.readerArtifact,
              root,
              variables,
              nestedRefetchQueries: resolverRefetchQueries,
            } as const,
          );
        }
        break;
      }
    }
  }
  return {
    kind: 'Success',
    data: target as any,
    encounteredRecords: mutableEncounteredRecords,
  };
}

function filterVariables(
  variables: { [index: string]: string },
  allowedVariables: string[],
): { [index: string]: string } {
  const result: { [index: string]: string } = {};
  for (const key of allowedVariables) {
    result[key] = variables[key];
  }
  return result;
}
