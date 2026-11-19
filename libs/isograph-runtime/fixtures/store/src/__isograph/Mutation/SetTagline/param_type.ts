import type { Mutation__SetTagline__parameters } from "./parameters_type";
export type Mutation__SetTagline__param = {
  readonly data: {
    readonly setTagline: {
      readonly pet: {
        readonly id: string;
        readonly tagline: string;
      };
      readonly query: {
        readonly greeting: (string | null);
      };
    };
  };
  readonly parameters: Mutation__SetTagline__parameters;
};
