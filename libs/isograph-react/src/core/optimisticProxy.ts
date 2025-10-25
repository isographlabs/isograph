import {
  callSubscriptions,
  insertEmptySetIfMissing,
  type EncounteredIds,
} from './cache';
import type {
  DataLayer,
  IsographEnvironment,
  IsographStore,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';
import { logMessage } from './logging';

export function getOrInsertRecord(dataLayer: DataLayer, link: StoreLink) {
  const recordsById = (dataLayer[link.__typename] ??= {});
  return (recordsById[link.__link] ??= {});
}

export function readOptimisticRecord(
  environment: IsographEnvironment,
  link: StoreLink,
) {
  return new Proxy<StoreRecord>(
    {},
    {
      get(_, p) {
        let node: OptimisticLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.get(storeRecord, p, storeRecord);
            if (value !== undefined) {
              return value;
            }
          }
          node = node.childNode;
        }
      },
      has(_, p) {
        let node: OptimisticLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.has(storeRecord, p);
            if (value) {
              return true;
            }
          }
          node = node.childNode;
        }
        return false;
      },
    },
  );
}

type BaseNode = {
  readonly kind: 'BaseNode';
  parentNode: OptimisticNode | null;
  childNode: null;
  readonly data: IsographStore;
};

type NetworkResponseNode = {
  readonly kind: 'NetworkResponseNode';
  parentNode: OptimisticNode | StartUpdateNode | null;
  childNode: OptimisticNode | StartUpdateNode;
  data: DataLayer;
};

export type WithEncounteredIds<T> = {
  readonly encounteredIds: EncounteredIds;
  readonly data: T;
};

type FirstUpdate = () => WithEncounteredIds<DataLayer>;
type DataUpdate = () => Pick<WithEncounteredIds<DataLayer>, 'data'>;

type StartUpdateNode = {
  readonly kind: 'StartUpdateNode';
  parentNode: OptimisticNode | NetworkResponseNode | null;
  childNode: OptimisticNode | NetworkResponseNode;
  data: DataLayer;
  startUpdate: DataUpdate;
};

type OptimisticNode = {
  readonly kind: 'OptimisticNode';
  parentNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | null;
  childNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | BaseNode;
  data: DataLayer;
  startUpdate: DataUpdate;
};

