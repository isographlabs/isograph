import {
  FragmentRenderer,
  useClientSideDefer,
  useImperativeReference,
} from '@isograph/react';
import { iso } from '../__isograph/iso';
import { Button } from '@mui/material';

export const setMututalBestFriend = iso(`
  field Mutation.MututalBestFriendSetterMutation(
    $id: ID !,
    $new_best_friend_id: ID !
  ) @component {
    set_pet_best_friend(
      id: $id,
      new_best_friend_id: $new_best_friend_id
    ) {
      pet {
        id
        best_friend_relationship {
          best_friend {
            id
          }
        }
      }
    }
    MutualBestFriendSetterOtherSide @loadable
  }
`)(({ data }) => {
  if (!data.set_pet_best_friend.pet.best_friend_relationship?.best_friend.id) {
    throw new Error('Somehow the new best friend id was not set');
  }

  const { fragmentReference } = useClientSideDefer(
    data.MutualBestFriendSetterOtherSide,
    {
      id: data.set_pet_best_friend.pet.best_friend_relationship?.best_friend.id,
      new_best_friend_id: data.set_pet_best_friend.pet.id,
    },
  );

  return <FragmentRenderer fragmentReference={fragmentReference} />;
});

export const SomeThing = iso(`
  field Mutation.MutualBestFriendSetterOtherSide(
    $id: ID !,
    $new_best_friend_id: ID !
  ) @component {
    set_pet_best_friend(
      id: $id,
      new_best_friend_id: $new_best_friend_id
    ) {
      pet {
        id
        name
        best_friend_relationship {
          best_friend {
            id
            name
          }
        }
      }
    }
  }
`)(({ data }) => {
  return (
    <div>
      Congrats, you set {data.set_pet_best_friend.pet.name} best friend to{' '}
      {data.set_pet_best_friend.pet.best_friend_relationship?.best_friend.name}{' '}
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

  if (!mutationRef) {
    return (
      <Button
        onClick={() => {
          loadMutation({
            id: data.id,
            new_best_friend_id: '0',
          });
        }}
        variant="contained"
      >
        Set best friend to Makayla
      </Button>
    );
  } else {
    return <FragmentRenderer fragmentReference={mutationRef} />;
  }
});
