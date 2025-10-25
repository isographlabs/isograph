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
        let node: StoreNode | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.get(storeRecord, p, storeRecord);
            if (value !== undefined) {
              return value;
            }
          }
          node = node.parentNode;
        }
      },
      has(_, p) {
        let node: StoreNode | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.has(storeRecord, p);
            if (value) {
              return true;
            }
          }
          node = node.parentNode;
        }
        return false;
      },
    },
  );
}

type BaseNode = {
  readonly kind: 'BaseNode';
  childNode: OptimisticNode | null;
  parentNode: null;
  readonly data: IsographStore;
};

type NetworkResponseNode = {
  readonly kind: 'NetworkResponseNode';
  childNode: OptimisticNode | StartUpdateNode | null;
  parentNode: OptimisticNode | StartUpdateNode;
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
  childNode: OptimisticNode | NetworkResponseNode | null;
  parentNode: OptimisticNode | NetworkResponseNode;
  data: DataLayer;
  startUpdate: DataUpdate;
};

type OptimisticNode = {
  readonly kind: 'OptimisticNode';
  childNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | null;
  parentNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | BaseNode;
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
        parentNode: environment.store,
        childNode: null,
        data,
      };
      environment.store.childNode = node;
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
        parentNode: environment.store,
        childNode: null,
        data,
        startUpdate: startUpdate,
      };
      environment.store.childNode = node;
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
        parentNode: environment.store,
        childNode: null,
        data,
        startUpdate: startUpdate,
      };

      environment.store.childNode = node;
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
    node = node.childNode;
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
    node.parentNode = environment.store;
    environment.store.childNode = node;
    environment.store = node;

    node = node.childNode;

    environment.store.childNode = null;
  }
}

function makeRootNode(environment: IsographEnvironment, node: StoreNode) {
  node.childNode = null;
  environment.store = node;
}

function replaceOptimisticNodeWithNetworkResponseNode(
  environment: IsographEnvironment,
  optimisticNode: OptimisticNode,
  data: DataLayer,
  encounteredIds: EncounteredIds,
) {
  if (optimisticNode.parentNode.kind === 'BaseNode') {
    mergeDataLayer(optimisticNode.parentNode.data, data);

    makeRootNode(environment, optimisticNode.parentNode);
    const node = mergeParentNodes(
      environment,
      optimisticNode.childNode,
      encounteredIds,
    );
    reexecuteUpdates(environment, node, encounteredIds);
  } else if (optimisticNode.parentNode.kind === 'NetworkResponseNode') {
    mergeDataLayer(optimisticNode.parentNode.data, data);

    makeRootNode(environment, optimisticNode.parentNode);
    reexecuteUpdates(environment, optimisticNode.childNode, encounteredIds);
  } else if (optimisticNode.childNode?.kind === 'NetworkResponseNode') {
    const networkResponseNode = optimisticNode.childNode;
    mergeDataLayer(data, networkResponseNode.data);
    networkResponseNode.data = data;

    networkResponseNode.parentNode = optimisticNode.parentNode;
    optimisticNode.parentNode.childNode = networkResponseNode;

    const childNode = optimisticNode.childNode.childNode;
    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, childNode, encounteredIds);
  } else {
    const networkResponseNode: NetworkResponseNode = {
      kind: 'NetworkResponseNode',
      data,
      parentNode: optimisticNode.parentNode,
      childNode: null,
    };

    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, optimisticNode.childNode, encounteredIds);
  }
}

export type StoreNode =
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
