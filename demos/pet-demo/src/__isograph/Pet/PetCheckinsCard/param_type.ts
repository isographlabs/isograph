import type { Pet__PetCheckinsCard__parameters } from "./parameters_type";
export type Pet__PetCheckinsCard__param = {
  readonly data: {
    /**
The pet's ID
*/ readonly id: string;
    /**
Even pets use social media, mmkay.
*/ readonly checkins: ReadonlyArray<{
      readonly id: string;
      readonly location: string;
      readonly time: string;
    }>;
  };
  readonly parameters: Pet__PetCheckinsCard__parameters;
};
