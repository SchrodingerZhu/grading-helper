use std::path::PathBuf;
use std::process::ExitStatus;

use anyhow::*;
use diesel::{QueryDsl, SqliteConnection};
use diesel::prelude::*;

use crate::container::*;
use crate::utils::{AndThenInto, UnwrapWithLog};

/// # Project Template
/// project-name
/// - student/
/// - build.sh
/// - run.sh
/// - other-stuffs
#[derive(structopt::StructOpt, Debug)]
pub enum JudgeCommand {
    #[structopt(about = "Build the current project and run")]
    Go {
        #[structopt(long, short, help = "Show stdout and stderr")]
        verbose: bool
    },
    #[structopt(about = "Edit comment")]
    Comment {
        #[structopt(long, short, help = "Set the editor to use", env = "EDITOR", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "Change manual grade")]
    ManualGrade {
        #[structopt(long, short, help = "Target grade")]
        grade: i32
    },
    #[structopt(about = "Change auto grade")]
    AutoGrade {
        #[structopt(long, short, help = "Target grade")]
        grade: i32
    },
}

#[derive(Debug)]
struct BuildResult {
    stdout: String,
    stderr: String,
    return_code: ExitStatus,
}

fn build(container: &Container) -> Result<BuildResult> {
    container
        .cmd()
        .and_then_into(|mut x| {
            x.arg("sh")
                .arg("build.sh")
                .output()
        })
        .and_then(|output| {
            Ok(BuildResult {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                return_code: output.status,
            })
        })
}

struct RunResult {
    stdout: String,
    stderr: String,
    return_code: ExitStatus,
    auto_grade: i32,
}

fn run(container: &Container) -> Result<RunResult> {
    let output = container
        .cmd()
        .and_then_into(|mut x| {
            x.arg("sh")
                .arg("run.sh")
                .output()
        })?;

    let grade = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .ok_or(anyhow::anyhow!("empty output"))
            .and_then_into(|x|
                x.split("/").next()
                    .and_then(|x| x.split("[RESULT] ")
                        .last())
                    .ok_or(anyhow!("invalid format")))
            .and_then_into(|x| x.parse())
            .map_err(|x| {
                log::error!("failed to extract auto grade: {}", x);
                x
            }).unwrap_or(0)
    } else { 0 };
    Ok(RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        return_code: output.status,
        auto_grade: grade,
    })
}

pub fn handle(conn: &SqliteConnection, subcommand: &JudgeCommand) {
    let mut conf = crate::model::Configuration::get_global(conn)
        .unwrap_with_log();
    if conf.current_student.is_none() {
        log::error!("please set a student first");
        std::process::exit(1);
    }
    match subcommand {
        JudgeCommand::Comment { editor } => {
            dialoguer::Editor::new()
                .executable(editor)
                .edit(conf.comment.as_ref().map(AsRef::as_ref).unwrap_or(""))
                .and_then_into(|new_comment| {
                    match new_comment {
                        None => Ok(()),
                        Some(comment) => {
                            conf.comment.replace(comment);
                            conf.store(conn)
                                .and(Ok(()))
                        }
                    }
                })
                .unwrap_with_log()
        }
        JudgeCommand::ManualGrade { grade } => {
            conf.manual_grade.replace(*grade);
            conf.store(conn)
                .unwrap_with_log();
        }
        JudgeCommand::AutoGrade { grade } => {
            conf.auto_grade.replace(*grade);
            conf.store(conn)
                .unwrap_with_log();
        }
        JudgeCommand::Go { verbose } => {
            let project: crate::model::Project = crate::schema::project::table
                .find(conf.current_project.clone()
                    .unwrap())
                .get_result(conn)
                .unwrap_with_log();
            let student: crate::model::Student = crate::schema::student::table
                .find(conf.current_student.clone()
                    .unwrap())
                .get_result(conn)
                .unwrap_with_log();
            let student_path = PathBuf::from(student.path);
            Container::new(
                conf.base_image.as_ref(),
                student_path.as_path(),
                project.path.as_ref(),
            ).and_then(|x| {
                build(&x)
                    .map(|y| (y, x))
            }).and_then(|(x, container)| {
                if *verbose {
                    log::info!("Return Code: {}", x.return_code.code()
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| String::from("unknown")));
                    log::info!("Compile Stdout: \n{}", x.stdout);
                    log::info!("Compile Stderr: \n{}", x.stderr);
                }
                conf.compile_stdout.replace(x.stdout);
                conf.compile_stderr.replace(x.stderr);
                conf.compile_return = x.return_code.code();
                conf.store(conn)
                    .and(if x.return_code.success() {Ok(container)} else {
                        Err(anyhow!("compile failed"))
                    })
            }).and_then(|container| {
                run(&container)
            }).and_then(|x| {
                if *verbose {
                    log::info!("Return Code: {}", x.return_code.code()
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| String::from("unknown")));
                    log::info!("Run Stdout: \n{}", x.stdout);
                    log::info!("Run Stderr: \n{}", x.stderr);
                }
                conf.run_stdout.replace(x.stdout);
                conf.run_stderr.replace(x.stderr);
                conf.run_return = x.return_code.code();
                conf.auto_grade.replace(x.auto_grade);
                conf.store(conn)
                    .and(if x.return_code.success() {Ok(())} else {
                        Err(anyhow!("runtime failed"))
                    })
            }).unwrap_with_log();
        }
    }
}