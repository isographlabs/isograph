import { Variables } from './FragmentReference';
import {
  DataId,
  IsographEnvironment,
  IsographStore,
  ROOT_ID,
  StoreRecord,
  assertLink,
  type TypeName,
} from './IsographEnvironment';
import { getParentRecordKey } from './cache';
import { NormalizationAst } from './entrypoint';

export type RetainedQuery = {
  readonly normalizationAst: NormalizationAst;
  readonly variables: {};
  readonly typeName: TypeName;
  readonly root: DataId;
};

type DidUnretainSomeQuery = boolean;
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
  const retainedIds: RetainedIds = { Query: new Set<DataId>([ROOT_ID]) };

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
    if (!retainedTypeIds) {
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
  const record = store[retainedQuery.typeName]?.[retainedQuery.root];
  if (record)
    recordReachableIdsFromRecord(
      store,
      record,
      mutableRetainedIds,
      retainedQuery.normalizationAst,
      retainedQuery.variables,
    );
}

function recordReachableIdsFromRecord(
  store: IsographStore,
  currentRecord: StoreRecord,
  mutableRetainedIds: RetainedIds,
  selections: NormalizationAst,
  variables: Variables | null,
) {
  for (const selection of selections) {
    switch (selection.kind) {
      case 'Linked':
        const linkKey = getParentRecordKey(selection, variables ?? {});
        const linkedFieldOrFields = currentRecord[linkKey];

        const ids = [];
        if (Array.isArray(linkedFieldOrFields)) {
          for (const maybeLink of linkedFieldOrFields) {
            const link = assertLink(maybeLink);
            if (link != null) {
              ids.push(link.__link);
            }
          }
        } else {
          const link = assertLink(linkedFieldOrFields);
          if (link != null) {
            ids.push(link.__link);
          }
        }

        const typeStore = store[selection.concreteType];

        if (!typeStore) {
          continue;
        }

        for (const nextRecordId of ids) {
          const nextRecord = typeStore[nextRecordId];
          if (nextRecord != null) {
            mutableRetainedIds[selection.concreteType] ??= new Set();
            mutableRetainedIds[selection.concreteType].add(nextRecordId);
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
