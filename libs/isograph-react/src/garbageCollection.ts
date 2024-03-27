import {
  DataId,
  DataTypeValue,
  IsographEnvironment,
  IsographStore,
  ROOT_ID,
  StoreRecord,
} from './IsographEnvironment';
import { NormalizationAst } from './index';
import { getParentRecordKey } from './cache';

export type RetainedQuery = {
  normalizationAst: NormalizationAst;
  variables: {};
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

function getLinkedId(data: Exclude<DataTypeValue, null | void>): string {
  // @ts-expect-error
  if (data.__link != null) {
    // @ts-expect-error
    return data.__link;
  } else {
    throw new Error('Record in an invalid state');
  }
}

function recordReachableIdsFromRecord(
  store: IsographStore,
  currentRecord: StoreRecord,
  mutableRetainedIds: Set<DataId>,
  selections: NormalizationAst,
  variables: { [index: string]: string } | null,
) {
  for (const selection of selections) {
    switch (selection.kind) {
      case 'Linked':
        const linkKey = getParentRecordKey(selection, variables ?? {});
        const linkedFieldOrFields = currentRecord[linkKey];

        const ids = [];
        if (Array.isArray(linkedFieldOrFields)) {
          for (const link of linkedFieldOrFields) {
            if (link != null) {
              const id = getLinkedId(link);
              ids.push(id);
            }
          }
        } else {
          if (linkedFieldOrFields != null) {
            const id = getLinkedId(linkedFieldOrFields);
            ids.push(id);
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
