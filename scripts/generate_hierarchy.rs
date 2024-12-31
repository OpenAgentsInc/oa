use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn build_tree(paths: &[PathBuf], root: &Path) -> String {
    println!("Starting tree build with {} paths", paths.len());
    let mut output = String::new();
    
    // First, let's print what we're working with
    println!("Paths to process:");
    for path in paths {
        println!("  {}", path.strip_prefix(root).unwrap_or(path).display());
    }

    for path in paths {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let depth = relative.components().count();
        
        // Simple indentation based on depth
        let indent = "  ".repeat(depth);
        output.push_str(&format!("{}├── {}\n", 
            indent,
            relative.components().last()
                .unwrap_or_default()
                .as_os_str()
                .to_string_lossy()
        ));
    }

    output
}

fn main() -> io::Result<()> {
    println!("Starting hierarchy generation...");
    
    let root_dir = Path::new(".").canonicalize()?;
    println!("Root directory: {:?}", root_dir);

    // Create docs directory if it doesn't exist
    fs::create_dir_all(root_dir.join("docs"))?;
    println!("Created docs directory");

    // Common patterns to ignore
    let ignore_patterns = [
        ".git",
        "target",
        "Cargo.lock",
        ".DS_Store",
        "docs/hierarchy.md",
        "node_modules",
    ];

    println!("Collecting paths...");
    // Collect all paths, filtering out ignored ones
    let mut paths: Vec<PathBuf> = WalkDir::new(&root_dir)
        .into_iter()
        .filter_map(|e| {
            match e {
                Ok(entry) => Some(entry),
                Err(err) => {
                    println!("Error walking directory: {:?}", err);
                    None
                }
            }
        })
        .filter(|e| {
            let path = e.path();
            let relative = path.strip_prefix(&root_dir).unwrap();
            let path_str = relative.to_string_lossy();
            
            // Skip ignored patterns
            let should_include = !ignore_patterns.iter().any(|pattern| path_str.contains(pattern));
            if !should_include {
                println!("Ignoring: {}", path_str);
            }
            should_include
        })
        .map(|e| e.path().to_owned())
        .collect();

    println!("Found {} paths", paths.len());

    // Sort paths
    paths.sort();
    println!("Sorted paths");

    // Build the tree
    println!("Building tree...");
    let tree = build_tree(&paths, &root_dir);
    println!("Tree built successfully");

    // Create the output markdown
    let output = format!("# Project File Hierarchy\n\n```\n{}```\n", tree);

    // Write to docs/hierarchy.md
    let output_path = root_dir.join("docs/hierarchy.md");
    println!("Writing to {:?}", output_path);
    let mut file = File::create(&output_path)?;
    file.write_all(output.as_bytes())?;

    println!("Updated file hierarchy written to: docs/hierarchy.md");
    Ok(())
}