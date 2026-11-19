import type { Query____link__output_type } from "./../__link/output_type";
import type { Query__Leaves__parameters } from "./parameters_type";
export type Query__Leaves__param = {
  readonly data: {
    readonly greeting: (string | null);
    readonly literalGreeting: (string | null);
    readonly nullGreeting: (string | null);
    readonly variableGreeting: (string | null);
    readonly tags: (ReadonlyArray<(string | null)> | null);
    readonly settings: (unknown | null);
  };
  readonly parameters: Query__Leaves__parameters;
  readonly link: Query____link__output_type;
};
