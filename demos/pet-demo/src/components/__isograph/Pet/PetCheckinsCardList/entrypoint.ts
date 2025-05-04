import type {IsographEntrypoint, NormalizationAstLoader, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Pet__PetCheckinsCardList__param} from './param_type';
import {Pet__PetCheckinsCardList__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
// import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const artifact: IsographEntrypoint<
  Pet__PetCheckinsCardList__param,
  Pet__PetCheckinsCardList__output_type,
  NormalizationAstLoader
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst: { kind: "NormalizationAstLoader", loader: () => import('./normalization_ast').then(x => x.default) },
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
