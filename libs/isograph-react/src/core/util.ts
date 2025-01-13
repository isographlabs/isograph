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
    ? JSX.IntrinsicAttributes
    : T & JSX.IntrinsicAttributes;

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
    };
