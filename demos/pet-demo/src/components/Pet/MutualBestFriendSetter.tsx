import {
  FragmentRenderer,
  useClientSideDefer,
  useImperativeReference,
} from '@isograph/react';
import { iso } from '@iso';
import { Button, Card, CardContent } from '@mui/material';

/**
 * This is a bit of a contrived demo that shows that @loadable fields on
 * root mutations work.
 *
 * Note that we are fetching some additional, unused, fields, so that
 * the PetBestFriendCard continues to work. This is somewhat awkward!
 */

export const setMututalBestFriend = iso(`
  field Mutation.MututalBestFriendSetterMutation(
    $id: ID !
    $new_best_friend_id: ID !
  ) @component {
    set_pet_best_friend(
      id: $id
      new_best_friend_id: $new_best_friend_id
    ) {
      pet {
        id
        best_friend_relationship {
          picture_together
          best_friend {
            id
            fullName
            Avatar
          }
        }
      }
    }
    MutualBestFriendSetterOtherSide @loadable
  }
`)(({ data }) => {
  if (data.set_pet_best_friend.pet.best_friend_relationship == null) {
    throw new Error('Somehow the new best friend id was not set');
  }

  const { fragmentReference } = useClientSideDefer(
    data.MutualBestFriendSetterOtherSide,
    {
      pet_id:
        data.set_pet_best_friend.pet.best_friend_relationship?.best_friend.id,
      new_best_friend_id: data.set_pet_best_friend.pet.id,
    },
  );

  return <FragmentRenderer fragmentReference={fragmentReference} />;
});

export const SomeThing = iso(`
  field Mutation.MutualBestFriendSetterOtherSide(
    $pet_id: ID !
    $new_best_friend_id: ID !
  ) @component {
    set_pet_best_friend(
      id: $pet_id
      new_best_friend_id: $new_best_friend_id
    ) {
      pet {
        id
        fullName
        best_friend_relationship {
          best_friend {
            id
            fullName
          }
        }
      }
    }
  }
`)(({ data }) => {
  return (
    <div>
      Congrats, you set {data.set_pet_best_friend.pet.fullName} best friend to{' '}
      {
        data.set_pet_best_friend.pet.best_friend_relationship?.best_friend
          .fullName
      }{' '}
      and vice versa
    </div>
  );
});

export const MutualBestFriendSetter = iso(`
  field Pet.MutualBestFriendSetter @component {
    id
  }
`)(({ data }) => {
  const {
    fragmentReference: mutationRef,
    loadFragmentReference: loadMutation,
  } = useImperativeReference(
    iso(`entrypoint Mutation.MututalBestFriendSetterMutation`),
  );

  const cardContent =
    mutationRef == null ? (
      <Button
        onClick={() => {
          loadMutation(
            {
              id: data.id,
              new_best_friend_id: '0',
            },
            {
              shouldFetch: 'Yes',
            },
          );
        }}
        variant="contained"
      >
        Set best friend to Makayla
      </Button>
    ) : (
      <FragmentRenderer fragmentReference={mutationRef} />
    );

  return (
    <Card
      variant="outlined"
      sx={{ width: 450, boxShadow: 3, backgroundColor: '#BBB' }}
    >
      <CardContent>{cardContent}</CardContent>
    </Card>
  );
});
