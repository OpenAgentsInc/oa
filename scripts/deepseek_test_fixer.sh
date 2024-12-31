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

# Function to escape string for JSON
escape_json() {
    echo "$1" | jq -R -s '.'
}

# Function to call Deepseek API and handle response
call_deepseek() {
    local prompt="$1"
    echo -e "\nSending to Deepseek:\n---\n$prompt\n---"
    
    # Escape the prompt for JSON
    local escaped_prompt=$(escape_json "$prompt")
    
    local response
    response=$(curl -s "$DEEPSEEK_API_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
        -d "{
            \"model\": \"deepseek-chat\",
            \"messages\": [{\"role\": \"user\", \"content\": ${escaped_prompt}}],
            \"stream\": false
        }")
    
    if echo "$response" | jq -e '.choices[0].message.content' >/dev/null 2>&1; then
        local content=$(echo "$response" | jq -r '.choices[0].message.content')
        echo -e "\nDeepseek response:\n---\n$content\n---"
        echo "$content"
    else
        echo "ERROR: Invalid response from Deepseek API"
        echo "Full response: $response"
        return 1
    fi
}

# Update hierarchy for context
echo "Updating hierarchy..."
./scripts/generate_hierarchy.sh

# Main loop
MAX_ITERATIONS=5
iteration=1

while [ $iteration -le $MAX_ITERATIONS ]; do
    echo -e "\n=== Iteration $iteration of $MAX_ITERATIONS ===\n"
    
    # Run tests and capture output
    echo "Running tests..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    # Extract and display only the error messages
    echo -e "\nTest errors:"
    error_output=$(extract_errors "$test_output")
    echo "$error_output"
    echo "Test exit code: $test_exit_code"
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        exit 0
    fi

    # Get hierarchy content
    hierarchy_content=$(cat docs/hierarchy.md)

    # Ask Deepseek for files to examine
    echo -e "\nAsking Deepseek to analyze errors..."
    prompt="You are a Rust expert. Given these test failures:

$error_output

And this project hierarchy:
$hierarchy_content

Return ONLY a JSON array of file paths that need to be examined to fix these errors. Example format: [\"src/file1.rs\",\"src/file2.rs\"]. Return ONLY the JSON array, no other text or explanation."
    
    files_to_check=$(call_deepseek "$prompt")
    if [[ $files_to_check == ERROR:* ]]; then
        echo -e "\nFailed to get response from Deepseek:"
        echo "$files_to_check"
        echo "Skipping iteration..."
        ((iteration++))
        continue
    fi
    
    # Validate JSON array
    if ! echo "$files_to_check" | jq -e 'if type == "array" then . else null end' >/dev/null 2>&1; then
        echo -e "\nInvalid JSON array from Deepseek:"
        echo "$files_to_check"
        echo "Skipping iteration..."
        ((iteration++))
        continue
    fi
    
    echo -e "\nProcessing files: $files_to_check"
    
    # Process each file from the JSON array
    echo "$files_to_check" | jq -r '.[]' | while read -r file; do
        echo -e "\nAnalyzing $file..."
        
        if [ ! -f "$file" ]; then
            echo "File $file not found, skipping..."
            continue
        fi
        
        file_content=$(cat "$file")
        
        # Ask Deepseek if file needs changes
        echo "Asking Deepseek about necessary changes..."
        prompt="You are a Rust expert. Given this file content:

$file_content

And these test failures:

$error_output

Does this file need changes to fix the failing tests? If yes, provide the complete updated file content. If no, respond with 'NO_CHANGES_NEEDED'.

Format your response to start with either 'CHANGES:' followed by the new content, or 'NO_CHANGES_NEEDED'. If providing changes, ensure the code is complete and properly formatted."
        
        response=$(call_deepseek "$prompt")
        if [[ $response == ERROR:* ]]; then
            echo "Failed to get response from Deepseek for $file. Skipping..."
            continue
        fi
        
        if [[ $response == NO_CHANGES_NEEDED* ]]; then
            echo "No changes needed for $file"
            continue
        fi
        
        if [[ $response == CHANGES:* ]]; then
            # Extract new content (remove "CHANGES:" prefix)
            new_content="${response#CHANGES:}"
            
            # Get explanation from Deepseek
            prompt="Explain in one line what changes you made to $file and why they fix the failing tests."
            explanation=$(call_deepseek "$prompt")
            if [[ $explanation == ERROR:* ]]; then
                explanation="Updated file contents to fix failing tests"
            fi
            
            echo -e "\nDeepseek suggested changes with explanation: $explanation"
            
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