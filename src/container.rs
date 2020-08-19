use std::path::Path;
use std::process::Command;

use anyhow::*;
use tempfile as tmp;
use tempfile::{tempdir, TempDir, TempPath};

trait WaitSuccess {
    fn wait_success(&mut self) -> Result<()>;
}

impl WaitSuccess for std::process::Child {
    fn wait_success(&mut self) -> Result<(), Error> {
        self.wait()
            .map_err(Into::into)
            .and_then(|x| if x.success() {
                Ok(())
            } else {
                Err(anyhow!("exit with {}", x.code().map(|x|x.to_string()).unwrap_or_else(String::new)))
            })
    }
}

fn create_magic() -> Result<tmp::NamedTempFile> {
    let file = tmp::NamedTempFile::new()?;
    let display = std::env::var("DISPLAY")?;
    Command::new("sh")
        .arg("-c")
        .arg(format!("xauth nextract - \"{}\" | sed -e 's/^..../ffff/' | xauth -f \"{}\" nmerge -", display,
                     file.path().to_str().ok_or(anyhow!("file path initialized error"))?))
        .spawn()?
        .wait_success()?;
    Ok(file)
}



pub struct Container {
    lower_dir: tmp::TempDir,
    magic: tmp::NamedTempFile,
    _root_mount_pair: (tmp::TempDir, tmp::TempDir),
    _project_mount_pair: (tmp::TempDir, tmp::TempDir),
    _student_mount_pair: (tmp::TempDir, tmp::TempDir)
}

impl Container {
    fn overlay<A, B>(base_path: A, target_path: B) -> Result<(tmp::TempDir, tmp::TempDir)>
    where A : AsRef<Path>, B : AsRef<Path>{
        let current_dir = std::env::current_dir()?;
        let upper_dir = tmp::TempDir::new_in(&current_dir)?;
        let work_dir = tmp::TempDir::new_in(&current_dir)?;
        Command::new("sudo")
            .arg("mount")
            .arg("-t")
            .arg("overlay")
            .arg("-o")
            .arg(format!(
                "lowerdir={},upperdir={},workdir={}",
                base_path.as_ref().to_str().ok_or(anyhow!("invalid lower_dir"))?,
                upper_dir.path().to_str().ok_or(anyhow!("invalid upper_dir"))?,
                work_dir.path().to_str().ok_or(anyhow!("invalid work_dir"))?,
            ))
            .arg("overlay")
            .arg(target_path.as_ref())
            .spawn()?
            .wait_success()?;
        Ok((upper_dir, work_dir))
    }
    pub fn new(image_path: &Path, student_dir: &Path, project_dir: &Path) -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let lower_dir = tmp::TempDir::new_in(&current_dir)?;
        Command::new("sudo")
            .arg("mount")
            .arg(image_path)
            .arg(lower_dir.path())
            .arg("-t")
            .arg("squashfs")
            .arg("-o")
            .arg("loop")
            .spawn()?
            .wait_success()?;
        let _root_mount_pair
            = Self::overlay(lower_dir.path(), lower_dir.path())?;
        let mut project_target = lower_dir.path().to_path_buf();
        project_target.push("project");
        let mut student_target = project_target.clone();
        student_target.push("student");
        let _project_mount_pair = Self::overlay(project_dir, project_target)?;
        let _student_mount_pair = Self::overlay(student_dir, student_target)?;
        let magic = create_magic()?;
        Ok(Container {
            lower_dir,
            magic,
            _root_mount_pair,
            _project_mount_pair,
            _student_mount_pair
        })
    }
    pub fn clean_up(&self) -> Result<()> {
        Command::new("sudo")
            .arg("umount")
            .arg("-R")
            .arg(self.lower_dir.path())
            .spawn()?
            .wait_success()?;
        Command::new("sudo")
            .arg("umount")
            .arg("-R")
            .arg(self.lower_dir.path())
            .spawn()?
            .wait_success()?;
        Ok(())
    }
    pub fn cmd(&self) -> Result<Command> {
        let display = std::env::var("DISPLAY")?;
        let mut command = Command::new("sudo");
        command
            .arg("systemd-nspawn")
            .arg("--quiet")
            .arg("-D")
            .arg(self.lower_dir.path())
            .arg("--bind=/tmp/.X11-unix")
            .arg("--bind")
            .arg(self.magic.path())
            .arg("-E")
            .arg(format!("DISPLAY={}", display))
            .arg("-E")
            .arg(format!("XAUTHORITY={}",
                         self.magic.path().to_str().ok_or(anyhow!("file path initialized error"))?))
            .arg("--as-pid2");
        Ok(command)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_magic() -> Result<()> {
        let file = create_magic()?;
        let content = std::fs::read(file.path())?;
        Ok(println!("{}", String::from_utf8_lossy(content.as_slice())))
    }
}