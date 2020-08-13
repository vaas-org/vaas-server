CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username varchar(40) NOT NULL
);

CREATE TABLE IF NOT EXISTS issues (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title text NOT NULL,
    description text NOT NULL,
    state text NOT NULL,
    max_voters integer NOT NULL,
    show_distribution boolean NOT NULL
);

CREATE TABLE IF NOT EXISTS alternatives (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    issue_id UUID references issues(id) NOT NULL,
    title text NOT NULL,
    UNIQUE (id, issue_id)
);

CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID references users(id) NOT NULL
);

CREATE TABLE IF NOT EXISTS votes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID references users(id) NOT NULL,
    issue_id UUID references issues(id) NOT NULL,
    alternative_id UUID NOT NULL,
    FOREIGN KEY (alternative_id, issue_id) REFERENCES alternatives(id, issue_id),
    UNIQUE (user_id, issue_id)
);
