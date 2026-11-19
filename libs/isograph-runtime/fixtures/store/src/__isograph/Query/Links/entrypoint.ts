import type { IsographEntrypoint } from '@isograph/runtime';
import type { Query__Links__param } from './param_type';
import type { Query__Links__output_type } from './output_type';
import Query__Links__normalization_ast from './normalization_ast';
import Query__Links__query_text from './query_text';
import Query__Links__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Query__Links__param,
  Query__Links__output_type,
  'Query'
> = {
  kind: 'Entrypoint',
  name: 'Query.Links',
  queryText: Query__Links__query_text,
  normalizationAst: Query__Links__normalization_ast,
  readerArtifact: () => Query__Links__resolver_reader(),
};
export default artifact;