export function addNetworkResponseNode(
  environment: IsographEnvironment,
  data: DataLayer,
  encounteredIds: EncounteredIds,
) {
  switch (environment.store.kind) {
    case 'NetworkResponseNode':
    case 'BaseNode': {
      mergeDataLayer(environment.store.data, data);
      break;
    }
    case 'StartUpdateNode':
    case 'OptimisticNode': {
      const node: NetworkResponseNode = {
        kind: 'NetworkResponseNode',
        childNode: environment.store,
        parentNode: null,
        data,
      };
      environment.store.parentNode = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }

  logMessage(environment, () => ({
    kind: 'AfterNormalization',
    store: environment.store,
    encounteredIds: encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);
}

function mergeDataLayer(target: DataLayer, source: DataLayer): void {
  for (const typeName in source) {
    target[typeName] ??= {};
    for (const id in source[typeName]) {
      target[typeName][id] ??= {};
      Object.assign(target[typeName][id], source[typeName][id]);
    }
  }
}

export function addStartUpdateNode(
  environment: IsographEnvironment,
  startUpdate: FirstUpdate,
) {
  const { data, encounteredIds } = startUpdate();

  switch (environment.store.kind) {
    case 'BaseNode': {
      mergeDataLayer(environment.store.data, data);
      break;
    }
    case 'StartUpdateNode': {
      const prevStartUpdate = environment.store.startUpdate;

      mergeDataLayer(environment.store.data, data);

      environment.store.startUpdate = () => {
        const { data } = prevStartUpdate();
        mergeDataLayer(data, startUpdate().data);
        return { data };
      };

      break;
    }
    case 'NetworkResponseNode':
    case 'OptimisticNode': {
      const node: StartUpdateNode = {
        kind: 'StartUpdateNode',
        childNode: environment.store,
        parentNode: null,
        data,
        startUpdate: startUpdate,
      };
      environment.store.parentNode = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
    }
  }

  logMessage(environment, () => ({
    kind: 'StartUpdateComplete',
    updatedIds: encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);
}

export function addOptimisticNode(
  environment: IsographEnvironment,
  startUpdate: FirstUpdate,
) {
  const { data, encounteredIds } = startUpdate();

  switch (environment.store.kind) {
    case 'BaseNode':
    case 'StartUpdateNode':
    case 'NetworkResponseNode':
    case 'OptimisticNode': {
      const node: OptimisticNode = {
        kind: 'OptimisticNode',
        childNode: environment.store,
        parentNode: null,
        data,
        startUpdate: startUpdate,
      };

      environment.store.parentNode = node;
      environment.store = node;

      callSubscriptions(environment, encounteredIds);
      return (data: DataLayer) => {
        const encounteredIds: EncounteredIds = new Map();
        compareData(node.data, data, encounteredIds);
        replaceOptimisticNodeWithNetworkResponseNode(
          environment,
          node,
          data,
          encounteredIds,
        );
        callSubscriptions(environment, encounteredIds);
      };
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }
}

function mergeParentNodes(
  environment: IsographEnvironment,
  node: OptimisticNode | NetworkResponseNode | StartUpdateNode | null,
  mutableEncounteredIds: EncounteredIds,
) {
  while (node && node?.kind !== 'OptimisticNode') {
    const data = 'startUpdate' in node ? node.startUpdate().data : node.data;
    compareData(node.data, data, mutableEncounteredIds);
    mergeDataLayer(environment.store.data, data);
    node = node.parentNode;
  }
  return node;
}

function reexecuteUpdates(
  environment: IsographEnvironment,
  node: OptimisticNode | NetworkResponseNode | StartUpdateNode | null,
  mutableEncounteredIds: EncounteredIds,
) {
  while (node !== null) {
    const oldData = node.data;
    if ('startUpdate' in node) {
      node.data = node.startUpdate().data;
    }
    compareData(oldData, node.data, mutableEncounteredIds);
    node.childNode = environment.store;
    environment.store.parentNode = node;
    environment.store = node;

    node = node.parentNode;

    environment.store.parentNode = null;
  }
}

function makeRootNode(environment: IsographEnvironment, node: OptimisticLayer) {
  node.parentNode = null;
  environment.store = node;
}

function replaceOptimisticNodeWithNetworkResponseNode(
  environment: IsographEnvironment,
  optimisticNode: OptimisticNode,
  data: DataLayer,
  encounteredIds: EncounteredIds,
) {
  if (optimisticNode.childNode.kind === 'BaseNode') {
    mergeDataLayer(optimisticNode.childNode.data, data);

    makeRootNode(environment, optimisticNode.childNode);
    const node = mergeParentNodes(
      environment,
      optimisticNode.parentNode,
      encounteredIds,
    );
    reexecuteUpdates(environment, node, encounteredIds);
  } else if (optimisticNode.childNode.kind === 'NetworkResponseNode') {
    mergeDataLayer(optimisticNode.childNode.data, data);

    makeRootNode(environment, optimisticNode.childNode);
    reexecuteUpdates(environment, optimisticNode.parentNode, encounteredIds);
  } else if (optimisticNode.parentNode?.kind === 'NetworkResponseNode') {
    const networkResponseNode = optimisticNode.parentNode;
    mergeDataLayer(data, networkResponseNode.data);
    networkResponseNode.data = data;

    networkResponseNode.childNode = optimisticNode.childNode;
    optimisticNode.childNode.parentNode = networkResponseNode;

    const parentNode = optimisticNode.parentNode.parentNode;
    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, parentNode, encounteredIds);
  } else {
    const networkResponseNode: NetworkResponseNode = {
      kind: 'NetworkResponseNode',
      data,
      childNode: optimisticNode.childNode,
      parentNode: null,
    };

    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, optimisticNode.parentNode, encounteredIds);
  }
}

export type OptimisticLayer =
  | OptimisticNode
  | NetworkResponseNode
  | StartUpdateNode
  | BaseNode;

function compareData(
  oldData: DataLayer,
  newData: DataLayer,
  encounteredIds: EncounteredIds,
) {
  if (oldData === newData) {
    for (const [typeName, ids] of encounteredIds.entries()) {
      for (const id of ids) {
        encounteredIds.get(typeName)?.delete(id);
      }
    }
    return;
  }
  for (const [typeName, records] of Object.entries(newData)) {
    if (!records) {
      continue;
    }
    outer: for (const [id, record] of Object.entries(records)) {
      if (!record) {
        continue;
      }

      for (const [recordKey, recordValue] of Object.entries(record)) {
        // TODO: compare links, compare arrays
        if (recordValue !== oldData[typeName]?.[id]?.[recordKey]) {
          const set = insertEmptySetIfMissing(encounteredIds, typeName);
          set.add(id);
          continue outer;
        }
      }

      encounteredIds.get(typeName)?.delete(id);
    }
  }
}
