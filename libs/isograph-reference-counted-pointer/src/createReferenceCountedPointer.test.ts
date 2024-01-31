import { describe, test, vi, expect, assert } from 'vitest';
import { createReferenceCountedPointer } from './createReferenceCountedPointer';

describe('createReferenceCountedPointer', () => {
  describe('it should not dispose the underlying item until all outstanding pointers are disposed', () => {
    test('one pointer', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointer();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('linked list, FIFO', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointer2, disposePointer2] = pointer.cloneIfNotDisposed()!;
      const [pointer3, disposePointer3] = pointer2.cloneIfNotDisposed()!;
      const [pointer4, disposePointer4] = pointer3.cloneIfNotDisposed()!;

      disposePointer4();
      disposePointer3();
      disposePointer2();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointer();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('linked list, LIFO', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointer2, disposePointer2] = pointer.cloneIfNotDisposed()!;
      const [pointer3, disposePointer3] = pointer2.cloneIfNotDisposed()!;
      const [pointer4, disposePointer4] = pointer3.cloneIfNotDisposed()!;

      disposePointer();
      disposePointer2();
      disposePointer3();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointer4();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('linked list, mixed order', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointer2, disposePointer2] = pointer.cloneIfNotDisposed()!;
      const [pointer3, disposePointer3] = pointer2.cloneIfNotDisposed()!;
      const [pointer4, disposePointer4] = pointer3.cloneIfNotDisposed()!;

      disposePointer2();
      disposePointer();
      disposePointer4();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointer3();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('DAG, from root', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointerA, disposePointerA] = pointer.cloneIfNotDisposed()!;
      const [pointerA_1, disposePointerA_1] = pointerA.cloneIfNotDisposed()!;
      const [pointerA_2, disposePointerA_2] = pointerA.cloneIfNotDisposed()!;
      const [pointerB, disposePointerB] = pointer.cloneIfNotDisposed()!;
      const [pointerB_1, disposePointerB_1] = pointerB.cloneIfNotDisposed()!;
      const [pointerB_2, disposePointerB_2] = pointerB.cloneIfNotDisposed()!;

      disposePointer();
      disposePointerA();
      disposePointerA_1();
      disposePointerA_2();
      disposePointerB();
      disposePointerB_1();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointerB_2();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('DAG, from leaves', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointerA, disposePointerA] = pointer.cloneIfNotDisposed()!;
      const [pointerA_1, disposePointerA_1] = pointerA.cloneIfNotDisposed()!;
      const [pointerA_2, disposePointerA_2] = pointerA.cloneIfNotDisposed()!;
      const [pointerB, disposePointerB] = pointer.cloneIfNotDisposed()!;
      const [pointerB_1, disposePointerB_1] = pointerB.cloneIfNotDisposed()!;
      const [pointerB_2, disposePointerB_2] = pointerB.cloneIfNotDisposed()!;

      disposePointerB_1();
      disposePointerB_2();
      disposePointerB();
      disposePointerA_1();
      disposePointerA_2();
      disposePointerA();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointer();
      expect(disposeItem).toHaveBeenCalled();
    });

    test('DAG, random', () => {
      const disposeItem = vi.fn();
      const [pointer, disposePointer] = createReferenceCountedPointer([
        1,
        disposeItem,
      ]);
      const [pointerA, disposePointerA] = pointer.cloneIfNotDisposed()!;
      const [pointerA_1, disposePointerA_1] = pointerA.cloneIfNotDisposed()!;
      const [pointerA_2, disposePointerA_2] = pointerA.cloneIfNotDisposed()!;
      const [pointerB, disposePointerB] = pointer.cloneIfNotDisposed()!;
      const [pointerB_1, disposePointerB_1] = pointerB.cloneIfNotDisposed()!;
      const [pointerB_2, disposePointerB_2] = pointerB.cloneIfNotDisposed()!;

      disposePointerB_1();
      disposePointerA();
      disposePointer();
      disposePointerB_2();
      disposePointerB();
      disposePointerA_2();
      expect(disposeItem).not.toHaveBeenCalled();
      disposePointerA_1();
      expect(disposeItem).toHaveBeenCalled();
    });
  });

  test('it should throw when disposed twice', () => {
    const disposeItem = vi.fn();
    const [pointer, disposePointer] = createReferenceCountedPointer([
      1,
      disposeItem,
    ]);
    disposePointer();
    expect(() => {
      disposePointer();
    }).toThrow();
  });

  test('it should return null when you attempt to retain a disposed pointer', () => {
    const disposeItem = vi.fn();
    const [pointer, disposePointer] = createReferenceCountedPointer([
      1,
      disposeItem,
    ]);
    disposePointer();
    expect(pointer.cloneIfNotDisposed()).toBe(null);
  });

  test('it should expose the underlying object only when undisposed', () => {
    const disposeItem = vi.fn();
    const [pointer, disposePointer] = createReferenceCountedPointer([
      1,
      disposeItem,
    ]);
    expect(pointer.getItemIfNotDisposed()).toBe(1);
    disposePointer();
    expect(pointer.getItemIfNotDisposed()).toBe(null);
  });

  test('it should accurately report its disposed status', () => {
    const disposeItem = vi.fn();
    const [pointer, disposePointer] = createReferenceCountedPointer([
      1,
      disposeItem,
    ]);
    expect(pointer.isDisposed()).toBe(false);
    disposePointer();
    expect(pointer.isDisposed()).toBe(true);
  });

  test('disposable status is unaffected by the presence of other undisposed pointers', () => {
    const disposeItem = vi.fn();
    const [pointer, disposePointer] = createReferenceCountedPointer([
      1,
      disposeItem,
    ]);
    const pointer2 = pointer.cloneIfNotDisposed();
    assert(pointer2 != null);
    expect(pointer2[0].isDisposed()).toBe(false);
    expect(pointer.isDisposed()).toBe(false);
    disposePointer();
    expect(pointer.isDisposed()).toBe(true);
    expect(pointer2[0].isDisposed()).toBe(false);
  });
});
