import type { ReaderArtifact } from '@isograph/runtime';
import type { Pet__PetName__output_type } from './output_type';
import type { Pet__PetName__param } from './param_type';
import { PetName as resolver } from './../../../PetName';

const readerArtifact = (): ReaderArtifact<
  Pet__PetName__param,
  Pet__PetName__output_type,
  'Pet'
> => ({
  readerAst: [
    {
      kind: 'Scalar',
      alias: 'name',
      selectableName: 'name',
      arguments: null,
    },
    {
      kind: 'Scalar',
      alias: 'nickname',
      selectableName: 'nickname',
      arguments: null,
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
