import { Variables } from './FragmentReference';
import {
  DataId,
  IsographEnvironment,
  IsographStore,
  ROOT_ID,
  StoreRecord,
  assertLink,
} from './IsographEnvironment';
import { getParentRecordKey } from './cache';
import { NormalizationAst } from './entrypoint';

export type RetainedQuery = {
  readonly normalizationAst: NormalizationAst;
  readonly variables: {};
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
  const retainedIds = new Set<DataId>([ROOT_ID]);

  for (const query of environment.retainedQueries) {
    recordReachableIds(environment.store, query, retainedIds);
  }
  for (const query of environment.gcBuffer) {
    recordReachableIds(environment.store, query, retainedIds);
  }

  for (const dataId in environment.store) {
    if (!retainedIds.has(dataId)) {
      delete environment.store[dataId];
    }
  }
}

function recordReachableIds(
  store: IsographStore,
  retainedQuery: RetainedQuery,
  mutableRetainedIds: Set<DataId>,
) {
  recordReachableIdsFromRecord(
    store,
    store[ROOT_ID],
    mutableRetainedIds,
    retainedQuery.normalizationAst,
    retainedQuery.variables,
  );
}

function recordReachableIdsFromRecord(
  store: IsographStore,
  currentRecord: StoreRecord,
  mutableRetainedIds: Set<DataId>,
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

        for (const nextRecordId of ids) {
          const nextRecord = store[nextRecordId];
          if (nextRecord != null) {
            mutableRetainedIds.add(nextRecordId);
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
