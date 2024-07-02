import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__LoadableDemo__param } from './param_type';
import { Query__LoadableDemo__output_type } from './output_type';
import { Bar as resolver } from '../../../Loadable';
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
        fieldName: "tagline",
        alias: null,
        arguments: null,
      },
      {
        kind: "ImperativelyLoadedField",
        alias: "LoadableField",
        readerArtifact: Pet__LoadableField__refetch_reader,
        refetchQuery: 0,
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__LoadableDemo__param,
  Query__LoadableDemo__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
