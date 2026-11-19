import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__PetBestFriendCard__output_type } from "./output_type";
import type { Pet__PetBestFriendCard__param } from "./param_type";
import { PetBestFriendCard as resolver } from "./../../../PetBestFriendCard";
import Pet__fullName__resolver_reader from "./../fullName/resolver_reader";
const readerArtifact = (): ReaderArtifact<Pet__PetBestFriendCard__param, Pet__PetBestFriendCard__output_type, "Pet"> =>({
    readerAst: [
      {
        kind: "Scalar",
        alias: "id",
        selectableName: "id",
        arguments: null
      },
      {
        kind: "Linked",
        alias: "best_friend_relationship",
        selectableName: "best_friend_relationship",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            alias: "picture_together",
            selectableName: "picture_together",
            arguments: null
          },
          {
            kind: "Linked",
            alias: "best_friend",
            selectableName: "best_friend",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                alias: "id",
                selectableName: "id",
                arguments: null
              },
              {
                kind: "Scalar",
                alias: "friendName",
                selectableName: "firstName",
                arguments: null
              },
              {
                kind: "Resolver",
                alias: "fullName",
                arguments: null,
                readerArtifact: Pet__fullName__resolver_reader
              }
            ]
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
