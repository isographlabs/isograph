export const HomeRoute = require("@isograph/react").hmr(function HomeRouteComponent({ data }) {
    const { fragmentReference, loadFragmentReference } = useImperativeReference(require("./__isograph/Query/PetFavoritePhrase/entrypoint.ts").default);
    return "Render";
});
