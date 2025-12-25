use centy_installer::{Installer, Project, VersionManager};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "centy-installer")]
#[command(about = "Installer for Centy TUI and TUI Manager binaries")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a project binary
    Install {
        /// Project to install (tui, tui-manager)
        project: String,

        /// Version to install (e.g., 1.0.0). If not specified, installs latest
        #[arg(short, long)]
        version: Option<String>,

        /// Install from a local file instead of downloading
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Uninstall a project binary
    Uninstall {
        /// Project to uninstall (tui, tui-manager)
        project: String,

        /// Version to uninstall. If not specified, uninstalls all versions
        #[arg(short, long)]
        version: Option<String>,
    },

    /// List installed binaries
    List {
        /// Only list versions for a specific project
        #[arg(short, long)]
        project: Option<String>,
    },

    /// List available versions from GitHub
    Available {
        /// Project to list versions for (tui, tui-manager)
        project: String,

        /// Include prerelease versions
        #[arg(long)]
        prerelease: bool,
    },

    /// Get the path to an installed binary
    Which {
        /// Project (tui, tui-manager)
        project: String,

        /// Version
        version: String,
    },

    /// Show installation directory information
    Info,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> centy_installer::Result<()> {
    let installer = Installer::new()?;

    match cli.command {
        Commands::Install {
            project,
            version,
            file,
        } => {
            let proj = parse_project(&project)?;

            if let Some(file_path) = file {
                let version = version.ok_or_else(|| {
                    centy_installer::InstallerError::InvalidVersion(
                        "Version is required when installing from file".to_string(),
                    )
                })?;
                installer.install_from_file(proj, &version, &file_path)?;
            } else {
                let version = match version {
                    Some(v) => v,
                    None => {
                        println!("Fetching latest version...");
                        let vm = VersionManager::new("centy-io".to_string())?;
                        vm.get_latest_version(&proj).await?
                    }
                };
                installer.install(proj, &version).await?;
            }
        }

        Commands::Uninstall { project, version } => {
            let proj = parse_project(&project)?;

            if let Some(v) = version {
                installer.uninstall(proj, &v)?;
            } else {
                installer.uninstall_all(proj)?;
            }
        }

        Commands::List { project } => {
            if let Some(project_name) = project {
                let proj = parse_project(&project_name)?;
                let versions = installer.paths().list_versions(proj.name())?;

                if versions.is_empty() {
                    println!("No versions of {} installed", proj.display_name());
                } else {
                    println!("Installed versions of {}:", proj.display_name());
                    for v in versions {
                        let binaries = installer.paths().list_binaries(proj.name(), &v)?;
                        println!("  {} (binaries: {})", v, binaries.join(", "));
                    }
                }
            } else {
                let installed = installer.list_installed()?;

                if installed.is_empty() {
                    println!("No binaries installed");
                } else {
                    println!("Installed binaries:");
                    for (proj, versions) in installed {
                        println!("\n{}:", proj.display_name());
                        for v in versions {
                            let binaries = installer.paths().list_binaries(proj.name(), &v)?;
                            println!("  {} (binaries: {})", v, binaries.join(", "));
                        }
                    }
                }
            }
        }

        Commands::Available {
            project,
            prerelease,
        } => {
            let proj = parse_project(&project)?;
            let vm = VersionManager::new("centy-io".to_string())?;

            println!("Fetching available versions for {}...", proj.display_name());
            let versions = vm.list_available_versions(&proj, prerelease).await?;

            if versions.is_empty() {
                println!("No versions available");
            } else {
                println!("Available versions:");
                for v in versions {
                    println!("  {}", v);
                }
            }
        }

        Commands::Which { project, version } => {
            let proj = parse_project(&project)?;
            let path = installer.get_binary_path(proj, &version)?;
            println!("{}", path.display());
        }

        Commands::Info => {
            let paths = installer.paths();
            println!("Centy Installer Information");
            println!("===========================");
            println!("Base directory: {}", paths.base_dir().display());
            println!("Bin directory:  {}", paths.bin_dir().display());
            println!();
            println!("Installation path structure:");
            println!("  ~/.centy/bin/<project>/<version>/<binary>");
            println!();
            println!("Supported projects:");
            for proj in Project::all() {
                println!("  - {} ({})", proj.name(), proj.display_name());
            }
        }
    }

    Ok(())
}

fn parse_project(s: &str) -> centy_installer::Result<Project> {
    Project::parse(s).ok_or_else(|| centy_installer::InstallerError::ProjectNotFound(s.to_string()))
}
