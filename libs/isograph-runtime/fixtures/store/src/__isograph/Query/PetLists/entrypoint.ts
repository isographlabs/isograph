import type { IsographEntrypoint } from '@isograph/runtime';
import type { Query__PetLists__param } from './param_type';
import type { Query__PetLists__output_type } from './output_type';
import Query__PetLists__normalization_ast from './normalization_ast';
import Query__PetLists__query_text from './query_text';
import Query__PetLists__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Query__PetLists__param,
  Query__PetLists__output_type,
  'Query'
> = {
  kind: 'Entrypoint',
  name: 'Query.PetLists',
  queryText: Query__PetLists__query_text,
  normalizationAst: Query__PetLists__normalization_ast,
  readerArtifact: () => Query__PetLists__resolver_reader(),
};
export default artifact;
