import type { Query____link__output_type } from './../__link/output_type';

export type Query__NestedQuery__param = {
  readonly data: {
    readonly greeting: string | null;
    readonly query: {
      readonly tags: ReadonlyArray<(string | null)> | null;
    };
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly link: Query____link__output_type;
};
