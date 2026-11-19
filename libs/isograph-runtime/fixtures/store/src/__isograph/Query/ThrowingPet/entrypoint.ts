import type { IsographEntrypoint } from '@isograph/runtime';
import type { Query__ThrowingPet__param } from './param_type';
import type { Query__ThrowingPet__output_type } from './output_type';
import Query__ThrowingPet__normalization_ast from './normalization_ast';
import Query__ThrowingPet__query_text from './query_text';
import Query__ThrowingPet__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Query__ThrowingPet__param,
  Query__ThrowingPet__output_type,
  'Query'
> = {
  kind: 'Entrypoint',
  name: 'Query.ThrowingPet',
  queryText: Query__ThrowingPet__query_text,
  normalizationAst: Query__ThrowingPet__normalization_ast,
  readerArtifact: () => Query__ThrowingPet__resolver_reader(),
};
export default artifact;
