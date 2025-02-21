import { insertIfNotExists, type EncounteredIds } from './cache';
import type {
  DataId,
  IsographEnvironment,
  IsographStore,
  StoreRecord,
  TypeName,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { type StorePatch } from './startUpdate';

export function applyAndQueueStartUpdate(
  environment: IsographEnvironment,
  apply: () => StorePatch,
  mutableEncounteredIds: EncounteredIds,
) {
  let revert = applyPatch(environment.store, apply(), mutableEncounteredIds);

  logMessage(environment, {
    kind: 'AppliedUpdate',
    store: environment.store,
    revert,
  });
}

function patchRecord(mutableRecord: StoreRecord, patch: StoreRecord) {
  let revertRecord: StoreRecord = {};
  for (const key in patch) {
    revertRecord[key] = mutableRecord[key];
    mutableRecord[key] = patch[key];
  }
  return revertRecord;
}

export type MutableStorePatch = {
  [index: TypeName]:
    | {
        [index: DataId]: StoreRecord | null | undefined;
      }
    | null
    | undefined;
};

function applyPatch(
  mutableStore: IsographStore,
  storePatch: StorePatch,
  mutableEncounteredIds: EncounteredIds,
): StorePatch {
  let mutableRevert: MutableStorePatch = {};

  for (const typeName in storePatch) {
    let encounteredRecordsIds = insertIfNotExists(
      mutableEncounteredIds,
      typeName,
    );

    let patchById = storePatch[typeName];
    let recordById = mutableStore[typeName];

    if (patchById === null) {
      mutableRevert[typeName] = recordById;
      mutableStore[typeName] = null;
      continue;
    } else if (patchById === undefined) {
      mutableRevert[typeName] = recordById;
      delete mutableStore[typeName];
      continue;
    }
    recordById = mutableStore[typeName] ??= {};
    let revertById = (mutableRevert[typeName] ??= {});

    for (const [recordId, patch] of Object.entries(patchById)) {
      const data = recordById[recordId];

      if (patch === undefined) {
        revertById[recordId] = data;
        delete recordById[recordId];
      } else if (patch == null || data == null) {
        revertById[recordId] = data;
        recordById[recordId] = patch;
      } else {
        revertById[recordId] = patchRecord(data, patch);
      }

      encounteredRecordsIds.add(recordId);
    }
  }
  return mutableRevert;
}
