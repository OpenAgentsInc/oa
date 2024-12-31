#!/bin/bash

# Deepseek Test Fixer Script
# This script uses Deepseek API to analyze and fix failing tests

# Enable debug mode
set -x

# Configuration
LOG_FILE="docs/deepseek-fixer-log.md"
MAX_ITERATIONS=5

# Check if DEEPSEEK_API_KEY is set
if [ -z "${DEEPSEEK_API_KEY}" ]; then
    echo "Error: DEEPSEEK_API_KEY environment variable is not set"
    exit 1
fi

# Initialize log file
if [ ! -f "$LOG_FILE" ]; then
    mkdir -p docs
    echo "# Deepseek Test Fixer Log\n\n" > "$LOG_FILE"
fi

# Function to extract relevant error messages
extract_errors() {
    echo "$1" | grep -A 2 "error\[E[0-9]*\]:\|error: \|thread.*panicked\|FAILED" || true
}

# Function to call Deepseek API
call_deepseek() {
    local prompt="$1"
    local response
    
    echo "Calling Deepseek API..."
    
    response=$(curl -s "https://api.deepseek.com/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
        -d "{
            \"model\": \"deepseek-coder-33b-instruct\",
            \"messages\": [{\"role\": \"user\", \"content\": $(echo "$prompt" | jq -R -s '.')}],
            \"temperature\": 0.1,
            \"max_tokens\": 4000
        }")
    
    echo "Raw API Response: $response"
    
    if echo "$response" | jq -e '.choices[0].message.content' >/dev/null 2>&1; then
        echo "$response" | jq -r '.choices[0].message.content'
    else
        echo "ERROR: Invalid response from Deepseek API"
        echo "Response: $response"
        return 1
    fi
}

# Update hierarchy
echo "Updating hierarchy..."
./scripts/generate_hierarchy.sh

# Main loop
iteration=1

while [ $iteration -le $MAX_ITERATIONS ]; do
    echo -e "\n=== Iteration $iteration of $MAX_ITERATIONS ===\n"
    
    # Run tests and capture ALL output
    echo "Running tests..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    echo "Full test output:"
    echo "$test_output"
    
    # Process test results
    echo -e "\nExtracting test errors..."
    error_output=$(extract_errors "$test_output")
    echo "Extracted errors:"
    echo "$error_output"
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        exit 0
    fi

    if [ -z "$error_output" ]; then
        echo "No error output found but tests failed. Using full test output..."
        error_output="$test_output"
    fi

    # Get project hierarchy
    echo "Reading hierarchy file..."
    hierarchy_content=$(cat docs/hierarchy.md)

    # Get files to examine
    echo -e "\nAnalyzing test failures..."
    files_json=$(call_deepseek "You are a Rust expert. Given these test failures:

$error_output

And this project hierarchy:
$hierarchy_content

Return ONLY a JSON array of file paths that need to be examined to fix these errors. Example: [\"src/file1.rs\",\"src/file2.rs\"]. Return ONLY the JSON array.")

    echo "Received files_json: $files_json"

    # Parse files or fallback to error messages
    if ! files_array=($(echo "$files_json" | jq -r '.[]' 2>/dev/null)); then
        echo "Using files from error messages..."
        mapfile -t files_array < <(echo "$error_output" | grep -o 'src/[^:]*' | sort -u)
    fi

    echo -e "\nFiles to examine: ${files_array[*]}"
    
    if [ ${#files_array[@]} -eq 0 ]; then
        echo "No files found to examine. Trying to extract from full test output..."
        mapfile -t files_array < <(echo "$test_output" | grep -o 'src/[^:]*' | sort -u)
        
        if [ ${#files_array[@]} -eq 0 ]; then
            echo "Still no files found. Examining default locations..."
            files_array=("src/lib.rs" "src/main.rs")
        fi
    fi
    
    # Process each file
    for file in "${files_array[@]}"; do
        echo -e "\nAnalyzing $file..."
        
        if [ ! -f "$file" ]; then
            echo "File $file not found, skipping..."
            continue
        fi
        
        file_content=$(cat "$file")
        
        # Check if file needs changes
        echo "Asking Deepseek about necessary changes..."
        response=$(call_deepseek "You are a Rust expert. Given this file:

$file_content

And these test failures:

$error_output

Does this file need changes to fix the failing tests? If yes, provide the complete updated file content. If no, respond with 'NO_CHANGES_NEEDED'.

Format your response to start with either 'CHANGES:' followed by the new content, or 'NO_CHANGES_NEEDED'.")
        
        echo "Received response from Deepseek: $response"
        
        if [[ "$response" == NO_CHANGES_NEEDED* ]]; then
            echo "No changes needed for $file"
            continue
        fi
        
        if [[ "$response" == CHANGES:* ]]; then
            # Get new content
            new_content="${response#CHANGES:}"
            
            # Get explanation
            explanation=$(call_deepseek "Explain in one line what changes were made to $file and why they fix the failing tests.")
            
            echo "Change explanation: $explanation"
            
            # Log changes
            {
                echo -e "\n## $(date '+%Y-%m-%d %H:%M:%S')\n"
                echo "File: $file"
                echo "Changes: $explanation"
                echo "\`\`\`diff"
                diff -u "$file" <(echo "$new_content") || true
                echo "\`\`\`"
            } >> "$LOG_FILE"
            
            # Update file and commit
            echo "$new_content" > "$file"
            git add "$file" "$LOG_FILE"
            git commit -m "fix($file): $explanation" -n
        fi
    done
    
    ((iteration++))
done

echo "Maximum iterations reached. Some tests may still be failing."
exit 1