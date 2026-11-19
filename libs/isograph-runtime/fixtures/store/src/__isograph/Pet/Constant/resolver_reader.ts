import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__Constant__output_type } from "./output_type";
import type { Pet__Constant__param } from "./param_type";
import { Constant as resolver } from "./../../../Constant";
const readerArtifact = (): ReaderArtifact<Pet__Constant__param, Pet__Constant__output_type, "Pet"> =>({
    readerAst: [],
    resolver: resolver
  });
export default readerArtifact;
