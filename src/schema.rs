table! {
    configuration (id) {
        id -> Integer,
        current_student -> Nullable<Integer>,
        current_project -> Nullable<Integer>,
        auto_grade -> Nullable<Integer>,
        manual_grade -> Nullable<Integer>,
        comment -> Nullable<Text>,
        base_image -> Text,
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
    }
}

table! {
    project (id) {
        id -> Integer,
        path -> Text,
        manual_grade -> Integer,
        auto_grade -> Integer,
    }
}

table! {
    student (id) {
        id -> Integer,
        path -> Text,
        graded -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    configuration,
    grade,
    project,
    student,
);
