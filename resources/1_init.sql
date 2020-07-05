CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL DEFAULT uuid_generate_v4(),
    username varchar(40) NOT NULL
);
