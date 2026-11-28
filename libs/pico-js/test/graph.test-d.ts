import { expectTypeOf, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  readDerived,
  readDerivedNode,
  readSource,
  releaseRetain,
  retainDerivedNode,
  writeSource,
  type CallingNode,
  type DerivedNode,
  type Retain,
  type SourceNode,
} from '../src/index';

test('a derived node that depends on another derived node has the type of the value its computation returns', () => {
  const count = createDerivedNode(() => 1);
  const label = createDerivedNode(
    (callingNode) => `${readDerived(callingNode, count)}`,
  );
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});

test('the constructors and the tracked reads carry the value type', () => {
  const s1 = createSourceNode(1);
  expectTypeOf(s1).toEqualTypeOf<SourceNode<number>>();
  const lazy = createDerivedNode((callingNode) => readSource(callingNode, s1));
  const label = createDerivedNode(
    (callingNode) => `${readDerived(callingNode, lazy)}`,
  );
  expectTypeOf(lazy).toEqualTypeOf<DerivedNode<number>>();
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
  expectTypeOf(readDerivedNode(label)).toEqualTypeOf<string>();
  expectTypeOf<DerivedNode<number>>().toMatchTypeOf<DerivedNode<unknown>>();
  expectTypeOf<SourceNode<number>>().toMatchTypeOf<SourceNode<unknown>>();
  expectTypeOf<Retain<number>>().toMatchTypeOf<Retain<unknown>>();
});

test('retainDerivedNode returns a retain of the node type, which releaseRetain releases', () => {
  const d1 = createDerivedNode(() => 'value');
  const r1 = retainDerivedNode(d1);
  expectTypeOf(r1).toEqualTypeOf<Retain<string>>();
  expectTypeOf(retainDerivedNode(d1, () => {})).toEqualTypeOf<Retain<string>>();
  expectTypeOf(releaseRetain(r1)).toEqualTypeOf<void>();
});

test('onChange receives the value type of the node, and may ignore it', () => {
  const d1 = createDerivedNode(() => 'value');
  retainDerivedNode(d1, (value) => {
    expectTypeOf(value).toEqualTypeOf<string>();
  });
  retainDerivedNode(d1, () => {});
  // @ts-expect-error onChange of a string node does not take a number
  retainDerivedNode(d1, (value: number) => value);
});

test('createDerivedNode takes only a computation', () => {
  createDerivedNode(() => 1);
  // @ts-expect-error createDerivedNode has no second parameter
  createDerivedNode(() => 1, 0);
});

test('a computation receives its previous value as the value type or undefined', () => {
  createDerivedNode<number>((callingNode, previous) => {
    expectTypeOf(callingNode).toEqualTypeOf<CallingNode>();
    expectTypeOf(previous).toEqualTypeOf<number | undefined>();
    return 1;
  });
});

test('a write takes a value of the source type', () => {
  const s1 = createSourceNode(1);
  // @ts-expect-error a string is not a number
  writeSource(s1, 'one');
});

test('a caller cannot read or write any field of a source, a derived node, or a retain', () => {
  const s1 = createSourceNode(1);
  const d1 = createDerivedNode((callingNode) => readSource(callingNode, s1));
  const r1 = retainDerivedNode(d1);
  // @ts-expect-error not a field of a source
  s1.value;
  // @ts-expect-error not a field of a source
  s1.value = 2;
  // @ts-expect-error not a field of a source
  s1.firstDependent;
  // @ts-expect-error not a field of a derived node
  d1.value;
  // @ts-expect-error not a field of a derived node
  d1.value = 2;
  // @ts-expect-error not a field of a derived node
  d1.status;
  // @ts-expect-error not a field of a derived node
  d1.computation;
  // @ts-expect-error not a field of a retain
  r1.node;
  // @ts-expect-error not a field of a retain
  r1.onChange;
  // @ts-expect-error not a field of a retain
  r1.next = undefined;
});

test('a source, a derived node, and a retain are not interchangeable', () => {
  expectTypeOf<SourceNode<number>>().not.toMatchTypeOf<DerivedNode<number>>();
  expectTypeOf<DerivedNode<number>>().not.toMatchTypeOf<SourceNode<number>>();
  expectTypeOf<Retain<number>>().not.toMatchTypeOf<DerivedNode<number>>();
  expectTypeOf<DerivedNode<number>>().not.toMatchTypeOf<DerivedNode<string>>();
});

test('readSource and readDerived accept the callingNode of a computation and no other node', () => {
  const s1 = createSourceNode(1);
  const d1 = createDerivedNode(() => 1);
  createDerivedNode((callingNode) => {
    readSource(callingNode, s1);
    readDerived(callingNode, d1);
    return 1;
  });
  // @ts-expect-error a derived node is not a callingNode
  readSource(d1, s1);
  // @ts-expect-error a derived node is not a callingNode
  readDerived(d1, d1);
  // @ts-expect-error a source is not a callingNode
  readSource(s1, s1);
  // @ts-expect-error a source is not a callingNode
  readDerived(s1, d1);
});

test('a callingNode and a derived node are not interchangeable', () => {
  expectTypeOf<CallingNode>().not.toMatchTypeOf<DerivedNode<unknown>>();
  expectTypeOf<DerivedNode<unknown>>().not.toMatchTypeOf<CallingNode>();
  expectTypeOf<CallingNode>().not.toMatchTypeOf<SourceNode<unknown>>();
  expectTypeOf<SourceNode<unknown>>().not.toMatchTypeOf<CallingNode>();
});
