import { useRouter } from 'next/router';

export type PetId = string;

export type Route = HomeRoute | PetDetailRoute | PetDetailDeferredRoute;

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
  }
}

export function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}
