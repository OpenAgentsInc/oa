#!/bin/bash

echo "Starting hierarchy generation..."

# Create docs directory if it doesn't exist
mkdir -p docs

# Get the absolute path of the current directory
ROOT_DIR=$(pwd)

# Create a temporary file to store the tree
TEMP_FILE=$(mktemp)

# Find all files and directories, excluding common patterns
find . -name .git -prune -o \
       -name target -prune -o \
       -name Cargo.lock -prune -o \
       -name .DS_Store -prune -o \
       -name node_modules -prune -o \
       -name "docs/hierarchy.md" -prune -o \
       -type f -o -type d | sort | while read -r file; do
    # Skip the current directory entry
    if [ "$file" = "." ]; then
        continue
    fi
    
    # Calculate the depth by counting slashes (subtract 1 for leading ./)
    depth=$(echo "$file" | tr -cd '/' | wc -c)
    indent=$(printf '%*s' "$((depth * 2))" '')
    
    # Get the basename of the file/directory
    name=$(basename "$file")
    
    # Output the tree line
    echo "${indent}├── ${name}"
done > "$TEMP_FILE"

# Create the final markdown file
{
    echo "# Project File Hierarchy"
    echo
    echo "\`\`\`"
    cat "$TEMP_FILE"
    echo "\`\`\`"
} > docs/hierarchy.md

# Clean up
rm "$TEMP_FILE"

echo "Updated file hierarchy written to: docs/hierarchy.md"