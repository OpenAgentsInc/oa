use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

// Custom error type to handle both IO and Ignore errors
#[derive(Debug)]
enum Error {
    Io(io::Error),
    Ignore(ignore::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ignore::Error> for Error {
    fn from(err: ignore::Error) -> Error {
        Error::Ignore(err)
    }
}

fn get_ignore_patterns(root_dir: &Path) -> Result<Gitignore, Error> {
    let mut builder = GitignoreBuilder::new(root_dir);
    
    // Add common Rust ignores
    builder.add_line(None, "target/**").map_err(Error::Ignore)?;
    builder.add_line(None, "**/*.rs.bk").map_err(Error::Ignore)?;
    builder.add_line(None, "Cargo.lock").map_err(Error::Ignore)?;
    
    // Add .gitignore patterns if they exist
    let gitignore_path = root_dir.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path).map_err(Error::Ignore)?;
    }
    
    Ok(builder.build().map_err(Error::Ignore)?)
}

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

fn main() -> Result<(), Error> {
    let root_dir = Path::new(".").canonicalize()?;
    let gitignore = get_ignore_patterns(&root_dir)?;

    // Create docs directory if it doesn't exist
    fs::create_dir_all(root_dir.join("docs"))?;

    // Collect all paths, filtering out ignored ones
    let mut paths: Vec<PathBuf> = WalkDir::new(&root_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            let relative = path.strip_prefix(&root_dir).unwrap();
            
            // Skip the .git directory and the docs/hierarchy.md file
            if relative.starts_with(".git") || relative == Path::new("docs/hierarchy.md") {
                return false;
            }

            // Use gitignore patterns
            !gitignore.matched_path_or_any_parents(relative, false).is_ignore()
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