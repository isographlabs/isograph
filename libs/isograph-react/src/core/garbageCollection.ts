import { getParentRecordKey, TYPENAME_FIELD_NAME } from './cache';
import type { NormalizationAst, NormalizationAstNodes } from './entrypoint';
import type { Variables } from './FragmentReference';
import {
  assertLink,
  isWithErrors,
  type DataId,
  type IsographEnvironment,
  type StoreLayerData,
  type StoreLink,
  type StoreRecord,
  type TypeName,
} from './IsographEnvironment';
import type { BaseStoreLayer } from './optimisticProxy';
import {
  NOT_SET,
  type PromiseWrapper,
  type PromiseWrapperOk,
} from './PromiseWrapper';
import { isArray } from './util';

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
  if (environment.store.kind !== 'BaseStoreLayer') {
    return;
  }

  const retainedQueries: RetainedQueryWithNormalizationAst[] = [];
  for (const query of environment.retainedQueries) {
    if (!isRetainedQueryWithNormalizationAst(query)) {
      return;
    }
    retainedQueries.push(query);
  }

  for (const query of environment.gcBuffer) {
    if (!isRetainedQueryWithNormalizationAst(query)) {
      return;
    }
    retainedQueries.push(query);
  }

  garbageCollectBaseStoreLayer(retainedQueries, environment.store);
}

export function garbageCollectBaseStoreLayer(
  retainedQueries: RetainedQueryWithNormalizationAst[],
  baseStoreLayer: BaseStoreLayer,
) {
  const retainedIds: RetainedIds = {};

  for (const query of retainedQueries) {
    recordReachableIds(baseStoreLayer.data, query, retainedIds);
  }

  for (const typeName in baseStoreLayer.data) {
    const dataById = baseStoreLayer.data[typeName];
    if (dataById == null) continue;
    const retainedTypeIds = retainedIds[typeName];

    // delete all objects
    if (retainedTypeIds === undefined || retainedTypeIds.size === 0) {
      delete baseStoreLayer.data[typeName];
      continue;
    }

    for (const dataId in dataById) {
      if (!retainedTypeIds.has(dataId)) {
        delete dataById[dataId];
      }
    }

    if (Object.keys(dataById).length === 0) {
      delete baseStoreLayer.data[typeName];
    }
  }
}

interface RetainedIds {
  [typeName: TypeName]: Set<DataId>;
}

function recordReachableIds(
  dataLayer: StoreLayerData,
  retainedQuery: RetainedQueryWithNormalizationAst,
  mutableRetainedIds: RetainedIds,
) {
  const record =
    dataLayer[retainedQuery.root.__typename]?.[retainedQuery.root.__link];

  const retainedRecordsIds = (mutableRetainedIds[
    retainedQuery.root.__typename
  ] ??= new Set());
  retainedRecordsIds.add(retainedQuery.root.__link);

  if (record != null) {
    recordReachableIdsFromRecord(
      dataLayer,
      record,
      mutableRetainedIds,
      retainedQuery.normalizationAst.result.value.selections,
      retainedQuery.variables,
    );
  }
}

function recordReachableIdsFromRecord(
  dataLayer: StoreLayerData,
  currentRecord: StoreRecord,
  mutableRetainedIds: RetainedIds,
  selections: NormalizationAstNodes,
  variables: Variables | null,
) {
  for (const selection of selections) {
    switch (selection.kind) {
      case 'InlineFragment':
        if (currentRecord[TYPENAME_FIELD_NAME] === selection.type) {
          recordReachableIdsFromRecord(
            dataLayer,
            currentRecord,
            mutableRetainedIds,
            selection.selections,
            variables,
          );
        }
        continue;
      case 'Linked':
        const linkKey = getParentRecordKey(selection, variables ?? {});
        let linkedFieldOrFields = currentRecord[linkKey];

        if (isWithErrors(linkedFieldOrFields, selection.isFallible)) {
          if (linkedFieldOrFields.kind === 'Errors') {
            continue;
          }
          linkedFieldOrFields = linkedFieldOrFields.value;
        }

        const links: StoreLink[] = [];
        if (isArray(linkedFieldOrFields)) {
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
          selection.concreteType != null
            ? dataLayer[selection.concreteType]
            : null;

        if (typeStore == null && selection.concreteType != null) {
          continue;
        }

        for (const nextRecordLink of links) {
          let __typename = nextRecordLink.__typename;

          const resolvedTypeStore = typeStore ?? dataLayer[__typename];

          if (resolvedTypeStore == null) {
            continue;
          }

          const nextRecord = resolvedTypeStore[nextRecordLink.__link];
          if (nextRecord != null) {
            const retainedRecordsIds = (mutableRetainedIds[__typename] ??=
              new Set());
            retainedRecordsIds.add(nextRecordLink.__link);
            recordReachableIdsFromRecord(
              dataLayer,
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
