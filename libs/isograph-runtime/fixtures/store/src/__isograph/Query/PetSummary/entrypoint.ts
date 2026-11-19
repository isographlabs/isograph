import type { IsographEntrypoint } from '@isograph/runtime';
import type { Query__PetSummary__param } from './param_type';
import type { Query__PetSummary__output_type } from './output_type';
import Query__PetSummary__normalization_ast from './normalization_ast';
import Query__PetSummary__query_text from './query_text';
import Query__PetSummary__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Query__PetSummary__param,
  Query__PetSummary__output_type,
  'Query'
> = {
  kind: 'Entrypoint',
  name: 'Query.PetSummary',
  queryText: Query__PetSummary__query_text,
  normalizationAst: Query__PetSummary__normalization_ast,
  readerArtifact: () => Query__PetSummary__resolver_reader(),
};
export default artifact;
