table! {
    configuration (id) {
        id -> Integer,
        current_student -> Nullable<Integer>,
        current_project -> Nullable<Integer>,
        auto_grade -> Nullable<Integer>,
        manual_grade -> Nullable<Integer>,
        comment -> Nullable<Text>,
        base_image -> Text,
        compile_stdout -> Nullable<Text>,
        compile_stderr -> Nullable<Text>,
        compile_return -> Nullable<Integer>,
        run_stdout -> Nullable<Text>,
        run_stderr -> Nullable<Text>,
        run_return -> Nullable<Integer>,
    }
}

table! {
    grade (id) {
        id -> Integer,
        student_id -> Integer,
        project_id -> Integer,
        manual_grade -> Integer,
        auto_grade -> Integer,
        comment -> Text,
        compile_stdout -> Text,
        compile_stderr -> Text,
        compile_return -> Integer,
        run_stdout -> Text,
        run_stderr -> Text,
        run_return -> Integer,
    }
}

table! {
    project (id) {
        id -> Integer,
        path -> Text,
        name -> Text,
    }
}

table! {
    student (id) {
        id -> Integer,
        path -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    configuration,
    grade,
    project,
    student,
);
