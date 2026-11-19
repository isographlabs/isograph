import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__PetLink__output_type } from "./output_type";
import type { Pet__PetLink__param } from "./param_type";
import { PetLink as resolver } from "./../../../PetLink";
const readerArtifact = (): ReaderArtifact<Pet__PetLink__param, Pet__PetLink__output_type, "Pet"> =>({
    readerAst: [],
    resolver: resolver
  });
export default readerArtifact;
