#[macro_use]
extern crate diesel;

use std::path::PathBuf;

use diesel::{Connection, RunQueryDsl, ExpressionMethods};
use serde::export::fmt::Display;
use structopt as opt;
use structopt::StructOpt;

use crate::model::{ChangeStudent, Configuration};

mod container;
mod schema;
mod model;

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
        subcommand: StatusCommand,
    },
    #[structopt(about = "Commit current grading result")]
    Commit {
        #[structopt(short, long, help = "Ignore current result and clear the status")]
        skip: bool
    },
    #[structopt(about = "Project templates management")]
    Project {
        #[structopt(subcommand)]
        subcommand: ProjectCommand
    },
}

#[derive(opt::StructOpt, Debug)]
enum StatusCommand {
    #[structopt(about = "Check configuration and current project")]
    Current,
    #[structopt(about = "List all project template(s)")]
    Projects
}

#[derive(opt::StructOpt, Debug)]
enum ProjectCommand {
    #[structopt(about = "Add a new template")]
    Add {
        #[structopt(short, long, help = "Path to the project template")]
        path: PathBuf,
        #[structopt(short, long, help = "Grade for manual judging")]
        manual_grade: i32,
        #[structopt(short, long, help = "Grade for automatic judging")]
        auto_grade: i32,
    },
    #[structopt(about = "Remove the template")]
    Remove {
        #[structopt(short, long, help="The id to remove")]
        id: i32
    }
}

trait UnwrapWithLog<T> {
    fn unwrap_with_log(self) -> T;
}

trait AndThenInto<T, U> {
    fn and_then_into<E: Into<anyhow::Error>, F>
    (self, f: F) -> anyhow::Result<U> where F: FnOnce(T) -> std::result::Result<U, E>;
}

impl<T, E: Into<anyhow::Error>, U> AndThenInto<T, U> for std::result::Result<T, E> {
    fn and_then_into<K: Into<anyhow::Error>, F>
    (self, f: F) -> anyhow::Result<U>
        where F: FnOnce(T) -> std::result::Result<U, K> {
        self.map_err(Into::into)
            .and_then(|x| f(x).map_err(Into::into))
    }
}

impl<T, E: Display> UnwrapWithLog<T> for std::result::Result<T, E> {
    fn unwrap_with_log(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                log::error!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

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
                                    graded: Some(false),
                                })
                                .execute(&conn)
                                .map(|y| x + y))
                    })
                })
                .map(|x| log::info!("{} entries added", x))
                .unwrap_with_log();
        }
        SubCommand::Commit { skip } => {
            model::Configuration::get_global(&conn)
                .and_then(|mut conf| {
                    if conf.current_project.is_some() && *skip {
                        let change_set = model::ChangeConfig {
                            id: 1,
                            current_student: None,
                            current_project: None,
                            auto_grade: None,
                            manual_grade: None,
                            comment: None,
                            base_image: Some(conf.base_image.as_str()),
                        };
                        diesel::replace_into(schema::configuration::table)
                            .values(change_set)
                            .execute(&conn)
                            .map_err(Into::into)
                            .and(Ok(()))
                    } else if conf.current_project.is_none() {
                        Err(anyhow::anyhow!("no current grading project"))
                    } else if conf.auto_grade.is_none() || conf.manual_grade.is_none() {
                        Err(anyhow::anyhow!("current grading is not finished"))
                    } else {
                        let new_grade = model::ChangeGrade {
                            student_id: conf.current_student.take(),
                            project_id: conf.current_project.take(),
                            manual_grade: conf.manual_grade.take(),
                            auto_grade: conf.auto_grade.take(),
                        };
                        diesel::replace_into(schema::grade::table)
                            .values(new_grade)
                            .execute(&conn)
                            .and_then(|_| {
                                diesel::replace_into(schema::configuration::table)
                                    .values(conf)
                                    .execute(&conn)
                                    .map_err(Into::into)
                            })
                            .map_err(Into::into)
                            .and(Ok(()))
                    }
                })
                .unwrap_with_log();
        }
        SubCommand::Project { subcommand } => {
            let sql_result = match subcommand {
                ProjectCommand::Remove { id : target_id} => {
                    use schema::project::dsl::*;
                    diesel::delete(project)
                        .filter(id.eq(target_id))
                        .execute(&conn)
                        .map_err(Into::into)
                }
                ProjectCommand::Add { path, manual_grade, auto_grade } => {
                    path.to_str()
                        .ok_or(anyhow::anyhow!("invalid path"))
                        .and_then_into(|x| {
                            diesel::insert_into(schema::project::table)
                                .values(model::ChangeProject {
                                    path: Some(x),
                                    manual_grade: Some(*manual_grade),
                                    auto_grade: Some(*auto_grade)
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
        SubCommand::Status { subcommand } => {
            match subcommand {
                StatusCommand::Current => {
                    model::Configuration::get_global(&conn)
                        .map(|x| {
                            println!("{}", tablefy::into_string(&vec![x]))
                        })
                        .unwrap_with_log();
                }
                StatusCommand::Projects => {
                    let projects = schema::project::table
                        .load::<model::Project>(&conn)
                        .unwrap_with_log();
                    println!("{}", tablefy::into_string(&projects));
                }
            }
        }
    }
}