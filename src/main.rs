use std::{process::{Command, Stdio}, path::{Path, PathBuf}, fs::{self, OpenOptions, File}, io::{Write, Read}, collections::HashMap};
use anyhow::{Context, Result, anyhow};
use clap::Parser;

pub struct LitFile {
    testdir: PathBuf,
    litcmd: LitCmd,
    debug: bool,
}

impl LitFile {
    pub fn new<P: AsRef<Path>>(input: P, defines: Option<HashMap<String, String>>, debug: bool) -> Result<Self> {
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
        let code = format!(r#"config.suffixes = ["{}"]"#, &input_suffix);
        let litcfg = litcfg.replace("### __LITFILE_SUFFIX__", &code);

        let mut code = vec![];
        if let Some(defines) = defines {
            for (k, v) in defines {
                code.push(format!(r#"config.substitutions.append(("%{}", "{}"))"#, k, v));
            }
        }
        let litcfg = litcfg.replace("### __LITFILE_SUBSTITUTIONS__", &code.join("\n"));

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

        Ok(LitFile { testdir, litcmd: LitCmd::new()?, debug })
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
            if !self.debug {
                if let Err(e) = fs::remove_dir_all(&self.testdir) {
                    println!(
                        "failed to remove temporary directory {} with error {e:?}",
                        self.testdir.display()
                    );
                }
            } else {
                println!("debug files can be found at: {}", self.testdir.display());
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
        let options = {
            if let Ok(val) = std::env::var("LIT_OPTIONS") {
                val
            } else {
                "".to_owned()
            }
        };
        let mut litcmd = Command::new(&self.cmdpath)
            .args(["-sv", &format!("{}", testdir.display())])
            .args(&shlex::split(&options).context(format!("failed to parse LIT_OPTIONS: {options}"))?)
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

#[derive(Parser)]
#[clap(verbatim_doc_comment)]
#[command(version)]
/// litfile - run llvm lit on single file
///
/// To pass extra options to lit, you can use environment LIT_OPTIONS, for example
///
///     LIT_OPTIONS="-o output.log" litfile /path/to/test.cpp
///
pub struct Cli {
    /// The file to test
    file: PathBuf,
    /// Provide in <KEY>=<VALUE> format such as "-D CLANG=/path/to/clang", you can repeat this
    /// option many times
    #[arg(short = 'D', value_name = "LIT_VARIABLE")]
    defines: Option<Vec<String>>,
    /// Keep generated lit files
    #[arg(long)]
    debug: bool,
}

fn app() -> Result<()> {
    let cli = Cli::parse();
    let input = cli.file;
    let defines = if let Some(defines) = cli.defines {
        let mut mp = HashMap::new();
        for entry in defines {
            let kvs = entry.split("=").collect::<Vec<&str>>();
            if let (Some(k), Some(v)) = (kvs.get(0), kvs.get(1)) {
                mp.insert(k.trim().to_owned(), v.trim().to_owned());
            }
        }
        if mp.len() > 0 {
            Some(mp)
        } else {
            None
        }
    } else {
        None
    };
    let litfile = LitFile::new(input, defines, cli.debug)?;
    litfile.info()?;
    litfile.run()
}

fn main() {
    if let Err(err) = app() {
        println!("{err:?}");
        std::process::exit(1);
    }
}
