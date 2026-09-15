use std::{fs, io::Write, path::PathBuf};

use clap::{Args, ValueEnum, ValueHint};
use forge_approval_template::ApprovalTemplate;

use crate::Result;
use crate::forge_project::find_manifest_path;

/// `approval-template` arguments.
#[derive(Args, Debug)]
pub(crate) struct ApprovalTemplateArgs {
    /// Forge app directory. Defaults to the current directory.
    #[arg(name = "APP_DIR", value_hint = ValueHint::DirPath, default_value = ".")]
    app_dir: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    format: Format,

    /// Write to this file instead of standard output.
    #[arg(long, short, value_name = "PATH", value_hint = ValueHint::FilePath)]
    out: Option<PathBuf>,

    /// Exit non-zero when a check would block listing.
    ///
    /// Intended for CI, so a partner's pipeline fails before submission rather
    /// than after review.
    #[arg(long)]
    fail_on_blocking: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
enum Format {
    /// Human-readable summary.
    Markdown,
    /// Canonical machine-readable form.
    Json,
    /// Machine-readable form, matching the manifest's own syntax.
    Yaml,
}

impl ApprovalTemplateArgs {
    pub(crate) fn diagnostic_logging_requested(&self) -> bool {
        false
    }
}

pub(crate) fn run(args: &ApprovalTemplateArgs) -> Result<()> {
    let manifest_path = find_manifest_path(&args.app_dir)?;
    let manifest = fs::read_to_string(&manifest_path)?;
    let template = ApprovalTemplate::from_manifest(&manifest, manifest_path.display().to_string())?;

    let rendered = match args.format {
        Format::Markdown => forge_approval_template::to_markdown(&template),
        Format::Json => serde_json::to_string_pretty(&template)?,
        Format::Yaml => serde_yaml::to_string(&template)?,
    };

    match &args.out {
        Some(path) => {
            fs::write(path, rendered)?;
            println!("Wrote {} to {}", args.format.describe(), path.display());
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(rendered.as_bytes())?;
            stdout.flush()?;
        }
    }

    let blocking = template.blocking_flags().count();
    if blocking > 0 {
        eprintln!("\n{blocking} check(s) would block listing. Resolve them before submitting.");
        if args.fail_on_blocking {
            return Err(format!("{blocking} blocking check(s) found").into());
        }
    }

    Ok(())
}

impl Format {
    fn describe(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown summary",
            Self::Json => "JSON template",
            Self::Yaml => "YAML template",
        }
    }
}
