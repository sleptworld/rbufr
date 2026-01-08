use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use genlib::{
    TableType,
    config::ScanConfig,
    pattern::{TableKind, TableScanner},
    prelude::{BUFRTableB, BUFRTableD},
};
#[cfg(feature = "opera")]
use genlib::{BUFRTableMPH, opera, tables::BitMap};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "gen-ctl")]
#[command(about = "BUFR Table conversion tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory and convert all BUFR tables to MPH format
    Scan {
        /// Input directory containing BUFR CSV files
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for generated .bufrtbl files
        #[arg(short, long)]
        output: PathBuf,

        /// Table type to process: "d", "b", or "all"
        #[arg(short, long, default_value = "all")]
        table_type: String,

        /// Optional config file with custom patterns
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Loader type: "auto" (try all), "wmo" (WMO only), "fr" (French only)
        #[arg(short, long, default_value = "auto")]
        loader: String,
    },
    /// Convert a single BUFR table file
    Convert {
        /// Input CSV file
        #[arg(short, long)]
        input: PathBuf,

        /// Output path (without extension)
        #[arg(short, long)]
        output: PathBuf,

        /// Table type: "d" for Table D, "b" for Table B
        #[arg(short, long)]
        table_type: String,

        /// Loader type: "auto" (try all), "wmo" (WMO only), "fr" (French only)
        #[arg(short, long, default_value = "auto")]
        loader: String,
    },
    /// Print a BUFR table in formatted output
    Print {
        /// Path to .bufrtbl file (without extension)
        #[arg(short, long)]
        input: PathBuf,

        /// Table type: "d" for Table D, "b" for Table B
        #[arg(short, long)]
        table_type: String,

        /// Maximum number of entries to print (optional)
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Generate example configuration file
    GenConfig {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "scan-config.toml")]
        output: PathBuf,
    },
    /// Convert Opera bitmap file to BUFR format
    #[cfg(feature = "opera")]
    ConvertOperaBitmap {
        /// Input Opera bitmap CSV file
        #[arg(short, long)]
        input: PathBuf,

        /// Output path (without extension)
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Print Opera bitmap table
    #[cfg(feature = "opera")]
    PrintOperaBitmap {
        /// Path to Opera bitmap .bufrtbl file (without extension)
        #[arg(short, long)]
        input: PathBuf,

        /// Maximum number of entries to print (optional)
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            input,
            output,
            table_type,
            config,
            loader,
        } => {
            scan_and_convert(&input, &output, &table_type, config.as_deref(), &loader)?;
        }
        Commands::Convert {
            input,
            output,
            table_type,
            loader,
        } => {
            convert_single_file(&input, &output, &table_type, &loader)?;
        }
        Commands::Print {
            input,
            table_type,
            limit,
        } => {
            print_table(&input, &table_type, limit)?;
        }
        Commands::GenConfig { output } => {
            generate_config_file(&output)?;
        }
        #[cfg(feature = "opera")]
        Commands::ConvertOperaBitmap { input, output } => {
            convert_opera_bitmap(&input, &output)?;
        }
        #[cfg(feature = "opera")]
        Commands::PrintOperaBitmap { input, limit } => {
            print_opera_bitmap(&input, limit)?;
        }
    }

    Ok(())
}

