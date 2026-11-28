import type {
  CallingNode,
  DerivedNode,
  MutableDerivedNode,
  MutableRetain,
  MutableSourceNode,
  Retain,
  SourceNode,
} from '../src/nodes';

export function internal<TValue>(
  node: DerivedNode<TValue>,
): MutableDerivedNode<TValue>;
export function internal<TValue>(
  node: SourceNode<TValue>,
): MutableSourceNode<TValue>;
export function internal(
  node: DerivedNode<unknown> | SourceNode<unknown>,
): MutableDerivedNode<unknown> | MutableSourceNode<unknown>;
export function internal<TValue>(retain: Retain<TValue>): MutableRetain<TValue>;
export function internal(callingNode: CallingNode): MutableDerivedNode<unknown>;
export function internal(
  node:
    | DerivedNode<unknown>
    | SourceNode<unknown>
    | Retain<unknown>
    | CallingNode,
): unknown {
  return node;
}
