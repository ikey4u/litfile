use std::{process::{Command, Stdio}, path::{Path, PathBuf}, fs::{self, OpenOptions, File}, io::{Write, Read}};
use anyhow::{Context, Result, anyhow};

pub struct LitFile {
    testdir: PathBuf,
    litcmd: LitCmd,
}

impl LitFile {
    pub fn new<P: AsRef<Path>>(input: P) -> Result<Self> {
        let input = input.as_ref();

        let input = input.canonicalize()
            .context(format!("invalid input: {}", input.display()))?;
        let input_name = input.file_name()
            .context("cannot get input file name")?
            .to_str()
            .context("filename contain non-utf8 character")?
            .to_owned();
        let inputdir = input.parent().context("input has no parent directory")?;
        let testdir = inputdir.join(input_name.clone() + ".litfile");
        if testdir.exists() {
            fs::remove_dir_all(&testdir)?;
        }
        fs::create_dir_all(&testdir)?;

        let litcfg = include_str!("lit.cfg.py.tpl");
        let input_suffix = format!(".{}", input
            .extension().context("failed to get input extension")?
            .to_str().context("input extension contains non-utf8 character")?
            .to_owned());
        let litcfg = litcfg.replace("__LFVAR_SUFFIX__", &input_suffix);

        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(testdir.join("lit.cfg.py"))?;
        f.write_all(litcfg.as_bytes())?;

        let mut f = File::open(input)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;

        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(testdir.join(input_name))?;
        f.write_all(content.as_bytes())?;

        Ok(LitFile { testdir, litcmd: LitCmd::new()? })
    }

    pub fn run(&self) -> Result<()> {
        LitCmd::new()?.run(self.testdir.as_path())
    }

    pub fn info(&self) -> Result<()> {
        println!("lit version: {}", self.litcmd.version()?);
        Ok(())
    }
}

impl Drop for LitFile {
    fn drop(&mut self) {
        if self.testdir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.testdir) {
                println!(
                    "failed to remove temporary directory {} with error {e:?}",
                    self.testdir.display()
                );
            }
        }
    }
}

struct LitCmd {
    cmdpath: String,
}

impl LitCmd {
    pub fn new() -> Result<Self> {
        if let Ok(cmdpath) = which::which("lit") {
            return Ok(Self {
                cmdpath: cmdpath
                    .to_str()
                    .context("lit path contain invalid character")?
                    .to_owned()
            })
        }
        return Err(anyhow!("lit is not found"));
    }

    pub fn version(&self) -> Result<String> {
        let mut litcmd = Command::new(&self.cmdpath);
        let output = litcmd.args(["--version"]).output()?;
        if !output.status.success() {
            return Err(anyhow!("cannot find lit version"));
        }
        let output = String::from_utf8(output.stdout)?;
        Ok(output.trim_start_matches("lit").trim().to_owned())
    }

    pub fn run<P: AsRef<Path>>(&self, testdir: P) -> Result<()> {
        let testdir = testdir.as_ref();
        let mut litcmd = Command::new(&self.cmdpath)
            .args(["-sv", &format!("{}", testdir.display())])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spwan lit tool");
        let status = litcmd.wait()?;
        if !status.success() {
            return Err(anyhow!(""));
        }
        Ok(())
    }
}

fn app() -> Result<()> {
    let mut args = std::env::args();
    let input = if let Some(input) = args.nth(1) {
        input
    } else {
        return Err(anyhow!("no input file"));
    };

    let litfile = LitFile::new(input)?;
    litfile.info()?;
    litfile.run()
}

fn main() {
    if let Err(err) = app() {
        println!("{err:?}");
        std::process::exit(1);
    }
}
