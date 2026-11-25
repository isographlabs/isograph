import type { DerivedNode, Status } from '../src/nodes';
import { internal } from './internal';

/** The engine's status of `node`, which the public `DerivedNode` type does not expose. */
export function statusOf(node: DerivedNode<unknown>): Status {
  return internal(node).status;
}
