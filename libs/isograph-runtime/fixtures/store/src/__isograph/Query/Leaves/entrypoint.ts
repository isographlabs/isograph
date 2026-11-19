import type { IsographEntrypoint } from '@isograph/runtime';
import type { Query__Leaves__param } from './param_type';
import type { Query__Leaves__output_type } from './output_type';
import Query__Leaves__normalization_ast from './normalization_ast';
import Query__Leaves__query_text from './query_text';
import Query__Leaves__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Query__Leaves__param,
  Query__Leaves__output_type,
  'Query'
> = {
  kind: 'Entrypoint',
  name: 'Query.Leaves',
  queryText: Query__Leaves__query_text,
  normalizationAst: Query__Leaves__normalization_ast,
  readerArtifact: () => Query__Leaves__resolver_reader(),
};
export default artifact;
