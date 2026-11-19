import type { ReaderArtifact } from '@isograph/runtime';
import type { Pet__owner__output_type } from './output_type';
import type { Pet__owner__param } from './param_type';
import { owner as resolver } from './../../../Owner';

const readerArtifact = (): ReaderArtifact<
  Pet__owner__param,
  Pet__owner__output_type,
  'Pet'
> => ({
  readerAst: [
    {
      kind: 'Scalar',
      alias: 'id',
      selectableName: 'id',
      arguments: null,
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
