use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn build_tree(paths: &[PathBuf], root: &Path) -> String {
    let mut output = String::new();
    let mut current_level = 0;
    let mut is_last_at_level = vec![];

    for (i, path) in paths.iter().enumerate() {
        let relative = path.strip_prefix(root).unwrap();
        let depth = relative.components().count();

        // Adjust is_last tracking
        if depth > current_level {
            while is_last_at_level.len() < depth - 1 {
                is_last_at_level.push(false);
            }
        } else {
            while is_last_at_level.len() >= depth {
                is_last_at_level.pop();
            }
        }

        // Check if this is the last item at this level
        let is_last = i == paths.len() - 1 || {
            let next = paths.get(i + 1).and_then(|p| p.strip_prefix(root).ok());
            next.map_or(true, |next| next.components().count() <= depth)
        };

        // Build the line prefix
        let mut prefix = String::new();
        for &last in &is_last_at_level {
            prefix.push_str(if last { "    " } else { "│   " });
        }
        prefix.push_str(if is_last { "└── " } else { "├── " });

        // Add the line
        output.push_str(&format!("{}{}\n", prefix, relative.components().last().unwrap().as_os_str().to_string_lossy()));

        is_last_at_level.push(is_last);
        current_level = depth;
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
    let tree = build_tree(&paths, &root_dir);
    println!("Built tree structure");

    // Create the output markdown
    let output = format!("# Project File Hierarchy\n\n```\n{}```\n", tree);

    // Write to docs/hierarchy.md
    let mut file = File::create(root_dir.join("docs/hierarchy.md"))?;
    file.write_all(output.as_bytes())?;

    println!("Updated file hierarchy written to: docs/hierarchy.md");
    Ok(())
}