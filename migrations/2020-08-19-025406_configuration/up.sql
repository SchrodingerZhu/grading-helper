-- Your SQL goes here
CREATE TABLE configuration (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    current_student INTEGER,
    current_project INTEGER,
    auto_grade INTEGER,
    manual_grade INTEGER,
    comment VARCHAR,
    base_image VARCHAR NOT NULL
)