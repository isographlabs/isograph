import type { Node__NodeLink__output_type } from './../../Node/NodeLink/output_type';
import type { Pet__PetLink__output_type } from './../../Pet/PetLink/output_type';
import type { PetStats__StatsLink__output_type } from './../../PetStats/StatsLink/output_type';
import type { Query____link__output_type } from './../__link/output_type';
import type { Query__Links__parameters } from './parameters_type';

export type Query__Links__param = {
  readonly data: {
    readonly pet: {
      readonly PetLink: Pet__PetLink__output_type;
      readonly stats: {
        readonly StatsLink: PetStats__StatsLink__output_type;
      } | null;
      readonly bestFriend: {
        readonly PetLink: Pet__PetLink__output_type;
      } | null;
    } | null;
    readonly node: {
      readonly NodeLink: Node__NodeLink__output_type;
    } | null;
  };
  readonly parameters: Query__Links__parameters;
  readonly link: Query____link__output_type;
};
