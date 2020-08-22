use diesel::prelude::*;

use crate::utils::*;

pub fn dump(conn: &SqliteConnection, target: &str) {
    use crate::schema::grade::dsl as g;
    let students = crate::schema::student::table
        .load::<crate::model::Student>(conn)
        .unwrap_with_log();
    let projects = crate::schema::project::table
        .load::<crate::model::Project>(conn)
        .unwrap_with_log();
    let mut csv = String::with_capacity(1024);
    csv.push_str("Student,");
    for i in projects.iter() {
        csv.extend(format!("Manual Grade ({}),", i.name).chars());
        csv.extend(format!("Auto Grade ({}),", i.name).chars());
        csv.extend(format!("Comment ({}),", i.name).chars());
        csv.extend(format!("Compile Output ({}),", i.name).chars());
        csv.extend(format!("Compile Stderr ({}),", i.name).chars());
        csv.extend(format!("Compile Return Code ({}),", i.name).chars());
        csv.extend(format!("Run Output ({}),", i.name).chars());
        csv.extend(format!("Run Stderr ({}),", i.name).chars());
        csv.extend(format!("Run Return Code ({}),", i.name).chars());
    }
    csv.push('\n');
    for i in &students {
        csv.extend(i.path.chars());
        csv.push(',');
        for j in &projects {
            let grade: QueryResult<crate::model::Grade> = g::grade
                .filter(g::student_id
                    .eq(i.id)
                    .and(g::project_id.eq(j.id)))
                .first::<crate::model::Grade>(conn);
            if let Ok(grade) = grade {
                csv.extend(grade.manual_grade.to_string().chars());
                csv.push(',');
                csv.extend(grade.auto_grade.to_string().chars());
                csv.push_str(",\"");
                csv.extend(grade.comment.escape_debug());
                csv.push_str("\",\"");
                csv.extend(grade.compile_stdout.escape_debug());
                csv.push_str("\",\"");
                csv.extend(grade.compile_stderr.escape_debug());
                csv.push_str("\",");
                csv.extend(grade.compile_return.to_string().chars());
                csv.push_str(",\"");
                csv.extend(grade.run_stdout.escape_debug());
                csv.push_str("\",\"");
                csv.extend(grade.run_stderr.escape_debug());
                csv.push_str("\",");
                csv.extend(grade.run_return.to_string().chars());
                csv.push(',');
            } else {
                csv.push_str(",,,,,,,,,");
            }
        }
        csv.push('\n');
    }
    std::fs::write(target, csv)
        .unwrap_with_log();
}