import { OnlyOneRootLoadable } from '@/src/components/Pet/PetWithOneRootLoadable';
import { iso } from '@iso';
import { useLazyReference } from '@isograph/react';

export default function OnlyOneRootLoadablePage() {
  const router = useRouter();

  // During SSR, id will be nullish. So, we just render the shell.
  // This isn't ideal, and we should figure out how to fix that!
  const id = router.query.id;
  if (id == null || Array.isArray(id)) {
    return;
  }

  return (
    <OnlyOneRootLoadable route={{ kind: 'OnlyOneRootLoadableRoute', id: id }} />
  );
}
