CREATE TABLE titles (
    id INTEGER PRIMARY KEY NOT NULL,
    type INTEGER NOT NULL,
    primary_title TEXT NOT NULL,
    original_title TEXT,
    is_adult INTEGER NOT NULL,
    start_year INTEGER,
    end_year INTEGER,
    runtime_minutes INTEGER,
    genres TEXT
) STRICT;

CREATE TABLE ratings (
    id INTEGER PRIMARY KEY NOT NULL,
    average_rating REAL NOT NULL,
    num_votes INTEGER NOT NULL,

    FOREIGN KEY (id) REFERENCES titles(id)
) STRICT;

CREATE TABLE akas (
    id INTEGER NOT NULL,
    ordering INTEGER NOT NULL,
    title TEXT NOT NULL,
    region TEXT,
    language TEXT,
    types TEXT,
    attributes TEXT,
    is_original_title INTEGER NOT NULL,

    PRIMARY KEY (id, ordering),
    FOREIGN KEY (id) REFERENCES titles(id)
) STRICT;

CREATE TABLE episodes (
    id INTEGER PRIMARY KEY NOT NULL,
    parent_id INTEGER NOT NULL,
    season_number INTEGER NOT NULL,
    episode_number INTEGER NOT NULL,

    FOREIGN KEY (id) REFERENCES titles(id),
    FOREIGN KEY (parent_id) REFERENCES titles(id)
) STRICT;