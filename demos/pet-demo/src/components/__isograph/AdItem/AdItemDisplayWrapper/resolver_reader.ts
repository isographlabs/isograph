import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { AdItem__AdItemDisplayWrapper__param } from './param_type';
import { AdItemDisplayWrapper as resolver } from '../../../Newsfeed/AdItemDisplayWrapper';

const readerAst: ReaderAst<AdItem__AdItemDisplayWrapper__param> = [
  {
    kind: "LoadablySelectedField",
    alias: "AdItemDisplay",
    name: "AdItemDisplay",
    queryArguments: null,
    refetchReaderAst: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
    entrypoint: { 
      kind: "EntrypointLoader",
      typeAndField: "AdItem__AdItemDisplay",
      loader: () => import("../../AdItem/AdItemDisplay/entrypoint").then(module => module.default),
    },
  },
];

const artifact: ComponentReaderArtifact<
  AdItem__AdItemDisplayWrapper__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "AdItem.AdItemDisplayWrapper",
  resolver,
  readerAst,
};

export default artifact;
