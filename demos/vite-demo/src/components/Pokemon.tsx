import { iso } from '@iso';

export const Pokemon = iso(`
  field Pokemon.Pokemon @component { 
    num
    species
    sprite
    bulbapediaPage
  }
`)(function PokemonComponent({ data: pokemon }) {
  // Only render if you're not a alternative form
  return (
    <div
      style={{
        padding: '10px',
        backgroundColor: '#ffffff',
        width: '200px',
        height: '200px',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
        }}
      >
        <span
          style={{
            backgroundColor: '#000000',
            border: '3px solid black',
            borderRadius: '50%',
            color: '#ffffff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '30px',
            height: '30px',
            textAlign: 'center',
          }}
        >
          {pokemon.num}
        </span>
        <span>{pokemon.species.toLocaleUpperCase()}</span>
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        <img
          src={pokemon.sprite}
          alt={`${pokemon.num}`}
          style={{ maxWidth: '100px' }}
        />
      </div>
      <a
        href={pokemon.bulbapediaPage}
        target="_blank"
        style={{
          backgroundColor: '#4a5b91',
          color: '#ffffff',
          textAlign: 'center',
          borderRadius: '5px',
          padding: '5px',
        }}
      >
        Bulbapedia
      </a>
    </div>
  );
});
