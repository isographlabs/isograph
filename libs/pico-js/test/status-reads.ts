import type { DerivedNode, Status } from '../src/nodes';
import { internal } from './internal';

/**
 * Replaces the `status` field of each node with an accessor that counts how
 * often the engine reads it, and returns a function that gives the count.
 */
export function countStatusReads(
  nodes: ReadonlyArray<DerivedNode<unknown>>,
): () => number {
  let reads = 0;
  nodes.forEach((node) => {
    const mutableNode = internal(node);
    let status: Status = mutableNode.status;
    Object.defineProperty(mutableNode, 'status', {
      get: () => {
        reads += 1;
        return status;
      },
      set: (value: Status) => {
        status = value;
      },
    });
  });
  return () => reads;
}