fn scan_and_convert(
    input_dir: &Path,
    output_dir: &Path,
    table_type: &str,
    config_path: Option<&Path>,
    loader_type: &str,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    println!("Scanning directory: {}", input_dir.display());
    println!("Output directory: {}", output_dir.display());
    println!("Table type: {}", table_type);
    println!("Loader type: {}", loader_type);
    println!();

    // Create scanner with built-in patterns
    let mut scanner = TableScanner::new();

    // Load custom patterns from config file if provided
    if let Some(config_file) = config_path {
        println!("Loading custom patterns from: {}", config_file.display());
        let config =
            ScanConfig::load_from_file(config_file).context("Failed to load config file")?;

        let custom_patterns = config
            .compile_patterns()
            .context("Failed to compile custom patterns")?;

        println!("Loaded {} custom patterns", custom_patterns.len());
        for pattern in custom_patterns {
            scanner.add_pattern(pattern);
        }
        println!();
    }

    // Display registered patterns
    println!("Registered patterns:");
    for pattern in scanner.patterns() {
        println!("  - {}", pattern.description());
    }
    println!();

    // Determine which table kinds to process
    let kind_filter = match table_type.to_lowercase().as_str() {
        "b" => Some(TableKind::B),
        "d" => Some(TableKind::D),
        "all" => None,
        _ => anyhow::bail!("Invalid table type: {}. Use 'b', 'd', or 'all'", table_type),
    };

    // Scan directory
    let files = scanner
        .scan_directory(input_dir, kind_filter)
        .context("Failed to scan directory")?;

    println!("Found {} matching files", files.len());
    println!();

    let mut processed_count = 0;
    let mut error_count = 0;

    // Group files by table kind for organized output
    let mut table_b_files = Vec::new();
    let mut table_d_files = Vec::new();

    for (path, metadata) in files {
        match metadata.kind {
            TableKind::B => table_b_files.push((path, metadata)),
            TableKind::D => table_d_files.push((path, metadata)),
        }
    }

    // Process Table D files
    if !table_d_files.is_empty() {
        println!("Processing Table D files ({})...", table_d_files.len());
        for (path, metadata) in table_d_files {
            let output_name = metadata.output_name();
            let output_path = output_dir.join(&output_name);

            let file_type = if metadata.is_local { "local" } else { "WMO" };
            print!(
                "  Converting {} ({}) ... ",
                path.file_name().unwrap().to_str().unwrap(),
                file_type
            );

            match convert_table_d(&path, &output_path, loader_type) {
                Ok(_) => {
                    println!("OK -> {}", output_name);
                    processed_count += 1;
                }
                Err(e) => {
                    println!("ERROR: {}", e);
                    error_count += 1;
                }
            }
        }
        println!();
    }

    // Process Table B files
    if !table_b_files.is_empty() {
        println!("Processing Table B files ({})...", table_b_files.len());
        for (path, metadata) in table_b_files {
            let output_name = metadata.output_name();
            let output_path = output_dir.join(&output_name);

            let file_type = if metadata.is_local { "local" } else { "WMO" };
            print!(
                "  Converting {} ({}) ... ",
                path.file_name().unwrap().to_str().unwrap(),
                file_type
            );

            match convert_table_b(&path, &output_path, loader_type) {
                Ok(_) => {
                    println!("OK -> {}", output_name);
                    processed_count += 1;
                }
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    error_count += 1;
                }
            }
        }
        println!();
    }

    println!("Summary:");
    println!("  Successfully processed: {}", processed_count);
    println!("  Errors: {}", error_count);

    if error_count > 0 {
        anyhow::bail!("Conversion completed with {} errors", error_count);
    }

    Ok(())
}

fn convert_single_file(
    input_path: &Path,
    output_path: &Path,
    table_type: &str,
    loader_type: &str,
) -> Result<()> {
    println!(
        "Converting {} to {}",
        input_path.display(),
        output_path.display()
    );
    println!("Loader type: {}", loader_type);

    match table_type.to_lowercase().as_str() {
        "d" => convert_table_d(input_path, output_path, loader_type)?,
        "b" => convert_table_b(input_path, output_path, loader_type)?,
        _ => anyhow::bail!("Invalid table type: {}. Use 'd' or 'b'", table_type),
    }

    println!("Conversion completed successfully!");
    Ok(())
}

type BuildFn = fn(&Path, &Path) -> Result<()>;

fn run_with_fallbacks(
    kind: TableType,
    input_path: &Path,
    output_path: &Path,
    attempts: &[(&str, BuildFn)],
) -> Result<()> {
    let mut errors = Vec::new();
    for (label, build_fn) in attempts {
        match build_fn(input_path, output_path) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(format!("{label} failed: {err:#}")),
        }
    }

    Err(anyhow!(
        "all {:?} loaders failed:\n{}",
        kind,
        errors.join("\n---\n")
    ))
}

fn build_wmo_d(input_path: &Path, output_path: &Path) -> Result<()> {
    let loader = genlib::wmo::TableLoader::<genlib::wmo::WMODTableLoader>::default();
    BUFRTableD::build_from_csv(loader, input_path, output_path).map(|_| ())
}

fn build_fr_d(input_path: &Path, output_path: &Path) -> Result<()> {
    let loader = genlib::fr::FRDTableLoader::default();
    BUFRTableD::build_from_csv(loader, input_path, output_path).map(|_| ())
}

fn convert_table_d(input_path: &Path, output_path: &Path, loader_type: &str) -> Result<()> {
    match loader_type.to_lowercase().as_str() {
        "wmo" => {
            // WMO only
            build_wmo_d(input_path, output_path)
        }
        "fr" => {
            // French only
            build_fr_d(input_path, output_path)
        }
        "auto" => {
            // Try all loaders
            const ATTEMPTS: &[(&str, BuildFn)] = &[
                ("WMO Table D loader", build_wmo_d),
                ("FR Table D loader", build_fr_d),
            ];
            run_with_fallbacks(TableType::D, input_path, output_path, ATTEMPTS)
        }
        _ => anyhow::bail!(
            "Invalid loader type: {}. Use 'auto', 'wmo', or 'fr'",
            loader_type
        ),
    }
}

fn build_wmo_b(input_path: &Path, output_path: &Path) -> Result<()> {
    let loader = genlib::wmo::TableLoader::<genlib::wmo::WMOBTableLoader>::default();
    BUFRTableB::build_from_csv(loader, input_path, output_path).map(|_| ())
}

fn build_fr_b(input_path: &Path, output_path: &Path) -> Result<()> {
    let loader = genlib::fr::FRBTableLoader::default();
    BUFRTableB::build_from_csv(loader, input_path, output_path).map(|_| ())
}

