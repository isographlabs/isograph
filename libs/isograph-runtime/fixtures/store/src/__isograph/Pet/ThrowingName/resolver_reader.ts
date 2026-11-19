import type { ReaderArtifact } from '@isograph/runtime';
import type { Pet__ThrowingName__output_type } from './output_type';
import type { Pet__ThrowingName__param } from './param_type';
import { ThrowingName as resolver } from './../../../ThrowingName';

const readerArtifact = (): ReaderArtifact<
  Pet__ThrowingName__param,
  Pet__ThrowingName__output_type,
  'Pet'
> => ({
  readerAst: [
    {
      kind: 'Scalar',
      alias: 'name',
      selectableName: 'name',
      arguments: null,
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
