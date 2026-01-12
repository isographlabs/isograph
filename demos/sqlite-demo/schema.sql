PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS film_species;
DROP TABLE IF EXISTS film_vehicles;
DROP TABLE IF EXISTS film_starships;
DROP TABLE IF EXISTS film_planets;
DROP TABLE IF EXISTS film_characters;
DROP TABLE IF EXISTS species_people;
DROP TABLE IF EXISTS vehicle_pilots;
DROP TABLE IF EXISTS starship_pilots;
DROP TABLE IF EXISTS films;
DROP TABLE IF EXISTS species;
DROP TABLE IF EXISTS vehicles;
DROP TABLE IF EXISTS starships;
DROP TABLE IF EXISTS transport;
DROP TABLE IF EXISTS people;
DROP TABLE IF EXISTS planets;

CREATE TABLE planets (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  rotation_period TEXT,
  orbital_period TEXT,
  diameter TEXT,
  climate TEXT,
  gravity TEXT,
  terrain TEXT,
  surface_water TEXT,
  population TEXT,
  created TEXT,
  edited TEXT
);

CREATE TABLE people (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  height TEXT,
  mass TEXT,
  hair_color TEXT,
  skin_color TEXT,
  eye_color TEXT,
  birth_year TEXT,
  gender TEXT,
  homeworld_id INTEGER NOT NULL,
  created TEXT,
  edited TEXT,
  FOREIGN KEY (homeworld_id) REFERENCES planets(id)
);

CREATE TABLE transport (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  model TEXT,
  manufacturer TEXT,
  cost_in_credits TEXT,
  length TEXT,
  max_atmosphering_speed TEXT,
  crew TEXT,
  passengers TEXT,
  cargo_capacity TEXT,
  consumables TEXT,
  created TEXT,
  edited TEXT
);

CREATE TABLE starships (
  id INTEGER PRIMARY KEY,
  hyperdrive_rating TEXT,
  MGLT TEXT,
  starship_class TEXT,
  FOREIGN KEY (id) REFERENCES transport(id) ON DELETE CASCADE
);

CREATE TABLE vehicles (
  id INTEGER PRIMARY KEY,
  vehicle_class TEXT,
  FOREIGN KEY (id) REFERENCES transport(id) ON DELETE CASCADE
);

CREATE TABLE species (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  classification TEXT,
  designation TEXT,
  average_height TEXT,
  skin_colors TEXT,
  hair_colors TEXT,
  eye_colors TEXT,
  average_lifespan TEXT,
  language TEXT,
  homeworld_id INTEGER,
  created TEXT,
  edited TEXT,
  FOREIGN KEY (homeworld_id) REFERENCES planets(id)
);

CREATE TABLE films (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  episode_id INTEGER,
  opening_crawl TEXT,
  director TEXT,
  producer TEXT,
  release_date TEXT,
  created TEXT,
  edited TEXT
);

CREATE TABLE film_characters (
  film_id INTEGER NOT NULL,
  people_id INTEGER NOT NULL,
  PRIMARY KEY (film_id, people_id),
  FOREIGN KEY (film_id) REFERENCES films(id) ON DELETE CASCADE,
  FOREIGN KEY (people_id) REFERENCES people(id) ON DELETE CASCADE
);

CREATE TABLE film_planets (
  film_id INTEGER NOT NULL,
  planet_id INTEGER NOT NULL,
  PRIMARY KEY (film_id, planet_id),
  FOREIGN KEY (film_id) REFERENCES films(id) ON DELETE CASCADE,
  FOREIGN KEY (planet_id) REFERENCES planets(id) ON DELETE CASCADE
);

CREATE TABLE film_starships (
  film_id INTEGER NOT NULL,
  starship_id INTEGER NOT NULL,
  PRIMARY KEY (film_id, starship_id),
  FOREIGN KEY (film_id) REFERENCES films(id) ON DELETE CASCADE,
  FOREIGN KEY (starship_id) REFERENCES starships(id) ON DELETE CASCADE
);

CREATE TABLE film_vehicles (
  film_id INTEGER NOT NULL,
  vehicle_id INTEGER NOT NULL,
  PRIMARY KEY (film_id, vehicle_id),
  FOREIGN KEY (film_id) REFERENCES films(id) ON DELETE CASCADE,
  FOREIGN KEY (vehicle_id) REFERENCES vehicles(id) ON DELETE CASCADE
);

CREATE TABLE film_species (
  film_id INTEGER NOT NULL,
  species_id INTEGER NOT NULL,
  PRIMARY KEY (film_id, species_id),
  FOREIGN KEY (film_id) REFERENCES films(id) ON DELETE CASCADE,
  FOREIGN KEY (species_id) REFERENCES species(id) ON DELETE CASCADE
);

CREATE TABLE species_people (
  species_id INTEGER NOT NULL,
  people_id INTEGER NOT NULL,
  PRIMARY KEY (species_id, people_id),
  FOREIGN KEY (species_id) REFERENCES species(id) ON DELETE CASCADE,
  FOREIGN KEY (people_id) REFERENCES people(id) ON DELETE CASCADE
);

CREATE TABLE starship_pilots (
  starship_id INTEGER NOT NULL,
  people_id INTEGER NOT NULL,
  PRIMARY KEY (starship_id, people_id),
  FOREIGN KEY (starship_id) REFERENCES starships(id) ON DELETE CASCADE,
  FOREIGN KEY (people_id) REFERENCES people(id) ON DELETE CASCADE
);

CREATE TABLE vehicle_pilots (
  vehicle_id INTEGER NOT NULL,
  people_id INTEGER NOT NULL,
  PRIMARY KEY (vehicle_id, people_id),
  FOREIGN KEY (vehicle_id) REFERENCES vehicles(id) ON DELETE CASCADE,
  FOREIGN KEY (people_id) REFERENCES people(id) ON DELETE CASCADE
);
