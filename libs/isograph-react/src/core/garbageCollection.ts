import { getParentRecordKey } from './cache';
import { NormalizationAstNodes } from './entrypoint';
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

export type RetainedQuery = {
  readonly normalizationAst: NormalizationAstNodes;
  readonly variables: {};
  readonly root: StoreLink;
};

export type DidUnretainSomeQuery = boolean;
export function unretainQuery(
  environment: IsographEnvironment,
  retainedQuery: RetainedQuery,
): DidUnretainSomeQuery {
  environment.retainedQueries.delete(retainedQuery);
  environment.gcBuffer.push(retainedQuery);

  if (environment.gcBuffer.length > environment.gcBufferSize) {
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
    recordReachableIds(environment.store, query, retainedIds);
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
  retainedQuery: RetainedQuery,
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
      retainedQuery.normalizationAst,
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
