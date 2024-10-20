import { iso } from '@iso';
import { Fragment } from 'react';

export const HomePage = iso(`
  field Query.HomePage @component {
    # Gets the first 150 pokemon, take is higher because there are alternative forms
    # returned and the offset skips a bunch of other Pokemon that aren't in the first 150
    getAllPokemon(take: 232, offset: 93) {
      key
      forme
      Pokemon
    }
  }
`)(function HomePageComponent({ data }) {
  return (
    <div
      style={{
        display: 'flex',
        gap: '10px',
        flexWrap: 'wrap',
        backgroundColor: '#4a5b91',
        color: '#000000',
        padding: '10px',
      }}
    >
      {data.getAllPokemon
        ?.filter(({ forme }) => !forme)
        .map((pokemon) => (
          <Fragment key={pokemon.key}>
            <pokemon.Pokemon />
          </Fragment>
        ))}
    </div>
  );
});
