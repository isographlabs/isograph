import type { Query__Feed__parameters } from "./parameters_type";
export type Query__Feed__param = {
  readonly data: {
    readonly feed: ReadonlyArray<{
      readonly asBlogItem: ({
        readonly id: string;
        readonly title: string;
        readonly author: ({
          readonly id: string;
          readonly name: string;
        } | null);
      } | null);
      readonly asAdItem: ({
        readonly advertiser: string;
      } | null);
    }>;
  };
  readonly parameters: Query__Feed__parameters;
};
