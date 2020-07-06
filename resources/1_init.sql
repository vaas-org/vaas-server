CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4(),
    username varchar(40) NOT NULL
);

CREATE TABLE issues (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title text NOT NULL,
    description text NOT NULL,
    state text NOT NULL,
    max_voters integer NOT NULL,
    show_distribution boolean NOT NULL
);

CREATE TABLE alternatives (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    issue_id UUID references issues(id),
    title text NOT NULL
);
