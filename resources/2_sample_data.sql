INSERT INTO users (username) VALUES('user');
INSERT INTO users (username) VALUES('admin');
INSERT INTO issues
    (id, title, description, state, max_voters, show_distribution)
    VALUES ('2a38614a-fd5b-4d4c-826d-809a656db2ea', 'coronvorus bad??', 'yes or yes', 'in_progress', 10, true);
INSERT INTO alternatives
    (title, issue_id)
    VALUES ('yes', '2a38614a-fd5b-4d4c-826d-809a656db2ea');
INSERT INTO alternatives
    (title, issue_id)
    VALUES ('other yes', '2a38614a-fd5b-4d4c-826d-809a656db2ea');
INSERT INTO alternatives
    (title, issue_id)
    VALUES ('my name is trump i have control', '2a38614a-fd5b-4d4c-826d-809a656db2ea');
