import type { Pet__Constant__output_type } from "./../../Pet/Constant/output_type";
import type { Query____link__output_type } from "./../__link/output_type";
import type { Query__ConstantPet__parameters } from "./parameters_type";
export type Query__ConstantPet__param = {
  readonly data: {
    readonly pet: ({
      readonly Constant: Pet__Constant__output_type;
    } | null);
  };
  readonly parameters: Query__ConstantPet__parameters;
  readonly link: Query____link__output_type;
};
