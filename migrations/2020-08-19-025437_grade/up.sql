-- Your SQL goes here
CREATE TABLE grade (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    student_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    manual_grade INTEGER NOT NULL DEFAULT 0,
    auto_grade INTEGER NOT NULL DEFAULT 0,
    comment VARCHAR NOT NULL DEFAULT '',
    compile_stdout VARCHAR NOT NULL DEFAULT '',
    compile_stderr VARCHAR NOT NULL DEFAULT '',
    compile_return INTEGER NOT NULL DEFAULT 0,
    run_stdout VARCHAR NOT NULL DEFAULT '',
    run_stderr VARCHAR NOT NULL DEFAULT '',
    run_return INTEGER NOT NULL DEFAULT 0
)