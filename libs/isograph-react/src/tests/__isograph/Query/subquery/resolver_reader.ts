import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__subquery__param } from './param_type';
import { Query__subquery__output_type } from './output_type';
import { subquery as resolver } from '../../../normalizeData.test';
import Query____refetch__refetch_reader from '../../Query/__refetch/refetch_reader';

const readerAst: ReaderAst<Query__subquery__param> = [
  {
    kind: "ImperativelyLoadedField",
    alias: "__refetch",
    refetchReaderArtifact: Query____refetch__refetch_reader,
    refetchQueryIndex: 0,
    name: "__refetch",
  },
];

const artifact = (): EagerReaderArtifact<
  Query__subquery__param,
  Query__subquery__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "subquery",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
