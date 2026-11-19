import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__PetDetailRoute__output_type } from "./output_type";
import type { Query__PetDetailRoute__param } from "./param_type";
import { PetDetailRoute as resolver } from "./../../../PetDetailRoute";
import Pet__PetBestFriendCard__resolver_reader from "./../../Pet/PetBestFriendCard/resolver_reader";
import Pet__PetCheckinsCard__resolver_reader from "./../../Pet/PetCheckinsCard/resolver_reader";
import Pet__PetStatsCard__resolver_reader from "./../../Pet/PetStatsCard/resolver_reader";
import Pet__fullName__resolver_reader from "./../../Pet/fullName/resolver_reader";
const readerArtifact: ReaderArtifact<Query__PetDetailRoute__param, Query__PetDetailRoute__output_type, "Query"> = {
  readerAst: [
    {
      kind: "Linked",
      alias: "pet",
      selectableName: "pet",
      arguments: [
        [
          "id",
          {
            kind: "Variable",
            name: "id"
          }
        ]
      ],
      selections: [
        {
          kind: "Scalar",
          alias: "id",
          selectableName: "id",
          arguments: null
        },
        {
          kind: "Resolver",
          alias: "fullName",
          arguments: null,
          readerArtifact: Pet__fullName__resolver_reader
        },
        {
          kind: "Resolver",
          alias: "PetStatsCard",
          arguments: null,
          readerArtifact: Pet__PetStatsCard__resolver_reader
        },
        {
          kind: "Resolver",
          alias: "PetCheckinsCard",
          arguments: [
            [
              "limit",
              {
                kind: "Literal",
                value: 5
              }
            ]
          ],
          readerArtifact: Pet__PetCheckinsCard__resolver_reader
        },
        {
          kind: "Resolver",
          alias: "PetBestFriendCard",
          arguments: null,
          readerArtifact: Pet__PetBestFriendCard__resolver_reader
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
