import { useRouter } from 'next/router';

export type PetId = string;

export type Route =
  | HomeRoute
  | PetDetailRoute
  | PetDetailDeferredRoute
  | PetByNameRoute
  | PetCheckinListRoute
  | SmartestRoute;

export type HomeRoute = {
  kind: 'Home';
};

export type PetDetailRoute = {
  kind: 'PetDetail';
  id: PetId;
};

export type PetDetailDeferredRoute = {
  kind: 'PetDetailDeferred';
  id: PetId;
};

export type PetByNameRoute = {
  kind: 'PetByName';
  name: string;
};

export type PetCheckinListRoute = {
  kind: 'PetCheckinList';
  id: PetId;
};

export type SmartestRoute = {
  kind: 'Smartest';
};

export function useNavigateTo() {
  const router = useRouter();
  return (route: Route) => router.push(toRouteUrl(route));
}

function toRouteUrl(route: Route): string {
  switch (route.kind) {
    case 'Home': {
      return '/';
    }
    case 'PetDetail': {
      return `/pet/${route.id}`;
    }
    case 'PetDetailDeferred': {
      return `/pet/with-defer/${route.id}`;
    }
    case 'PetByName': {
      return `/pet/by-name/${route.name}`;
    }
    case 'PetCheckinList': {
      return `/pet/${route.id}/checkin-list`;
    }
    case 'Smartest': {
      return '/smartest';
    }
    default: {
      let _: never = route;
      throw new Error('Unhandled route');
    }
  }
}

export function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
