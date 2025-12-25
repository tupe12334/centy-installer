use centy_installer::{Installer, Project, VersionManager};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "centy")]
#[command(about = "Centy CLI - Install and run Centy tools")]
#[command(version)]
#[command(long_about = "Centy CLI - Install and run Centy tools\n\n\
    When run without arguments, launches centy-tui (installing it first if needed).\n\n\
    Examples:\n  \
    centy              # Run centy-tui\n  \
    centy install tui  # Install centy-tui\n  \
    centy run daemon   # Run centy-daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a project binary
    Install {
        /// Project to install (daemon, tui, tui-manager)
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
        /// Project to uninstall (daemon, tui, tui-manager)
        project: String,

        /// Version to uninstall. If not specified, uninstalls all versions
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Run an installed binary
    Run {
        /// Project to run (daemon, tui, tui-manager). Defaults to tui
        #[arg(default_value = "tui")]
        project: String,

        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List installed binaries
    List {
        /// Only list versions for a specific project
        #[arg(short, long)]
        project: Option<String>,
    },

    /// List available versions from GitHub
    Available {
        /// Project to list versions for (daemon, tui, tui-manager)
        project: String,

        /// Include prerelease versions
        #[arg(long)]
        prerelease: bool,
    },

    /// Get the path to an installed binary
    Which {
        /// Project (daemon, tui, tui-manager)
        project: String,

        /// Version (optional, defaults to latest installed)
        version: Option<String>,
    },

    /// Show installation directory information
    Info,

    /// Install all default binaries (daemon, tui)
    Setup,
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
        // No command = run centy-tui
        None => {
            run_project(&installer, Project::Tui, vec![]).await?;
        }

        Some(Commands::Install {
            project,
            version,
            file,
        }) => {
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

        Some(Commands::Uninstall { project, version }) => {
            let proj = parse_project(&project)?;

            if let Some(v) = version {
                installer.uninstall(proj, &v)?;
            } else {
                installer.uninstall_all(proj)?;
            }
        }

        Some(Commands::Run { project, args }) => {
            let proj = parse_project(&project)?;
            run_project(&installer, proj, args).await?;
        }

        Some(Commands::List { project }) => {
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

        Some(Commands::Available {
            project,
            prerelease,
        }) => {
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

        Some(Commands::Which { project, version }) => {
            let proj = parse_project(&project)?;

            let version = match version {
                Some(v) => v,
                None => {
                    // Get latest installed version
                    let versions = installer.paths().list_versions(proj.name())?;
                    versions.into_iter().last().ok_or_else(|| {
                        centy_installer::InstallerError::BinaryNotFound(format!(
                            "{} is not installed",
                            proj.display_name()
                        ))
                    })?
                }
            };

            let path = installer.get_binary_path(proj, &version)?;
            println!("{}", path.display());
        }

        Some(Commands::Info) => {
            let paths = installer.paths();
            println!("Centy CLI Information");
            println!("=====================");
            println!("Base directory:     {}", paths.base_dir().display());
            println!("Versions directory: {}", paths.versions_dir().display());
            println!("Bin directory:      {}", paths.bin_dir().display());
            println!();
            println!("Installation path structure:");
            println!("  ~/.centy/versions/<project>/<version>/<binary>");
            println!("  ~/.centy/bin/<binary> -> symlink to latest");
            println!();
            println!("Supported projects:");
            for proj in Project::all() {
                println!(
                    "  - {} ({}) -> {}",
                    proj.name(),
                    proj.display_name(),
                    proj.binary_name()
                );
            }
        }

        Some(Commands::Setup) => {
            println!("Setting up Centy...\n");

            let vm = VersionManager::new("centy-io".to_string())?;

            // Install centy-daemon
            println!("Installing centy-daemon...");
            match vm.get_latest_version(&Project::CentyDaemon).await {
                Ok(version) => {
                    if let Err(e) = installer.install(Project::CentyDaemon, &version).await {
                        eprintln!("  Warning: Failed to install centy-daemon: {}", e);
                    }
                }
                Err(e) => eprintln!("  Warning: Could not fetch centy-daemon version: {}", e),
            }

            // Install centy-tui
            println!("\nInstalling centy-tui...");
            match vm.get_latest_version(&Project::Tui).await {
                Ok(version) => {
                    if let Err(e) = installer.install(Project::Tui, &version).await {
                        eprintln!("  Warning: Failed to install centy-tui: {}", e);
                    }
                }
                Err(e) => eprintln!("  Warning: Could not fetch centy-tui version: {}", e),
            }

            println!("\nSetup complete!");
            println!("Run 'centy' to launch the TUI, or 'centy run daemon' to start the daemon.");
        }
    }

    Ok(())
}

/// Run a project binary, installing it first if needed
async fn run_project(
    installer: &Installer,
    project: Project,
    args: Vec<String>,
) -> centy_installer::Result<()> {
    let binary_name = project.binary_name();

    // Check if installed via symlink
    let symlink_path = installer.paths().symlink_path(binary_name);

    if !symlink_path.exists() {
        println!(
            "{} is not installed. Installing...\n",
            project.display_name()
        );

        let vm = VersionManager::new("centy-io".to_string())?;
        let version = vm.get_latest_version(&project).await?;
        installer.install(project, &version).await?;

        println!();
    }

    // Run the binary
    let status = Command::new(&symlink_path)
        .args(&args)
        .status()
        .map_err(|e| {
            centy_installer::InstallerError::InstallFailed(format!(
                "Failed to run {}: {}",
                binary_name, e
            ))
        })?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn parse_project(s: &str) -> centy_installer::Result<Project> {
    Project::parse(s).ok_or_else(|| centy_installer::InstallerError::ProjectNotFound(s.to_string()))
}
