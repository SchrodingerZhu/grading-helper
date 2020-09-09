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

    let mut wb = excel::Workbook::create(target);
    let mut sheet = wb.create_sheet("grades");

    wb.write_sheet(&mut sheet, |sheet_writer| {
        let sw = sheet_writer;
        let mut headers = excel::Row::new();
        headers.add_cell(String::from("Student"));
        for i in projects.iter() {
            headers.add_cell(format!("Manual Grade ({})", i.name));
            headers.add_cell(format!("Auto Grade ({})", i.name));
            headers.add_cell(format!("Comment ({})", i.name));
            headers.add_cell(format!("Compile Output ({})", i.name));
            headers.add_cell(format!("Compile Stderr ({})", i.name));
            headers.add_cell(format!("Compile Return Code ({})", i.name));
            headers.add_cell(format!("Run Output ({})", i.name));
            headers.add_cell(format!("Run Stderr ({})", i.name));
            headers.add_cell(format!("Run Return Code ({})", i.name));
        }
        sw.append_row(headers).unwrap();
        for i in &students {
            let mut row = excel::Row::new();
            row.add_cell(i.path.clone());
            for j in &projects {
                let grade: QueryResult<crate::model::Grade> = g::grade
                    .filter(g::student_id
                        .eq(i.id)
                        .and(g::project_id.eq(j.id)))
                    .first::<crate::model::Grade>(conn);
                if let Ok(grade) = grade {
                    row.add_cell(grade.manual_grade.to_string());
                    row.add_cell(grade.auto_grade.to_string());
                    row.add_cell(grade.comment);
                    row.add_cell(format!("{}", grade.compile_stdout));
                    row.add_cell(format!("{}", grade.compile_stdout));
                    row.add_cell(grade.compile_return.to_string());
                    row.add_cell(format!("{}", grade.run_stdout));
                    row.add_cell(format!("{}", grade.run_stderr));
                    row.add_cell(grade.run_return.to_string());
                } else {
                    row.add_empty_cells(9);
                }
            }
            sw.append_row(row).unwrap();
        }
        Ok(())
    }).unwrap_with_log();

    wb.close().unwrap_with_log();
}
