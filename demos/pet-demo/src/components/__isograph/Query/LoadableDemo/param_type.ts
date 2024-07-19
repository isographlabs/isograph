import { type Pet__LoadableField__output_type } from '../../Pet/LoadableField/output_type';
import { type Pet__LoadableField2__output_type } from '../../Pet/LoadableField2/output_type';

export type Query__LoadableDemo__param = {
  pet: ({
    name: string,
    LoadableField: Pet__LoadableField__output_type,
    LoadableField2: Pet__LoadableField2__output_type,
  } | null),
};
