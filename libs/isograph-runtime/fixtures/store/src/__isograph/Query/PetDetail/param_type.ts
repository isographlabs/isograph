import type { Query__PetDetail__parameters } from "./parameters_type";
export type Query__PetDetail__param = {
  readonly data: {
    readonly pet: ({
      readonly id: string;
      readonly name: string;
      readonly nickname: (string | null);
      readonly tagline: string;
      readonly stats: ({
        readonly weight: (number | null);
        readonly energy: (number | null);
      } | null);
      readonly bestFriend: ({
        readonly id: string;
        readonly name: string;
        readonly bestFriend: ({
          readonly id: string;
          readonly name: string;
        } | null);
      } | null);
      readonly checkins: ReadonlyArray<{
        readonly id: string;
        readonly location: string;
      }>;
      readonly pagedCheckins: ReadonlyArray<{
        readonly id: string;
        readonly location: string;
      }>;
    } | null);
  };
  readonly parameters: Query__PetDetail__parameters;
};
