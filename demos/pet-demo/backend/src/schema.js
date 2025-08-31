import fs from 'fs';
import path from 'path';
import { createSchema } from 'graphql-yoga';
import {
  getAdItem,
  getBlogItem,
  getImage,
  getNewsfeedItems,
  newsfeedResolvers,
} from './newsfeed.js';

const schemaContents = fs
  .readFileSync(path.resolve(import.meta.dirname, '../schema.graphql'))
  .toString();

const picturesTogether = [
  [
    null,
    'http://localhost:3000/makayla_2.jpg',
    'http://localhost:3000/makayla_3.jpg',
  ],
];

const checkins = [
  {
    __typename: 'Checkin',
    id: 'Checkin__0',
    location: 'Couch',
    pet_id: '0',
    time: '4:10pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__1',
    location: 'Food bowl',
    pet_id: '0',
    time: '10:30am',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__2',
    location: 'Doggie bed',
    pet_id: '0',
    time: '8:00am',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__3',
    location: 'Taco Bell',
    pet_id: '1',
    time: 'Many years ago',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__4',
    location: 'Crevice between couches',
    pet_id: '2',
    time: 'Every day',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__5',
    location: 'Hallway',
    pet_id: '3',
    time: '3:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__6',
    location: 'Dog park',
    pet_id: '3',
    time: '1:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__7',
    location: 'The sunshine',
    pet_id: '0',
    time: '1:30pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__8',
    location: 'Under the covers',
    pet_id: '0',
    time: '3:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__9',
    location: 'Dreamland',
    pet_id: '0',
    time: '9:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__10',
    location: 'Water bowl',
    pet_id: '0',
    time: '7:00am',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__11',
    location: 'On her pillow',
    pet_id: '0',
    time: '6:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__12',
    location: 'By the window',
    pet_id: '0',
    time: '4:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__13',
    location: 'Vet',
    pet_id: '0',
    time: '2:30pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__14',
    location: 'Chelsea dog park',
    pet_id: '0',
    time: '4:00pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__15',
    location: 'Under the benches',
    pet_id: '0',
    time: '4:15pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__16',
    location: 'On the grass',
    pet_id: '0',
    time: '4:20pm',
  },
  {
    __typename: 'Checkin',
    id: 'Checkin__17',
    location: 'Wandering free',
    pet_id: '0',
    time: '5:00pm',
  },
];

export const schema = createSchema({
  typeDefs: schemaContents,
  resolvers: {
    Query: {
      pet: (_obj, args) => getPet(args.id),
      petByName: (_obj, args) =>
        pets.find(
          (pet) =>
            pet.name.split(' ')[0].toLowerCase() === args.name.toLowerCase(),
        ),
      pets: () => pets,
      node: (_obj, args) => {
        if (args.id === 'Viewer') {
          return {
            __typename: 'Viewer',
            id: 'Viewer',
          };
        }
        return (
          getPet(args.id) ??
          getBlogItem(args.id) ??
          getAdItem(args.id) ??
          getImage(args.id)
        );
      },
      viewer: () => {
        return {
          __typename: 'Viewer',
          id: 'Viewer',
        };
      },
      topLevelField: () => null,
    },
    Viewer: {
      newsfeed: (_obj, args) => {
        return getNewsfeedItems(args.skip, args.limit);
      },
    },
    ...newsfeedResolvers,
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
      checkins: (pet, args) => {
        const allCheckins = checkins.filter(
          (checkin) => checkin.pet_id === pet.id,
        );
        const limit = args?.limit ?? Infinity;
        const skip = args?.skip ?? 0;
        return allCheckins.slice(skip, skip + limit);
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
      make_checkin_super: (_obj, params) => {
        const checkin = getCheckin(params.checkin_id);
        if (checkin != null) {
          checkin.location = 'Super ' + checkin.location;
          return { id: params.checkin_id };
        } else {
          return { id: null };
        }
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
    MakeCheckinSuperResponse: {
      icheckin: (object) => getCheckin(object.id),
    },
  },
});

const pets = [
  {
    __typename: 'Pet',
    id: '0',
    name: 'Makayla Balicka',
    nickname: 'Princess',
    picture: 'http://localhost:3000/makayla.jpg',
    best_friend_relationship: {
      best_friend: '1',
      picture_together: 'http://localhost:3000/makayla_2.jpg',
    },
    age: 16,
    tagline: 'The OG',
    alt_tagline: 'The cute one',
    favorite_phrase: "Don't bother me!",
    stats: {
      weight: 18,
      intelligence: 100,
      cuteness: 50,
      hunger: 67,
      sociability: 4,
      energy: 10,
    },
  },
  {
    __typename: 'Pet',
    id: '1',
    name: 'Mimi Balicka',
    nickname: 'Mimcia',
    picture: 'http://localhost:3000/mimi.jpg',
    age: 21,
    tagline: 'The lost one',
    favorite_phrase: null,
    stats: {
      weight: 10,
      intelligence: 90,
      cuteness: 40,
      hunger: 30,
      sociability: 14,
      energy: 12,
    },
  },
  {
    __typename: 'Pet',
    id: '2',
    name: 'Henry Balicki',
    nickname: 'Booboo',
    picture: 'http://localhost:3000/henry.jpg',
    age: 7,
    tagline: 'The lazy one',
    favorite_phrase: 'It would be too much effort to utter a phrase.',
    stats: {
      weight: 45,
      intelligence: 30,
      cuteness: 35,
      hunger: 100,
      sociability: 8,
      energy: 4,
    },
  },
  {
    __typename: 'Pet',
    id: '3',
    name: 'Tiberius Balicki',
    nickname: null,
    picture: 'http://localhost:3000/tiberius.jpg',
    age: 3,
    tagline: 'The golden child',
    favorite_phrase: "I'll get that lazer pointer, you just watch!",
    stats: {
      weight: 18,
      intelligence: 95,
      cuteness: 55,
      hunger: 30,
      sociability: 22,
      energy: 45,
    },
  },
  {
    __typename: 'Pet',
    id: '4',
    name: 'Kiki Balicka',
    nickname: null,
    picture: 'http://localhost:3000/kiki.jpg',
    age: 8,
    tagline: 'The troublemaker',
    favorite_phrase: null,
  },
  {
    __typename: 'Pet',
    id: '5',
    name: 'Rezor Balicki',
    nickname: null,
    picture: 'http://localhost:3000/rezor.jpg',
    age: 12,
    tagline: 'The defender',
    favorite_phrase: null,
  },
];

function getPet(id) {
  return pets[Number(id)];
}

function getCheckin(id) {
  return checkins.find((checkin) => checkin.id === id);
}
