import type { Query__Search__parameters } from "./parameters_type";
export type Query__Search__param = {
  readonly data: {
    readonly search: ReadonlyArray<{
      readonly id: string;
      readonly name: string;
    }>;
  };
  readonly parameters: Query__Search__parameters;
};
