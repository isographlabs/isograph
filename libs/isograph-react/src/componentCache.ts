import {
  ReaderArtifact,
  RefetchQueryArtifactWrapper,
  readButDoNotEvaluate,
} from "./index";
import { DataId, stableCopy } from "./cache";

type ComponentName = string;
type StringifiedArgs = string;
const cachedComponentsById: {
  [key: DataId]: {
    [key: ComponentName]: { [key: StringifiedArgs]: React.FC<any> };
  };
} = {};
export function getOrCreateCachedComponent(
  root: DataId,
  componentName: string,
  readerArtifact: ReaderArtifact<any, any, any>,
  variables: { [key: string]: string },
  resolverRefetchQueries: RefetchQueryArtifactWrapper[]
) {
  const stringifiedArgs = JSON.stringify(stableCopy(variables));
  cachedComponentsById[root] = cachedComponentsById[root] ?? {};
  const componentsByName = cachedComponentsById[root];
  componentsByName[componentName] = componentsByName[componentName] ?? {};
  const byArgs = componentsByName[componentName];
  byArgs[stringifiedArgs] =
    byArgs[stringifiedArgs] ??
    (() => {
      function Component(additionalRuntimeProps) {
        const data = readButDoNotEvaluate({
          kind: "FragmentReference",
          readerArtifact: readerArtifact,
          root,
          variables,
          nestedRefetchQueries: resolverRefetchQueries,
        });

        return readerArtifact.resolver({
          data,
          ...additionalRuntimeProps,
        });
      }
      Component.displayName = `${componentName} (id: ${root}) @component`;
      return Component;
    })();
  return byArgs[stringifiedArgs];
}
