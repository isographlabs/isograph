import { createSchema } from "graphql-yoga";
import fs from "fs";

const schemaContents = fs.readFileSync("./schema.graphql").toString();

const picturesTogether = [
  [
    null,
    "http://localhost:3000/makayla_2.jpg",
    "http://localhost:3000/makayla_3.jpg",
  ],
];

const checkins = [
  {
    id: "0",
    location: "Couch",
    pet_id: "0",
    time: "4:10pm",
  },
  {
    id: "1",
    location: "Food bowl",
    pet_id: "0",
    time: "10:30am",
  },
  {
    id: "2",
    location: "Doggie bed",
    pet_id: "0",
    time: "8:00am",
  },
  {
    id: "3",
    location: "Taco Bell",
    pet_id: "1",
    time: "Many years ago",
  },
  {
    id: "4",
    location: "Crevice between couches",
    pet_id: "2",
    time: "Every day",
  },
  {
    id: "5",
    location: "Hallway",
    pet_id: "3",
    time: "3:00pm",
  },
  {
    id: "6",
    location: "Dog park",
    pet_id: "3",
    time: "1:00pm",
  },
];

export const schema = createSchema({
  // LOL
  typeDefs:
    "input FieldMap { from: String!, to: String! } \n" +
    "directive @primary(path: String!, field_map: [FieldMap!]!) repeatable on OBJECT \n " +
    schemaContents,
  resolvers: {
    Query: {
      pet: (obj, args) => getPet(args.id),
      pets: () => pets,
    },
    Pet: {
      stats: (pet) => {
        return pet.stats;
      },
      best_friend_relationship: (pet) => {
        return pet.best_friend_relationship;
      },
      potential_new_best_friends: (pet) =>
        pets.filter((otherPet) => {
          return (
            otherPet.id !== pet.id &&
            otherPet.id !== pet.best_friend_relationship?.best_friend
          );
        }),
      checkins: (pet) => {
        return checkins.filter((checkin) => checkin.pet_id === pet.id);
      },
    },
    BestFriendRelationship: {
      best_friend: (relationship) => {
        return getPet(relationship.best_friend);
      },
    },
    Mutation: {
      set_pet_best_friend: (_obj, params) => {
        const modifiedPet = pets[params.id];
        const min =
          params.id < params.new_best_friend_id
            ? params.id
            : params.new_best_friend_id;
        const max =
          params.id < params.new_best_friend_id
            ? params.new_best_friend_id
            : params.id;

        modifiedPet.best_friend_relationship = {
          best_friend: params.new_best_friend_id,
          picture_together: (picturesTogether[min] ?? [])[max],
        };
        return {
          id: params.id,
        };
      },
      set_pet_tagline: (_obj, params) => {
        const modifiedPet = pets[params.input.id];
        modifiedPet.tagline = params.input.tagline;
        return {
          id: params.input.id,
        };
      },
    },
    SetBestFriendResponse: {
      pet: (object) => {
        return getPet(object.id);
      },
    },
    SetPetTaglineResponse: {
      pet: (object) => {
        return getPet(object.id);
      },
    },
  },
});

const pets = [
  {
    id: "0",
    name: "Makayla Balicka",
    nickname: "Princess",
    picture: "http://localhost:3000/makayla.jpg",
    best_friend_relationship: {
      best_friend: "1",
      picture_together: "http://localhost:3000/makayla_2.jpg",
    },
    age: 16,
    tagline: "The OG",
    favorite_phrase: "Don't bother me!",
  },
  {
    id: "1",
    name: "Mimi Balicka",
    nickname: "Mimcia",
    picture: "http://localhost:3000/mimi.jpg",
    age: 21,
    tagline: "The lost one",
    favorite_phrase: null,
  },
  {
    id: "2",
    name: "Henry Balicki",
    nickname: "Booboo",
    picture: "http://localhost:3000/henry.jpg",
    age: 7,
    tagline: "The lazy one",
    favorite_phrase: "It would be too much effort to utter a phrase.",
  },
  {
    id: "3",
    name: "Tiberius Balicki",
    nickname: null,
    picture: "http://localhost:3000/tiberius.jpg",
    age: 3,
    tagline: "The golden child",
    favorite_phrase: "I'll get that lazer pointer, you just watch!",
  },
  {
    id: "4",
    name: "Kiki Balicka",
    nickname: null,
    picture: "http://localhost:3000/kiki.jpg",
    age: 8,
    tagline: "The troublemaker",
    favorite_phrase: null,
  },
  {
    id: "5",
    name: "Rezor Balicki",
    nickname: null,
    picture: "http://localhost:3000/rezor.jpg",
    age: 12,
    tagline: "The defender",
    favorite_phrase: null,
  },
];

function getPet(id) {
  return pets[Number(id)];
}