fn convert_table_b(input_path: &Path, output_path: &Path, loader_type: &str) -> Result<()> {
    match loader_type.to_lowercase().as_str() {
        "wmo" => {
            // WMO only
            build_wmo_b(input_path, output_path)
        }
        "fr" => {
            // French only
            build_fr_b(input_path, output_path)
        }
        "auto" => {
            // Try all loaders
            const ATTEMPTS: &[(&str, BuildFn)] = &[
                ("WMO Table B loader", build_wmo_b),
                ("FR Table B loader", build_fr_b),
            ];
            run_with_fallbacks(TableType::B, input_path, output_path, ATTEMPTS)
        }
        _ => anyhow::bail!(
            "Invalid loader type: {}. Use 'auto', 'wmo', or 'fr'",
            loader_type
        ),
    }
}

fn print_table(input_path: &Path, table_type: &str, limit: Option<usize>) -> Result<()> {
    match table_type.to_lowercase().as_str() {
        "d" => print_table_d(input_path, limit)?,
        "b" => print_table_b(input_path, limit)?,
        _ => anyhow::bail!("Invalid table type: {}. Use 'd' or 'b'", table_type),
    }

    Ok(())
}

fn print_table_d(input_path: &Path, limit: Option<usize>) -> Result<()> {
    println!("Loading Table D from: {}", input_path.display());

    let table: BUFRTableD = BUFRTableD::load_from_disk(input_path)?;
    let entries = table.get_all_entries();

    println!("\nTable D Entries (Total: {})", entries.len());
    println!("{}", "=".repeat(140));
    println!(
        "{:<7} | {:<50} | {:<12} | {}",
        "FXY", "Title", "Status", "FXY Chain"
    );
    println!("{}", "-".repeat(140));

    let display_entries = if let Some(max) = limit {
        &entries[..entries.len().min(max)]
    } else {
        &entries[..]
    };

    for entry in display_entries {
        println!("{}", entry);
    }

    if let Some(max) = limit {
        if entries.len() > max {
            println!("\n... ({} more entries omitted)", entries.len() - max);
        }
    }

    Ok(())
}

fn print_table_b(input_path: &Path, limit: Option<usize>) -> Result<()> {
    println!("Loading Table B from: {}", input_path.display());

    let table: BUFRTableB = BUFRTableB::load_from_disk(input_path)?;
    let entries = table.get_all_entries();

    println!("\nTable B Entries (Total: {})", entries.len());
    println!("{}", "=".repeat(120));
    println!(
        "{:<7} | {:<40} | {:<15} | {:<5} | {:<8} | {:<8} | {}",
        "FXY", "Element Name", "Unit", "Scale", "Ref Val", "Width", "Status"
    );
    println!("{}", "-".repeat(120));

    let display_entries = if let Some(max) = limit {
        &entries[..entries.len().min(max)]
    } else {
        &entries[..]
    };

    for entry in display_entries {
        println!("{}", entry);
    }

    if let Some(max) = limit {
        if entries.len() > max {
            println!("\n... ({} more entries omitted)", entries.len() - max);
        }
    }

    Ok(())
}

fn generate_config_file(output_path: &Path) -> Result<()> {
    println!(
        "Generating example configuration file: {}",
        output_path.display()
    );

    // Create example configuration
    let config = ScanConfig::default_example();

    // Save to file
    config
        .save_to_file(output_path)
        .context("Failed to save configuration file")?;

    println!("Configuration file generated successfully!");
    println!();
    println!("The configuration file contains example patterns for:");
    for pattern_config in &config.patterns {
        println!("  - {}", pattern_config.name);
    }
    println!();
    println!("Edit this file to add your own custom patterns.");
    println!(
        "Use it with: gen-ctl scan -i <input> -o <output> -c {}",
        output_path.display()
    );

    Ok(())
}

#[cfg(feature = "opera")]
fn convert_opera_bitmap(input_path: &Path, output_path: &Path) -> Result<()> {
    println!(
        "Converting Opera bitmap from {} to {}",
        input_path.display(),
        output_path.display()
    );

    let loader = opera::TableLoader {};
    BUFRTableMPH::<BitMap>::build_from_csv(loader, input_path, output_path)?;

    println!("Conversion completed successfully!");
    Ok(())
}

#[cfg(feature = "opera")]
fn print_opera_bitmap(input_path: &Path, limit: Option<usize>) -> Result<()> {
    println!("Loading Opera bitmap from: {}", input_path.display());

    let table = BUFRTableMPH::<BitMap>::load_from_disk(input_path)?;
    let entries = table.get_all_entries();

    println!("\nOpera Bitmap Entries (Total: {})", entries.len());
    println!("{}", "=".repeat(60));
    println!("{:<10} | {}", "FXY", "Depth");
    println!("{}", "-".repeat(60));

    let display_entries = if let Some(max) = limit {
        &entries[..entries.len().min(max)]
    } else {
        &entries[..]
    };

    for entry in display_entries {
        println!("{}", entry);
    }

    if let Some(max) = limit {
        if entries.len() > max {
            println!("\n... ({} more entries omitted)", entries.len() - max);
        }
    }

    Ok(())
}
