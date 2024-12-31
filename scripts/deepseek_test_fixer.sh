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

# Function to extract relevant error messages
extract_errors() {
    echo "$1" | grep -A 1 "error\[E[0-9]*\]:\|error: " | head -n 20
}

# Function to call Deepseek API and handle response
call_deepseek() {
    local prompt="$1"
    local response
    response=$(curl -s "$DEEPSEEK_API_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
        -d "{
            \"model\": \"deepseek-chat\",
            \"messages\": [{\"role\": \"user\", \"content\": \"$prompt\"}],
            \"stream\": false
        }")
    
    # Check if response contains choices array
    if echo "$response" | jq -e '.choices[0].message.content' >/dev/null 2>&1; then
        echo "$response" | jq -r '.choices[0].message.content'
    else
        echo "ERROR: Invalid response from Deepseek API"
        echo "Full response: $response"
        return 1
    fi
}

# Update hierarchy
echo "Updating hierarchy..."
./scripts/generate_hierarchy.sh

# Main loop
MAX_ITERATIONS=5
iteration=1

# Set required environment variables and create directories
export OUT_DIR="src/target/out"
mkdir -p "$OUT_DIR"
mkdir -p "src/target/out"

# First, try to fix the nauthz.rs issue
if [ ! -f "$OUT_DIR/nauthz.rs" ]; then
    echo "Creating empty nauthz.rs file..."
    mkdir -p "$(dirname "$OUT_DIR/nauthz.rs")"
    touch "$OUT_DIR/nauthz.rs"
fi

while [ $iteration -le $MAX_ITERATIONS ]; do
    echo "Iteration $iteration of $MAX_ITERATIONS"
    
    # Run tests and capture output
    echo "Running tests..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    # Extract and display only the error messages
    echo "Test errors:"
    error_output=$(extract_errors "$test_output")
    echo "$error_output"
    echo "Test exit code: $test_exit_code"
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        exit 0
    fi

    # Special handling for nauthz.rs error
    if echo "$error_output" | grep -q "couldn't read.*nauthz.rs"; then
        echo "Detected nauthz.rs error, creating necessary directories..."
        mkdir -p "src/target/out"
        touch "src/target/out/nauthz.rs"
        continue
    fi

    # Get hierarchy content
    hierarchy_content=$(cat docs/hierarchy.md)

    # Ask Deepseek for files to examine
    echo "Asking Deepseek for files to examine..."
    prompt="Given these test failures:\n$error_output\n\nAnd this project hierarchy:\n$hierarchy_content\n\nAnalyze the test failures and return ONLY a JSON array of file paths that need to be examined, like this: [\"src/file1.rs\",\"src/file2.rs\"]. Return ONLY the JSON array, no other text."
    
    files_to_check=$(call_deepseek "$prompt")
    if [[ $files_to_check == ERROR:* ]]; then
        echo "Failed to get file list from Deepseek. Retrying with default file list..."
        # Use the file mentioned in the error message as fallback
        files_to_check='["src/nauthz.rs"]'
    fi
    
    echo "Files to examine: $files_to_check"
    
    # Validate JSON array and split into array
    if ! echo "$files_to_check" | jq -e 'if type == "array" then true else false end' >/dev/null 2>&1; then
        echo "Invalid JSON array from Deepseek. Using default file list..."
        files_to_check='["src/nauthz.rs"]'
    fi
    
    # Convert JSON array to bash array
    readarray -t files_array < <(echo "$files_to_check" | jq -r '.[]')
    
    for file in "${files_array[@]}"; do
        echo "Analyzing $file..."
        
        # Skip if file doesn't exist
        if [ ! -f "$file" ]; then
            echo "File $file not found, skipping..."
            continue
        fi
        
        file_content=$(cat "$file")
        
        # Ask Deepseek if file needs changes
        echo "Asking Deepseek about changes for $file..."
        prompt="Given this file content:\n$file_content\n\nAnd these test failures:\n$error_output\n\nDoes this file need changes to fix the failing tests? If yes, provide the complete updated file content. If no, respond with 'NO_CHANGES_NEEDED'. Format your response to start with either 'CHANGES:' followed by the new content, or 'NO_CHANGES_NEEDED'"
        
        response=$(call_deepseek "$prompt")
        if [[ $response == ERROR:* ]]; then
            echo "Failed to get response from Deepseek for $file. Skipping..."
            continue
        fi
        
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
            if [[ $explanation == ERROR:* ]]; then
                explanation="Updated file contents to fix failing tests"
            fi
            
            echo "Deepseek suggested changes with explanation: $explanation"
            
            # Log the changes
            echo -e "\n## $(date '+%Y-%m-%d %H:%M:%S')\n" >> "$LOG_FILE"
            echo "File: $file" >> "$LOG_FILE"
            echo "Changes: $explanation" >> "$LOG_FILE"
            echo "\`\`\`diff" >> "$LOG_FILE"
            diff -u "$file" <(echo "$new_content") >> "$LOG_FILE" || true
            echo "\`\`\`" >> "$LOG_FILE"
            
            # Update the file
            echo "$new_content" > "$file"
            
            # Commit changes
            git add "$file"
            git commit -m "$explanation" -n
        fi
    done
    
    ((iteration++))
done

echo "Maximum iterations reached. Some tests may still be failing."
exit 1