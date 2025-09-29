import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Mutation__TestLazyReference__param } from './param_type';
import { Mutation__TestLazyReference__output_type } from './output_type';
import { SomeThing as resolver } from '../../../Pet/PetTaglineCard2';

const readerAst: ReaderAst<Mutation__TestLazyReference__param> = [
  {
    kind: "Scalar",
    fieldName: "expose_field_on_mutation",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: EagerReaderArtifact<
  Mutation__TestLazyReference__param,
  Mutation__TestLazyReference__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Mutation.TestLazyReference",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
