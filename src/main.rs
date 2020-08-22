#[macro_use]
extern crate diesel;

use std::path::PathBuf;

use diesel::{BoolExpressionMethods, Connection, ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use prettytable::Cell;
use serde::export::fmt::Display;
use structopt as opt;
use structopt::StructOpt;

use utils::*;

use crate::judge::JudgeCommand;
use crate::model::{ChangeStudent, Configuration};

mod container;
mod schema;
mod model;
mod status;
mod utils;
mod judge;
mod dump;

#[derive(opt::StructOpt, Debug)]
struct Opt {
    #[structopt(short, long,
    possible_values = & ["warn", "off", "info", "debug", "error", "trace"],
    default_value = "info", env = "HELPER_LOG_LEVEL", help = "logging level")]
    log_level: String,
    #[structopt(short, long, default_value = ".", env = "HELPER_WORKDIR",
    help = "Working directory")]
    workdir: std::path::PathBuf,
    #[structopt(short, long, env = "HELPER_DATABASE",
    help = "Path to SQLite Database")]
    database: std::path::PathBuf,
    #[structopt(subcommand)]
    subcommand: SubCommand,
}

#[derive(opt::StructOpt, Debug)]
enum SubCommand {
    #[structopt(about = "Initialize grading")]
    Init {
        #[structopt(short, long, help = "Path to the root image")]
        base_image: PathBuf
    },
    #[structopt(about = "Check status")]
    Status {
        #[structopt(subcommand)]
        subcommand: status::StatusCommand,
    },
    #[structopt(about = "Commit current grading result")]
    Commit,
    /// no opts now
    #[structopt(about = "Clean current grading status")]
    Clean {
        #[structopt(subcommand)]
        subcommand: CleanCommand,
    },
    #[structopt(about = "Remove graded items")] // this is more dangerous
    Remove {
        #[structopt(long, help = "Remove all grades")]
        all: bool,
        #[structopt(short, long, help = "Remove by id (ignored if all is set)", required_unless = "all")]
        id: Option<i32>,
    },
    #[structopt(about = "Project templates management")]
    Project {
        #[structopt(subcommand)]
        subcommand: ProjectCommand
    },
    #[structopt(about = "Student management")]
    Student {
        #[structopt(subcommand)]
        subcommand: StudentCommand
    },
    #[structopt(about = "Get next student or project")]
    Next {
        #[structopt(subcommand)]
        subcommand: NextCommand
    },
    #[structopt(about = "Judge current project")]
    Judge {
        #[structopt(subcommand)]
        subcommand: JudgeCommand
    },
    #[structopt(about = "Dump grades")]
    Dump {
        #[structopt(long, short, help = "Path to the output file")]
        target: String
    },
}

#[derive(opt::StructOpt, Debug)]
enum NextCommand {
    #[structopt(about = "Get next student")]
    Student {
        #[structopt(short, long, help = "Get the student with id instead of next non-grading one")]
        id: Option<i32>
    },
    #[structopt(about = "Get next project")]
    Project {
        #[structopt(short, long, help = "Set the project to grade")]
        id: i32
    },
}

#[derive(opt::StructOpt, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum CleanCommand {
    #[structopt(about = "Clean current compile and run result")]
    Result,
    #[structopt(about = "Clean current auto grade")]
    AutoGrade,
    #[structopt(about = "Clean current manual grade")]
    ManualGrade,
    #[structopt(about = "Clean current comment")]
    Comment,
    #[structopt(about = "Clean current student and keep the project")]
    Student,
    #[structopt(about = "Clean current project and keep the student")]
    Project,
    #[structopt(about = "Clear all grading status")]
    All,
    #[structopt(about = "Clear global config")]
    Config,
}


#[derive(opt::StructOpt, Debug)]
enum ProjectCommand {
    #[structopt(about = "Add a new template")]
    Add {
        #[structopt(short, long, help = "Path to the project template")]
        path: PathBuf,
        #[structopt(short, long, help = "Project identifier")]
        name: String,
    },
    #[structopt(about = "Remove the template")]
    Remove {
        #[structopt(short, long, help = "The id to remove")]
        id: i32
    },
}

#[derive(opt::StructOpt, Debug)]
enum StudentCommand {
    #[structopt(about = "Add a new student")]
    Add {
        #[structopt(short, long, help = "Path to the student submission")]
        path: PathBuf,
    },
    #[structopt(about = "Remove the student")]
    Remove {
        #[structopt(short, long, help = "The id to remove")]
        id: i32
    },
}


/// TODO: Change the logic of grading process
/// Currently, we can iterate through projects and students at the same time
/// However, this brings too much load for check the correct logic
/// Hence, change the logic to
/// 1. first setup a project
/// 2. then select a student to grade

