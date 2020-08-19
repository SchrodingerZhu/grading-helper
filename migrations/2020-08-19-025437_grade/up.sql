-- Your SQL goes here
CREATE TABLE grade (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    student_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    manual_grade INTEGER NOT NULL,
    auto_grade INTEGER NOT NULL,
    comment VARCHAR NOT NULL DEFAULT ''
)