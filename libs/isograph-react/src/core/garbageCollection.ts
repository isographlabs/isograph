import { getParentRecordKey } from './cache';
import { NormalizationAstNodes, type NormalizationAst } from './entrypoint';
import { Variables } from './FragmentReference';
import {
  assertLink,
  DataId,
  IsographEnvironment,
  IsographStore,
  StoreRecord,
  type StoreLink,
  type TypeName,
} from './IsographEnvironment';
import {
  NOT_SET,
  type PromiseWrapper,
  type PromiseWrapperOk,
} from './PromiseWrapper';

export type RetainedQuery = {
  readonly normalizationAst: PromiseWrapper<NormalizationAst>;
  readonly variables: {};
  readonly root: StoreLink;
};

export interface RetainedQueryWithNormalizationAst extends RetainedQuery {
  readonly normalizationAst: PromiseWrapperOk<NormalizationAst>;
}

function isRetainedQueryWithNormalizationAst(
  query: RetainedQuery,
): query is RetainedQueryWithNormalizationAst {
  return (
    query.normalizationAst.result !== NOT_SET &&
    query.normalizationAst.result.kind === 'Ok'
  );
}

export type DidUnretainSomeQuery = boolean;
export function unretainQuery(
  environment: IsographEnvironment,
  retainedQuery: RetainedQuery,
): DidUnretainSomeQuery {
  environment.retainedQueries.delete(retainedQuery);

  if (isRetainedQueryWithNormalizationAst(retainedQuery)) {
    environment.gcBuffer.push(retainedQuery);
  } else if (environment.gcBuffer.length > environment.gcBufferSize) {
    environment.gcBuffer.shift();
    return true;
  }

  return false;
}

export function retainQuery(
  environment: IsographEnvironment,
  queryToRetain: RetainedQuery,
) {
  environment.retainedQueries.add(queryToRetain);
  // TODO can we remove this query from the buffer somehow?
  // We are relying on === equality, but we really should be comparing
  // id + variables
}

export function garbageCollectEnvironment(environment: IsographEnvironment) {
  const retainedIds: RetainedIds = {};

  for (const query of environment.retainedQueries) {
    if (isRetainedQueryWithNormalizationAst(query)) {
      recordReachableIds(environment.store, query, retainedIds);
    } else {
      // if we have any queries with loading normalizationAst, we can't garbage collect
      return;
    }
  }

  for (const query of environment.gcBuffer) {
    recordReachableIds(environment.store, query, retainedIds);
  }

  for (const typeName in environment.store) {
    const dataById = environment.store[typeName];
    if (dataById == null) continue;
    const retainedTypeIds = retainedIds[typeName];

    // delete all objects
    if (retainedTypeIds == undefined || retainedTypeIds.size == 0) {
      delete environment.store[typeName];
      continue;
    }

    for (const dataId in dataById) {
      if (!retainedTypeIds.has(dataId)) {
        delete dataById[dataId];
      }
    }

    if (Object.keys(dataById).length === 0) {
      delete environment.store[typeName];
    }
  }
}

interface RetainedIds {
  [typeName: TypeName]: Set<DataId>;
}

function recordReachableIds(
  store: IsographStore,
  retainedQuery: RetainedQueryWithNormalizationAst,
  mutableRetainedIds: RetainedIds,
) {
  const record =
    store[retainedQuery.root.__typename]?.[retainedQuery.root.__link];

  const retainedRecordsIds = (mutableRetainedIds[
    retainedQuery.root.__typename
  ] ??= new Set());
  retainedRecordsIds.add(retainedQuery.root.__link);

  if (record) {
    recordReachableIdsFromRecord(
      store,
      record,
      mutableRetainedIds,
      retainedQuery.normalizationAst.result.value.selections,
      retainedQuery.variables,
    );
  }
}

function recordReachableIdsFromRecord(
  store: IsographStore,
  currentRecord: StoreRecord,
  mutableRetainedIds: RetainedIds,
  selections: NormalizationAstNodes,
  variables: Variables | null,
) {
  for (const selection of selections) {
    switch (selection.kind) {
      case 'Linked':
        const linkKey = getParentRecordKey(selection, variables ?? {});
        const linkedFieldOrFields = currentRecord[linkKey];

        const links: StoreLink[] = [];
        if (Array.isArray(linkedFieldOrFields)) {
          for (const maybeLink of linkedFieldOrFields) {
            const link = assertLink(maybeLink);
            if (link != null) {
              links.push(link);
            }
          }
        } else {
          const link = assertLink(linkedFieldOrFields);
          if (link != null) {
            links.push(link);
          }
        }

        let typeStore =
          selection.concreteType !== null
            ? store[selection.concreteType]
            : null;

        if (typeStore == null && selection.concreteType !== null) {
          continue;
        }

        for (const nextRecordLink of links) {
          let __typename = nextRecordLink.__typename;

          const resolvedTypeStore = typeStore ?? store[__typename];

          if (resolvedTypeStore == null) {
            continue;
          }

          const nextRecord = resolvedTypeStore[nextRecordLink.__link];
          if (nextRecord != null) {
            const retainedRecordsIds = (mutableRetainedIds[__typename] ??=
              new Set());
            retainedRecordsIds.add(nextRecordLink.__link);
            recordReachableIdsFromRecord(
              store,
              nextRecord,
              mutableRetainedIds,
              selection.selections,
              variables,
            );
          }
        }

        continue;
      case 'Scalar':
        continue;
    }
  }
}
