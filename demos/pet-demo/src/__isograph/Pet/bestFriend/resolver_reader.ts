import type { ReaderArtifact } from '@isograph/runtime';
import type { Pet__bestFriend__output_type } from './output_type';
import type { Pet__bestFriend__param } from './param_type';
import { bestFriend as resolver } from './../../../Pet';

const readerArtifact = (): ReaderArtifact<
  Pet__bestFriend__param,
  Pet__bestFriend__output_type,
  'Pet'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'best_friend_relationship',
      selectableName: 'best_friend_relationship',
      arguments: null,
      selections: [
        {
          kind: 'Linked',
          alias: 'best_friend',
          selectableName: 'best_friend',
          arguments: null,
          selections: [
            {
              kind: 'Scalar',
              alias: 'id',
              selectableName: 'id',
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
