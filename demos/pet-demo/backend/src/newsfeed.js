/**
 * A news feed that populates itself when you ask for items.
 */

import { LoremIpsum } from 'lorem-ipsum';

const lorem = new LoremIpsum({
  sentencesPerParagraph: {
    max: 8,
    min: 4,
  },
  wordsPerSentence: {
    max: 16,
    min: 4,
  },
});

const firstBlog = {
  __typename: 'BlogItem',
  id: 'blog_0',
  author: 'Relatoris',
  title: 'Declaratio amoris',
  content:
    'Relator se sponso suo satis dementer declarat. Pulchra est. \
Saltat bene. Habet mirabilem saporem in domibus. \n\
Ac per hoc se felicissimum esse existimat. \
Pergit facere meliorem hominem, et sperat unum diem quibusdam modis ei aequare.\n\
Id abhorret a physicis mundi.',
  moreContent: 'Is that not enough for you?',
};
const newsfeedItems = [firstBlog];
const blogItems = [firstBlog];
const adItems = [];
const images = [
  {
    id: 'image_0',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/a/ae/Iglesia_de_San_Carlos_Borromeo%2C_Viena%2C_Austria%2C_2020-01-31%2C_DD_164-166_HDR.jpg',
  },
  {
    id: 'image_1',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/c/c9/Scuol-Motta_Naluns%2C_15-09-2023._%28actm.%29_09.jpg',
  },
  {
    id: 'image_2',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/0/03/Aktie_Canal_de_Panama_1880.jpg',
  },
  {
    id: 'image_3',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/8/8d/U.S._Air_Force_at_Pittsburgh_air_show.jpg',
  },
  {
    id: 'image_4',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/1/11/Strokkur%2C_%C3%81rea_geot%C3%A9rmica_de_Geysir%2C_Su%C3%B0urland%2C_Islandia%2C_2014-08-16%2C_DD_088.JPG',
  },
  {
    id: 'image_5',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/a/a0/Catedral_de_Santa_Mar%C3%ADa%2C_Sig%C3%BCenza%2C_Espa%C3%B1a%2C_2015-12-28%2C_DD_118-120_HDR.JPG',
  },
  {
    id: 'image_6',
    __typename: 'Image',
    url: 'https://upload.wikimedia.org/wikipedia/commons/d/d9/La_pirogue_%C3%A0_balancier.jpg',
  },
];

function getRandomInt(max) {
  return Math.floor(Math.random() * max);
}

export const getNewsfeedItems = (skip, limit) => {
  fillNewsfeedItems(limit + skip);
  return newsfeedItems.slice(skip, limit + skip);
};

function fillNewsfeedItems(limit) {
  for (let i = newsfeedItems.length; i < limit; i++) {
    if (i % 10 == 9) {
      const adItem = {
        __typename: 'AdItem',
        id: `ad_${adItems.length}`,
        advertiser: 'GraphQL Co',
        message: lorem.generateWords(5),
      };
      // insert an ad
      adItems.push(adItem);
      newsfeedItems.push(adItem);
    } else {
      const image =
        blogItems.length > 15 && Math.random() > 0.7
          ? images[getRandomInt(images.length)]
          : null;
      const blogItem = {
        __typename: 'BlogItem',
        id: `blog_${blogItems.length}`,
        author: lorem.generateWords(2),
        title: lorem.generateWords(5),
        content: lorem.generateParagraphs(2),
        moreContent: lorem.generateParagraphs(4),
        image,
      };
      // insert a blog
      blogItems.push(blogItem);
      newsfeedItems.push(blogItem);
    }
  }
}

export const getBlogItem = (id) =>
  blogItems.find((blogItem) => blogItem.id === id);
export const getAdItem = (id) => adItems.find((adItem) => adItem.id === id);
export const getImage = (id) => images.find((image) => image.id === id);

export const newsfeedResolvers = {
  NewsfeedItem: {
    __resolveType: (obj) => {
      return obj.__typename;
    },
  },
  BlogItem: {
    image: (obj) => obj.image,
  },
};
