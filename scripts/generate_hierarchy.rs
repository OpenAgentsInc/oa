use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn build_tree(paths: &[PathBuf], root: &Path) -> String {
    let mut output = String::new();

    for path in paths {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let depth = relative.components().count();
        
        // Simple indentation based on depth
        let indent = "  ".repeat(depth);
        
        // Get the filename, or use an empty string if we can't get it
        let filename = relative
            .components()
            .last()
            .map(|c| c.as_os_str().to_string_lossy())
            .unwrap_or_default();
            
        output.push_str(&format!("{}├── {}\n", indent, filename));
    }

    output
}

fn main() -> io::Result<()> {
    println!("Starting hierarchy generation...");
    
    let root_dir = Path::new(".").canonicalize()?;

    // Create docs directory if it doesn't exist
    fs::create_dir_all(root_dir.join("docs"))?;

    // Common patterns to ignore
    let ignore_patterns = [
        ".git",
        "target",
        "Cargo.lock",
        ".DS_Store",
        "docs/hierarchy.md",
        "node_modules",
    ];

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
            !ignore_patterns.iter().any(|pattern| path_str.contains(pattern))
        })
        .map(|e| e.path().to_owned())
        .collect();

    // Sort paths
    paths.sort();

    // Build the tree
    let tree = build_tree(&paths, &root_dir);

    // Create the output markdown
    let output = format!("# Project File Hierarchy\n\n```\n{}```\n", tree);

    // Write to docs/hierarchy.md
    let mut file = File::create(root_dir.join("docs/hierarchy.md"))?;
    file.write_all(output.as_bytes())?;

    println!("Updated file hierarchy written to: docs/hierarchy.md");
    Ok(())
}