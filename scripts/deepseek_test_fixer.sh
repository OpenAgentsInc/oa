#!/bin/bash

# Deepseek Test Fixer Script
# This script uses Deepseek API to analyze and fix failing tests

DEEPSEEK_API_KEY="sk-dfb76aaf6b3545b2bc8128ba90f87c33"
DEEPSEEK_API_URL="https://api.deepseek.com/chat/completions"
LOG_FILE="docs/deepseek-fixer-log.md"

# Initialize or create the log file if it doesn't exist
if [ ! -f "$LOG_FILE" ]; then
    mkdir -p docs
    echo "# Deepseek Test Fixer Log\n\n" > "$LOG_FILE"
fi

# Function to call Deepseek API
call_deepseek() {
    local prompt="$1"
    curl -s "$DEEPSEEK_API_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
        -d "{
            \"model\": \"deepseek-chat\",
            \"messages\": [{\"role\": \"user\", \"content\": \"$prompt\"}],
            \"stream\": false
        }" | jq -r '.choices[0].message.content'
}

# Update hierarchy
echo "Updating hierarchy..."
./scripts/generate_hierarchy.sh

# Function to capture test output and errors
run_tests() {
    cargo test
    return $?
}

# Main loop
MAX_ITERATIONS=5
iteration=1

while [ $iteration -le $MAX_ITERATIONS ]; do
    echo "Iteration $iteration of $MAX_ITERATIONS"
    
    # Run tests and capture output
    echo "Running tests..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    echo "Test output:"
    echo "$test_output"
    echo "Test exit code: $test_exit_code"
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        exit 0
    fi

    # Get hierarchy content
    hierarchy_content=$(cat docs/hierarchy.md)

    # Ask Deepseek for files to examine
    prompt="Given these test failures:\n$test_output\n\nAnd this project hierarchy:\n$hierarchy_content\n\nReturn a JSON array of file paths that are most likely to need examination to fix these failing tests. Format: [\"path/to/file1.rs\", \"path/to/file2.rs\"]"
    
    echo "Asking Deepseek for files to examine..."
    files_to_check=$(call_deepseek "$prompt")
    echo "Deepseek suggested files: $files_to_check"
    
    # Remove brackets and quotes, split into array
    files_array=($(echo "$files_to_check" | tr -d '[]"' | tr ',' '\n'))
    
    for file in "${files_array[@]}"; do
        echo "Analyzing $file..."
        
        # Skip if file doesn't exist
        if [ ! -f "$file" ]; then
            echo "File $file not found, skipping..."
            continue
        fi
        
        file_content=$(cat "$file")
        
        # Ask Deepseek if file needs changes
        prompt="Given this file content:\n$file_content\n\nAnd these test failures:\n$test_output\n\nDoes this file need changes to fix the failing tests? If yes, provide the complete updated file content. If no, respond with 'NO_CHANGES_NEEDED'. Format your response to start with either 'CHANGES:' followed by the new content, or 'NO_CHANGES_NEEDED'"
        
        echo "Asking Deepseek if file needs changes..."
        response=$(call_deepseek "$prompt")
        echo "Deepseek response starts with: ${response:0:50}..."
        
        if [[ $response == NO_CHANGES_NEEDED* ]]; then
            echo "No changes needed for $file"
            continue
        fi
        
        if [[ $response == CHANGES:* ]]; then
            # Extract new content (remove "CHANGES:" prefix)
            new_content="${response#CHANGES:}"
            
            # Get explanation from Deepseek
            prompt="Explain the changes you just suggested for $file in one line"
            explanation=$(call_deepseek "$prompt")
            
            echo "Deepseek suggested changes with explanation: $explanation"
            
            # Log the changes
            echo -e "\n## $(date '+%Y-%m-%d %H:%M:%S')\n" >> "$LOG_FILE"
            echo "File: $file" >> "$LOG_FILE"
            echo "Changes: $explanation" >> "$LOG_FILE"
            
            # Update the file
            echo "$new_content" > "$file"
            
            # Commit changes
            git add "$file"
            git commit -m "$explanation" -n
        fi
    done
    
    # Run tests again to check progress
    echo "Running tests again..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    echo "Test output:"
    echo "$test_output"
    echo "Test exit code: $test_exit_code"
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        exit 0
    fi
    
    ((iteration++))
done

echo "Maximum iterations reached. Some tests may still be failing."
exit 1