fn main() {
    let env = dotenv::dotenv();
    let opt: Opt = Opt::from_args();
    std::env::set_var("HELPER_LOG_LEVEL", &opt.log_level);
    pretty_env_logger::init_custom_env("HELPER_LOG_LEVEL");
    match env {
        Ok(path) => log::debug!("dotenv initialized with {}", path.display()),
        Err(e) => log::warn!("dotenv failed to initialize: {}", e)
    }
    let conn = opt.database
        .to_str()
        .ok_or(anyhow::anyhow!("invalid database path"))
        .and_then_into(|path|
            diesel::SqliteConnection::establish(path))
        .unwrap_with_log();
    match &opt.subcommand {
        SubCommand::Init { base_image } => {
            base_image
                .to_str()
                .ok_or(anyhow::anyhow!("invalid image path"))
                .and_then_into(|path| model::Configuration::initialize(&conn, path))
                .and_then_into(|_| {
                    std::fs::read_dir(&opt.workdir)
                })
                .map(|directory| {
                    directory.into_iter()
                        .filter_map(|x| match x {
                            Err(e) => {
                                log::error!("error while scan directory: {}", e);
                                None
                            }
                            Ok(entry) => Some(entry.path())
                        })
                        .filter(|x| x.is_dir() && !x.starts_with("."))
                        .flat_map(std::fs::canonicalize)
                        .flat_map(|x| x.to_str().map(|x| x.to_string()))
                })
                .and_then(|iter| {
                    iter.fold(Ok(0), |now, st_path| {
                        use schema::student::*;
                        now.and_then_into(|x|
                            diesel::insert_into(table)
                                .values(&ChangeStudent {
                                    path: Some(&st_path),
                                })
                                .execute(&conn)
                                .map(|y| x + y))
                    })
                })
                .map(|x| log::info!("{} entries added", x))
                .unwrap_with_log();
        }
        SubCommand::Commit => {
            model::Configuration::get_global(&conn)
                .and_then(|mut conf| {
                    if conf.current_project.is_none() {
                        Err(anyhow::anyhow!("no current grading project"))
                    } else if conf.current_student.is_none() {
                        Err(anyhow::anyhow!("not current grading student"))
                    } else {
                        use crate::schema::grade::dsl as g;
                        let grade: QueryResult<crate::model::Grade> = g::grade
                            .filter(g::student_id
                                .eq(conf.current_student.unwrap())
                                .and(g::project_id.eq(conf.current_project.unwrap())))
                            .first::<crate::model::Grade>(&conn);
                        let grade = model::ChangeGrade {
                            id: match grade {
                                Ok(x) => Some(x.id),
                                _ => None
                            },
                            student_id: conf.current_student.take(),
                            project_id: conf.current_project.clone(),
                            manual_grade: conf.manual_grade.take(),
                            auto_grade: conf.auto_grade.take(),
                            comment: conf.comment.take(),
                            compile_stdout: conf.compile_stdout.take(),
                            compile_stderr: conf.compile_stderr.take(),
                            compile_return: conf.compile_return.take(),
                            run_stdout: conf.run_stdout.take(),
                            run_stderr: conf.run_stderr.take(),
                            run_return: conf.run_return.take(),
                        };

                        diesel::replace_into(schema::grade::table)
                            .values(grade)
                            .execute(&conn)
                            .and_then_into(|_| {
                                conf.store(&conn)
                            })
                    }
                })
                .unwrap_with_log();
        }
        SubCommand::Dump { target } => {
            dump::dump(&conn, target);
        }
        SubCommand::Clean { subcommand } => {
            if subcommand == &CleanCommand::Config {
                diesel::delete(schema::configuration::table)
                    .execute(&conn)
                    .unwrap_with_log();
            } else {
                model::Configuration::get_global(&conn)
                    .and_then(|mut conf| {
                        if subcommand == &CleanCommand::Result || subcommand >= &CleanCommand::Student {
                            conf.compile_return.take();
                            conf.compile_stderr.take();
                            conf.compile_stdout.take();
                            conf.run_return.take();
                            conf.run_stderr.take();
                            conf.run_stdout.take();
                        }
                        if subcommand == &CleanCommand::Comment || subcommand >= &CleanCommand::Student {
                            conf.comment.take();
                        }
                        if subcommand == &CleanCommand::AutoGrade || subcommand >= &CleanCommand::Student {
                            conf.auto_grade.take();
                        }
                        if subcommand == &CleanCommand::ManualGrade || subcommand >= &CleanCommand::Student {
                            conf.manual_grade.take();
                        }
                        if subcommand >= &CleanCommand::Student {
                            conf.current_student.take();
                        }
                        if subcommand >= &CleanCommand::Project {
                            conf.current_project.take();
                        }
                        conf.store(&conn)
                    })
                    .unwrap_with_log();
            }
        }
        SubCommand::Project { subcommand } => {
            let sql_result = match subcommand {
                ProjectCommand::Remove { id: target_id } => {
                    use schema::project::dsl::*;
                    diesel::delete(project)
                        .filter(id.eq(target_id))
                        .execute(&conn)
                        .map_err(Into::into)
                }
                ProjectCommand::Add { path, name } => {
                    path.to_str()
                        .ok_or(anyhow::anyhow!("invalid path"))
                        .and_then_into(|x| {
                            diesel::insert_into(schema::project::table)
                                .values(model::ChangeProject {
                                    path: Some(x),
                                    name: Some(name),
                                })
                                .execute(&conn)
                        })
                }
            };
            match sql_result {
                Ok(delta) => {
                    log::info!("updated {} item(s)", delta);
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
        SubCommand::Student { subcommand } => {
            let sql_result = match subcommand {
                StudentCommand::Remove { id: target_id } => {
                    use schema::project::dsl::*;
                    diesel::delete(project)
                        .filter(id.eq(target_id))
                        .execute(&conn)
                        .map_err(Into::into)
                }
                StudentCommand::Add { path } => {
                    path.canonicalize()
                        .and_then_into(|path| path
                            .to_str()
                            .ok_or(anyhow::anyhow!("invalid path"))
                            .and_then_into(|x| {
                                diesel::insert_into(schema::student::table)
                                    .values(model::ChangeStudent {
                                        path: Some(x),
                                    })
                                    .execute(&conn)
                            }))
                }
            };
            match sql_result {
                Ok(delta) => {
                    log::info!("updated {} item(s)", delta);
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
        SubCommand::Next { subcommand } => {
            use schema::grade::dsl as g;
            use schema::project::dsl as p;
            use schema::student::dsl as s;
            let mut conf = model::Configuration::get_global(&conn)
                .unwrap_with_log();
            let result = match subcommand {
                NextCommand::Project { id } => {
                    if conf.current_student.is_some() {
                        Err(anyhow::anyhow!("Please commit current student first"))
                    } else {
                        conf.current_project.replace(*id);
                        diesel::select(diesel::dsl::exists(p::project.find(id)))
                            .get_result(&conn)
                            .and_then_into(|flag|
                                {
                                    if flag {
                                        conf.store(&conn).and(Ok(()))
                                    } else {
                                        Err(anyhow::anyhow!("no such project"))
                                    }
                                })
                    }
                }
                NextCommand::Student { id } => {
                    if conf.current_project.is_none() {
                        Err(anyhow::anyhow!("Please set a project first"))
                    } else if conf.current_student.is_some() {
                        Err(anyhow::anyhow!("Please commit current student first"))
                    } else {
                        let target: anyhow::Result<i32> = if let Some(id) = id {
                            diesel::select(diesel::dsl::exists(s::student.find(id)))
                                .get_result(&conn)
                                .and_then_into(|flag| if flag { Ok(*id) } else { Err(anyhow::anyhow!("no such student")) })
                        } else {
                            s::student.filter(diesel::dsl::not(
                                diesel::dsl::exists(
                                    g::grade.filter(g::project_id
                                        .eq(conf.current_project.unwrap())
                                        .and(g::student_id.eq(s::id))))))
                                .select(s::id)
                                .first(&conn)
                                .map_err(Into::into)
                        };
                        target.and_then(|id| {
                            conf.current_student.replace(id);
                            conf.store(&conn)
                                .and(Ok(()))
                        })
                    }
                }
            };
            result.unwrap_with_log();
        }
        SubCommand::Judge { subcommand } => {
            judge::handle(&conn, subcommand)
        }
        SubCommand::Status { subcommand } => {
            status::handle(subcommand, &conn)
        }
        SubCommand::Remove { all, id } => {
            if *all {
                dialoguer::Confirm::new().with_prompt("Are you sure to remove all grades")
                    .interact()
                    .and_then_into(|x| if x {
                        diesel::delete(schema::grade::table)
                            .execute(&conn)
                            .map(|x| {
                                log::info!("deleted {} items", x)
                            })
                            .map_err(Into::into)
                    } else {
                        Err(anyhow::anyhow!("operation canceled"))
                    })
                    .unwrap_with_log();
            } else {
                let id = id.unwrap();
                dialoguer::Confirm::new().with_prompt(format!("Are you sure to remove grade #{}", id))
                    .interact()
                    .and_then_into(|x| if x {
                        diesel::delete(schema::grade::table
                            .filter(schema::grade::id.eq(id)))
                            .execute(&conn)
                            .map(|x| {
                                log::info!("deleted {} items", x)
                            })
                            .map_err(Into::into)
                    } else {
                        Err(anyhow::anyhow!("operation canceled"))
                    })
                    .unwrap_with_log();
            }
        }
    }
}

