import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__LoadableDemo__param } from './param_type';
import { Bar as resolver } from '../../../LoadableDemo';
import Pet__LoadableField__resolver_reader from '../../Pet/LoadableField/resolver_reader';
import Pet__LoadableField__refetch_reader from '../../Pet/LoadableField/refetch_reader';

const readerAst: ReaderAst<Query__LoadableDemo__param> = [
  {
    kind: "Linked",
    fieldName: "pet",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Literal", value: 0 },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "ImperativelyLoadedField",
        alias: "LoadableField",
        refetchReaderArtifact: Pet__LoadableField__refetch_reader,
        resolverReaderArtifact: Pet__LoadableField__resolver_reader,
        refetchQuery: 0,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__LoadableDemo__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.LoadableDemo",
  resolver,
  readerAst,
};

export default artifact;
