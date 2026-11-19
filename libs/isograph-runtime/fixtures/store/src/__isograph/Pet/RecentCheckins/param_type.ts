import type { Pet____link__output_type } from "./../__link/output_type";
import type { Pet__RecentCheckins__parameters } from "./parameters_type";
export type Pet__RecentCheckins__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      readonly id: string;
      readonly location: string;
    }>;
  };
  readonly parameters: Pet__RecentCheckins__parameters;
  readonly link: Pet____link__output_type;
};
