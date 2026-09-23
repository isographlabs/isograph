import type React from 'react';

export type ExtractSecondParam<
  T extends (props: {
    firstParameter: any;
    additionalRuntimeProps: any;
  }) => any,
> = T extends (props: {
  firstParameter: any;
  additionalRuntimeProps: infer P;
}) => any
  ? P
  : never;
export type CombineWithIntrinsicAttributes<T> =
  T extends Record<PropertyKey, never>
    ? React.JSX.IntrinsicAttributes
    : T & React.JSX.IntrinsicAttributes;

export type Arguments = Argument[];
export type Argument = [ArgumentName, ArgumentValue];
export type ArgumentName = string;
export type ArgumentValue =
  | {
      readonly kind: 'Variable';
      readonly name: string;
    }
  | {
      readonly kind: 'Literal';
      readonly value: any;
    }
  | {
      readonly kind: 'String';
      readonly value: string;
    }
  | {
      readonly kind: 'Enum';
      readonly value: string;
    }
  | {
      readonly kind: 'Object';
      readonly value: Arguments;
    };

export function isArray(value: unknown): value is readonly unknown[] {
  return Array.isArray(value);
}

/**
 * Creates a copy of the provided value, ensuring any nested objects have their
 * keys sorted such that equivalent values would have identical JSON.stringify
 * results.
 */
export function stableCopy<T>(value: T): T {
  if (value == null || typeof value !== 'object') {
    return value;
  }
  if (isArray(value)) {
    // @ts-ignore
    return value.map(stableCopy);
  }
  const keys = Object.keys(value).sort();
  const stable: { [index: string]: any } = {};
  for (let i = 0; i < keys.length; i++) {
    // @ts-ignore
    stable[keys[i]] = stableCopy(value[keys[i]]);
  }
  return stable as any;
}
