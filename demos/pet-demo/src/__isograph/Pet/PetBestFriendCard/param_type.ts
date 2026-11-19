import type { Pet____link__output_type } from './../__link/output_type';
import type { Pet__fullName__output_type } from './../fullName/output_type';

export type Pet__PetBestFriendCard__param = {
  readonly data: {
    /**
     * The pet's ID
     */
    readonly id: string;
    /**
     * Pets have a very complex social life. Find out more!
     */
    readonly best_friend_relationship: {
      readonly picture_together: unknown | null;
      readonly best_friend: {
        /**
         * The pet's ID
         */
        readonly id: string;
        /**
         * What do you call the pet when you're happy?
         */
        readonly friendName: string;
        /**
         * The pet's first and last name, for example Makayla Balicki.
         */
        readonly fullName: Pet__fullName__output_type;
      };
    } | null;
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly link: Pet____link__output_type;
};
