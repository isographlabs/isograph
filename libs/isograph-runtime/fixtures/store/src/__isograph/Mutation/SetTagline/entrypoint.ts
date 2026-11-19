import type { IsographEntrypoint } from '@isograph/runtime';
import type { Mutation__SetTagline__param } from './param_type';
import type { Mutation__SetTagline__output_type } from './output_type';
import Mutation__SetTagline__normalization_ast from './normalization_ast';
import Mutation__SetTagline__query_text from './query_text';
import Mutation__SetTagline__resolver_reader from './resolver_reader';

const artifact: IsographEntrypoint<
  Mutation__SetTagline__param,
  Mutation__SetTagline__output_type,
  'Mutation'
> = {
  kind: 'Entrypoint',
  name: 'Mutation.SetTagline',
  queryText: Mutation__SetTagline__query_text,
  normalizationAst: Mutation__SetTagline__normalization_ast,
  readerArtifact: () => Mutation__SetTagline__resolver_reader(),
};
export default artifact;
