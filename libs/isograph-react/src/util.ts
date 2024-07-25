export type ExtractSecondParam<T extends (arg1: any, arg2: any) => any> =
  T extends (arg1: any, arg2: infer P) => any ? P : never;

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
