import { insertEmptySetIfMissing, type EncounteredIds } from './cache';
import { callSubscriptions } from './subscribe';
import type {
  BaseStoreLayerData,
  IsographEnvironment,
  StoreLayerData,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';

export function getOrInsertRecord(
  dataLayer: StoreLayerData,
  link: StoreLink,
): StoreRecord {
  const recordsById = (dataLayer[link.__typename] ??= {});
  return (recordsById[link.__link] ??= {});
}

/**
 * Given the child-most store layer (i.e. environment.store) and a link (identifying a
 * store record), create a proxy object that attempts to read through each successive
 * store layer until a value (i.e. field name) is found. If found, return that value.
 */
export function getStoreRecordProxy(
  storeLayer: StoreLayer,
  link: StoreLink,
): Readonly<StoreRecord> | null | undefined {
  let startNode: StoreLayer | null = storeLayer;
  while (startNode !== null) {
    const storeRecord = startNode.data[link.__typename]?.[link.__link];
    if (storeRecord === null) {
      return null;
    }
    if (storeRecord != null) {
      return getMutableStoreRecordProxy(startNode, link);
    }
    startNode = startNode.parentStoreLayer;
  }

  return undefined;
}

export function getMutableStoreRecordProxy(
  childMostStoreLayer: StoreLayer,
  link: StoreLink,
): StoreRecord {
  return new Proxy<StoreRecord>(
    {},
    {
      get(_, propertyName) {
        let currentStoreLayer: StoreLayer | null = childMostStoreLayer;
        while (currentStoreLayer !== null) {
          const storeRecord =
            currentStoreLayer.data[link.__typename]?.[link.__link];
          if (storeRecord === null) {
            return undefined;
          }
          if (storeRecord != null) {
            const value = Reflect.get(storeRecord, propertyName);
            if (value !== undefined) {
              return value;
            }
          }
          currentStoreLayer = currentStoreLayer.parentStoreLayer;
        }
      },
      has(_, propertyName) {
        let currentStoreLayer: StoreLayer | null = childMostStoreLayer;
        while (currentStoreLayer !== null) {
          const storeRecord =
            currentStoreLayer.data[link.__typename]?.[link.__link];
          if (storeRecord === null) {
            return false;
          }
          if (storeRecord != null) {
            const value = Reflect.has(storeRecord, propertyName);
            if (value !== undefined) {
              return true;
            }
          }
          currentStoreLayer = currentStoreLayer.parentStoreLayer;
        }
        return false;
      },
      set(_, p, newValue) {
        return Reflect.set(
          getOrInsertRecord(childMostStoreLayer.data, link),
          p,
          newValue,
        );
      },
    },
  );
}

export type BaseStoreLayer = {
  readonly kind: 'BaseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | null;
  readonly parentStoreLayer: null;
  readonly data: BaseStoreLayerData;
};

export type NetworkResponseStoreLayer = {
  readonly kind: 'NetworkResponseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer;
  readonly data: StoreLayerData;
};

export type DataUpdate<TStoreLayer extends StoreLayer> = (
  storeLayer: TStoreLayer,
) => void;

export type StartUpdateStoreLayer = {
  readonly kind: 'StartUpdateStoreLayer';
  childStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer;
  data: StoreLayerData;
  startUpdate: DataUpdate<StartUpdateStoreLayer | BaseStoreLayer>;
};

export type OptimisticStoreLayer =
  | OptimisticUpdaterStoreLayer
  | OptimisticNetworkResponseStoreLayer;

export type OptimisticUpdaterStoreLayer = {
  readonly kind: 'OptimisticUpdaterStoreLayer';
  childStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | null;
  parentStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | BaseStoreLayer;
  data: StoreLayerData;
  readonly startUpdate: DataUpdate<OptimisticUpdaterStoreLayer>;
};

export type OptimisticNetworkResponseStoreLayer = {
  readonly kind: 'OptimisticNetworkResponseStoreLayer';
  childStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | null;
  parentStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | BaseStoreLayer;
  data: StoreLayerData;
};

export function addNetworkResponseStoreLayer(
  parent: StoreLayer,
): StoreLayerWithData {
  switch (parent.kind) {
    case 'NetworkResponseStoreLayer':
    case 'BaseStoreLayer': {
      return parent;
    }
    case 'StartUpdateStoreLayer':
    case 'OptimisticNetworkResponseStoreLayer':
    case 'OptimisticUpdaterStoreLayer': {
      const node: NetworkResponseStoreLayer = {
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
      };
      parent.childStoreLayer = node;

      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

function mergeDataLayer(target: StoreLayerData, source: StoreLayerData): void {
  for (const [typeName, sourceById] of Object.entries(source)) {
    if (sourceById == null) {
      target[typeName] = sourceById;
      continue;
    }
    const targetRecordById = (target[typeName] ??= {});
    for (const [id, sourceRecord] of Object.entries(sourceById)) {
      if (sourceRecord === null) {
        targetRecordById[id] = null;
        continue;
      }
      const targetRecord = (targetRecordById[id] ??= {});
      Object.assign(targetRecord, sourceRecord);
    }
  }
}

export function addStartUpdateStoreLayer(
  parent: StoreLayer,
  startUpdate: StartUpdateStoreLayer['startUpdate'],
): StoreLayer {
  switch (parent.kind) {
    case 'BaseStoreLayer': {
      startUpdate(parent);
      return parent;
    }
    case 'StartUpdateStoreLayer': {
      const node = parent;

      const prevStartUpdate = node.startUpdate;
      node.startUpdate = () => {
        prevStartUpdate(node);
        startUpdate(node);
      };

      startUpdate(node);
      return node;
    }
    case 'NetworkResponseStoreLayer':
    case 'OptimisticNetworkResponseStoreLayer':
    case 'OptimisticUpdaterStoreLayer': {
      const node: StartUpdateStoreLayer = {
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
        startUpdate: startUpdate,
      };
      parent.childStoreLayer = node;

      startUpdate(node);
      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

export function addOptimisticUpdaterStoreLayer(
  parent: StoreLayer,
  startUpdate: OptimisticUpdaterStoreLayer['startUpdate'],
): OptimisticUpdaterStoreLayer {
  switch (parent.kind) {
    case 'BaseStoreLayer':
    case 'StartUpdateStoreLayer':
    case 'NetworkResponseStoreLayer':
    case 'OptimisticNetworkResponseStoreLayer':
    case 'OptimisticUpdaterStoreLayer': {
      const node: OptimisticUpdaterStoreLayer = {
        kind: 'OptimisticUpdaterStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
        startUpdate: startUpdate,
      };

      startUpdate(node);
      parent.childStoreLayer = node;

      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

export function addOptimisticNetworkResponseStoreLayer(
  parent: StoreLayer,
): OptimisticNetworkResponseStoreLayer {
  switch (parent.kind) {
    case 'BaseStoreLayer':
    case 'StartUpdateStoreLayer':
    case 'NetworkResponseStoreLayer':
    case 'OptimisticNetworkResponseStoreLayer':
    case 'OptimisticUpdaterStoreLayer': {
      const node: OptimisticNetworkResponseStoreLayer = {
        kind: 'OptimisticNetworkResponseStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
      };

      parent.childStoreLayer = node;

      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

/**
 * Merge storeLayerToMerge, and its children, into baseStoreLayer.
 * We can merge until we reach a revertible layer (i.e. an optimistic layer).
 * All other layers cannot be reverted, so for housekeeping + perf, we merge
 * them into a single layer.
 *
 * Note that BaseStoreLayer.childStoreLayer has type OptimisticStoreLayer | null.
 * So, the state of the stack is never e.g. base <- network response. Instead,
 * we have a base + a child that we would like to attach to the base. So, we merge
 * (flatten) until we reach an optimistic layer or null, at which point, we can
 * set baseStoreLayer.childStoreLayer = storeLayerToMerge (via setChildOfNode).
 */
function mergeLayersWithDataIntoBaseLayer(
  environment: IsographEnvironment,
  storeLayerToMerge: StoreLayer | null,
  baseStoreLayer: BaseStoreLayer,
) {
  while (
    storeLayerToMerge != null &&
    storeLayerToMerge.kind !== 'OptimisticUpdaterStoreLayer'
  ) {
    mergeDataLayer(baseStoreLayer.data, storeLayerToMerge.data);
    storeLayerToMerge = storeLayerToMerge.childStoreLayer;
  }
  setChildOfNode(environment, baseStoreLayer, storeLayerToMerge);
}

/**
 * Now that we have replaced the optimistic layer with a network response layer, we need
 * to
 * - re-execute startUpdate and optimistic nodes, in light of the replaced data, and
 * - create two objects containing the old merged data (from the optimistic update layer
 *   onward) and the new merged data (from the network response layer onward).
 * - we will compare the new and old merged data in order to determine the changed records
 *   and trigger subscriptions.
 *
 * Here, "merged data" means all of the records + fields that were modified, starting at
 * storeLayer, e.g. in BaseLayer <- OptimisticLayer <- StartUpdateLayer, if we
 * are replacing Optimistic, then oldData will contain the records + fields modified by
 * OptimisticLayer + StartUpdateLayer.
 */
function reexecuteUpdatesAndMergeData(
  storeLayer:
    | OptimisticStoreLayer
    | NetworkResponseStoreLayer
    | StartUpdateStoreLayer
    | null,
  // reflects the (now reverted) optimistic layer
  oldMergedData: StoreLayerData,
  // reflects whatever replaced the optimistic layer
  newMergedData: StoreLayerData,
): void {
  while (storeLayer !== null) {
    mergeDataLayer(oldMergedData, storeLayer.data);
    switch (storeLayer.kind) {
      case 'OptimisticNetworkResponseStoreLayer':
      case 'NetworkResponseStoreLayer':
        break;
      case 'StartUpdateStoreLayer': {
        storeLayer.data = {};
        storeLayer.startUpdate(storeLayer);
        break;
      }
      case 'OptimisticUpdaterStoreLayer': {
        storeLayer.data = {};
        storeLayer.startUpdate(storeLayer);
        break;
      }
      default: {
        storeLayer satisfies never;
        throw new Error('Unreachable. This is a bug in Isograph.');
      }
    }
    mergeDataLayer(newMergedData, storeLayer.data);

    storeLayer = storeLayer.childStoreLayer;
  }
}

/**
 * Set storeLayerToModify's child to a given layer. This may be null!
 * If it is null, set the environment.store to storeLayerToModify.
 * If it is not null, then the existing environment.store value remains
 * valid.
 */
function setChildOfNode<TStoreLayer extends StoreLayer>(
  environment: IsographEnvironment,
  storeLayerToModify: TStoreLayer,
  newChildStoreLayer: TStoreLayer['childStoreLayer'],
) {
  storeLayerToModify.childStoreLayer = newChildStoreLayer;
  if (newChildStoreLayer !== null) {
    newChildStoreLayer.parentStoreLayer = storeLayerToModify;
  } else {
    environment.store = storeLayerToModify;
  }
}

/**
 * Remove an optimistic store layer from the stack, potentially replacing it
 * with a network response.
 *
 * After we do this, we must re-execute all child startUpdate and optimistic
 * layers (since their data may have changed.) We also keep track of changed
 * records, in order to call affected subscriptions.
 */
export function revertOptimisticStoreLayerAndMaybeReplace(
  environment: IsographEnvironment,
  optimisticNode: OptimisticStoreLayer,
  normalizeData: null | ((storeLayer: StoreLayerWithData) => void),
): void {
  // We cannot just replace the optimistic node with the network response node,
  // because (e.g.) the types allow Base <- Opt, but not Base <- NetworkResponse.
  // We also may be removing the optimistic layer without replacing it with
  // anything, which would also be disallowed if the original stack was
  // Base <- Opt <- NetworkResponse.
  //
  // Thus, instead, we will (1) replace the optimistic node's data with an empty object
  // and attach the network response as a child.
  const oldMergedData = optimisticNode.data;
  optimisticNode.data = {};

  let newMergedData = {};
  let childNode = optimisticNode.childStoreLayer;
  if (normalizeData !== null) {
    const networkResponseStoreLayer: NetworkResponseStoreLayer = {
      kind: 'NetworkResponseStoreLayer',
      data: {},
      parentStoreLayer: optimisticNode,
      childStoreLayer: null,
    };
    normalizeData(networkResponseStoreLayer);

    if (childNode?.kind === 'NetworkResponseStoreLayer') {
      // (2) if the optimistic layer's child was a network response, and we are
      // replacing it with a network response, we must merge the replacement
      // and the child.
      mergeDataLayer(networkResponseStoreLayer.data, childNode.data);
      mergeDataLayer(oldMergedData, childNode.data);
      childNode = childNode.childStoreLayer;
    }
    newMergedData = structuredClone(networkResponseStoreLayer.data);
    setChildOfNode(environment, networkResponseStoreLayer, childNode);
    optimisticNode.childStoreLayer = networkResponseStoreLayer;
  }

  // (3) Re-execute all updates, accumulating all changed values into newMergedData.
  // Since we have already written the network response into newMergedData, we
  // can proceed from the child of the (potentially merged) network response layer.
  //
  // Note that it is important that reexecuteUpdatesAndMergeData is called here!
  // That is because we created newMergedData from the network response layer's data,
  // and later, we may merge that network response into the parent layer (if it is
  // a base layer). That merged layer will contain many extraneous records (unless the
  // base layer is empty).
  //
  // This would cause us to re-execute subscriptions unnecessarily, as these records
  // do not represent changes between the optimistic and network response layers.
  reexecuteUpdatesAndMergeData(childNode, oldMergedData, newMergedData);

  // (4) Now, we can finally remove the optimistic layer, i.e. do
  // optimistic.parent.child = optimistic.child.
  // But the types don't line up, so we handle the cases differently, based on the
  // parent layer type.
  if (optimisticNode.parentStoreLayer.kind === 'BaseStoreLayer') {
    // (4a) If the optimistic parent is the base layer, then we have a problem: base.child
    // must be an optimistic layer or null. So, we merge the optimistic children into the
    // base layer until we reach an optimistic layer.
    mergeLayersWithDataIntoBaseLayer(
      environment,
      optimisticNode.childStoreLayer,
      optimisticNode.parentStoreLayer,
    );
  } else if (
    optimisticNode.parentStoreLayer.kind === 'NetworkResponseStoreLayer' &&
    optimisticNode.childStoreLayer?.kind === 'NetworkResponseStoreLayer'
  ) {
    // (4b) if the parent is a network response layer, simply merge those. (We do not
    // attempt to merge other layers, e.g. startUpdate layers, because there is some
    // optimistic layer between this layer and the base, and the startUpdate will need
    // to be recalculated if the optimistic layer is reverted.)
    mergeDataLayer(
      optimisticNode.parentStoreLayer.data,
      optimisticNode.childStoreLayer.data,
    );

    setChildOfNode(
      environment,
      optimisticNode.parentStoreLayer,
      optimisticNode.childStoreLayer.childStoreLayer,
    );
  } else {
    // (4c) Otherwise, the parent is an optimistic or start update layer, and we can
    // set optimistic.parent.child = optimistic.child.
    setChildOfNode(
      environment,
      optimisticNode.parentStoreLayer,
      optimisticNode.childStoreLayer,
    );
  }

  // (5) finally, compare the oldMergedData and newMergedData objects, in order to extract
  // the modified IDs, and re-execute subscriptions.
  let encounteredIds: EncounteredIds = new Map();
  compareData(oldMergedData, newMergedData, encounteredIds);
  callSubscriptions(environment, encounteredIds);
}

export type StoreLayer =
  | OptimisticStoreLayer
  | NetworkResponseStoreLayer
  | StartUpdateStoreLayer
  | BaseStoreLayer;

export type StoreLayerWithData =
  | BaseStoreLayer
  | NetworkResponseStoreLayer
  | OptimisticNetworkResponseStoreLayer;

function compareData(
  oldData: StoreLayerData,
  newData: StoreLayerData,
  encounteredIds: EncounteredIds,
): void {
  const oldDataTypeNames = new Set(Object.keys(oldData));
  const newDataTypeNames = new Set(Object.keys(newData));

  for (const oldTypeName of oldDataTypeNames.difference(newDataTypeNames)) {
    const set = insertEmptySetIfMissing(encounteredIds, oldTypeName);
    for (const id in oldData[oldTypeName]) {
      set.add(id);
    }
  }

  for (const [typeName, newRecords] of Object.entries(newData)) {
    if (newRecords == null) {
      continue;
    }
    const oldRecords = oldData[typeName];
    outer: for (const [id, newRecord] of Object.entries(newRecords)) {
      if (newRecord == null) {
        continue;
      }
      const oldRecord = oldRecords?.[id];

      for (const [recordKey, newRecordValue] of Object.entries(newRecord)) {
        // TODO: compare links, compare arrays
        if (newRecordValue !== oldRecord?.[recordKey]) {
          const set = insertEmptySetIfMissing(encounteredIds, typeName);
          set.add(id);
          continue outer;
        }
      }

      encounteredIds.get(typeName)?.delete(id);
    }
  }
}
