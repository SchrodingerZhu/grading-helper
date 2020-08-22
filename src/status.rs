use diesel::prelude::*;
use prettytable::{Cell, Table};
use structopt as opt;

use crate::model;
use crate::schema;
use crate::utils::*;

#[derive(opt::StructOpt, Debug)]
pub enum StatusCommand {
    #[structopt(about = "Check configuration and current project")]
    Current,
    #[structopt(about = "List all project template(s)")]
    Projects,
    #[structopt(about = "List all students")]
    Students,
    #[structopt(about = "List grading result")]
    Grading,
    #[structopt(about = "List grades")]
    Grades {
        #[structopt(short, long, help = "Filter by student id")]
        student_id: Option<i32>,
        #[structopt(short, long, help = "Filter by project id")]
        project_id: Option<i32>,
    },
}

pub fn handle(subcommand: &StatusCommand, conn: &SqliteConnection) {
    match subcommand {
        StatusCommand::Current => {
            model::Configuration::get_global(conn)
                .map(|x| {
                    use prettytable::*;
                    let mut table = Table::new();
                    table.add_row(Row::new(vec![Cell::new("current_student"),
                                                Cell::new(&x.current_student.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.add_row(Row::new(vec![Cell::new("current project"),
                                                Cell::new(&x.current_project.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.add_row(Row::new(vec![Cell::new("auto_grade"),
                                                Cell::new(&x.auto_grade.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.add_row(Row::new(vec![Cell::new("manual_grade"),
                                                Cell::new(&x.manual_grade.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.add_row(Row::new(vec![Cell::new("comment"),
                                                Cell::new(x.comment.as_ref().map(AsRef::as_ref)
                                                    .unwrap_or(""))]));
                    table.add_row(Row::new(vec![Cell::new("base_image"),
                                                Cell::new(x.base_image.as_ref())]));
                    table.add_row(Row::new(vec![Cell::new("compile_stdout"),
                                                Cell::new(x.compile_stdout.as_ref().map(AsRef::as_ref)
                                                    .unwrap_or(""))]));
                    table.add_row(Row::new(vec![Cell::new("compile_stderr"),
                                                Cell::new(x.compile_stderr.as_ref().map(AsRef::as_ref)
                                                    .unwrap_or(""))]));
                    table.add_row(Row::new(vec![Cell::new("compile_return"),
                                                Cell::new(&x.compile_return.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.add_row(Row::new(vec![Cell::new("run_stdout"),
                                                Cell::new(x.run_stdout.as_ref().map(AsRef::as_ref)
                                                    .unwrap_or(""))]));
                    table.add_row(Row::new(vec![Cell::new("run_stderr"),
                                                Cell::new(x.run_stderr.as_ref().map(AsRef::as_ref)
                                                    .unwrap_or(""))]));
                    table.add_row(Row::new(vec![Cell::new("run_return"),
                                                Cell::new(&x.run_return.as_ref().map(|x| x.to_string())
                                                    .unwrap_or_else(String::new))]));
                    table.printstd();
                })
                .unwrap_with_log();
        }
        StatusCommand::Projects => {
            let projects = schema::project::table
                .load::<model::Project>(conn)
                .unwrap_with_log();
            println!("{}", tablefy::into_string(&projects));
        }
        StatusCommand::Students => {
            let students = schema::student::table
                .load::<model::Student>(conn)
                .unwrap_with_log();
            println!("{}", tablefy::into_string(&students));
        }
        StatusCommand::Grading => {
            let students = schema::student::table
                .load::<model::Student>(conn)
                .unwrap_with_log();
            let projects = schema::project::table
                .load::<model::Project>(conn)
                .unwrap_with_log();
            let mut table = prettytable::Table::new();
            let mut header = prettytable::Row::empty();
            header.add_cell(Cell::new("id"));
            header.add_cell(Cell::new("student"));
            for j in &projects {
                header.add_cell(Cell::new(&format!("{}[aut]", j.path)));
                header.add_cell(Cell::new(&format!("{}[man]", j.path)));
            }
            table.add_row(header);
            for i in &students {
                let mut row = prettytable::Row::empty();
                row.add_cell(Cell::new(&i.id.to_string()));
                row.add_cell(Cell::new(&i.path));
                for j in &projects {
                    use schema::grade::dsl as g;
                    let grade: QueryResult<model::Grade> = g::grade
                        .filter(g::student_id
                            .eq(i.id)
                            .and(g::project_id.eq(j.id)))
                        .first::<model::Grade>(conn);
                    match grade {
                        Ok(g) => {
                            row.add_cell(prettytable::Cell::new(&g.auto_grade.to_string()));
                            row.add_cell(prettytable::Cell::new(&g.manual_grade.to_string()));
                        }
                        _ => {
                            row.add_cell(prettytable::Cell::default());
                            row.add_cell(prettytable::Cell::default());
                        }
                    }
                }
                table.add_row(row);
            }
            table.printstd();
        }
        StatusCommand::Grades { student_id, project_id } => {
            let mut query = schema::grade::table
                .load::<model::Grade>(conn)
                .unwrap_with_log();
            if let Some(id) = student_id {
                query = query.into_iter().filter(|x| x.student_id == *id).collect()
            }
            if let Some(id) = project_id {
                query = query.into_iter().filter(|x| x.project_id == *id).collect()
            }
            println!("{}", tablefy::into_string(&query));
        }
    }
}

