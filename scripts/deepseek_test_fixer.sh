#!/bin/bash

# Deepseek Test Fixer Script
# This script uses Deepseek API to analyze and fix failing tests

# Configuration
LOG_FILE="docs/deepseek-fixer-log.md"
MAX_ITERATIONS=5

# Check if DEEPSEEK_API_KEY is set
if [ -z "${DEEPSEEK_API_KEY}" ]; then
    echo "Error: DEEPSEEK_API_KEY environment variable is not set"
    exit 1
fi

# Initialize log file
mkdir -p docs
echo -e "# Deepseek Test Fixer Log\n\nStarted analysis at $(date '+%Y-%m-%d %H:%M:%S')\n" > "$LOG_FILE"

# Function to extract relevant error messages
extract_errors() {
    echo "$1" | grep -A 2 "error\[E[0-9]*\]:\|error: \|thread.*panicked\|FAILED" || true
}

# Function to extract files from error messages
extract_files() {
    local input="$1"
    local files=""
    while IFS= read -r line; do
        if [[ $line =~ src/[^:]*\.rs ]]; then
            file="${BASH_REMATCH[0]}"
            if [[ ! $files =~ $file ]]; then
                files="$files $file"
            fi
        fi
    done <<< "$input"
    echo "$files"
}

# Function to call Deepseek API
call_deepseek() {
    local prompt="$1"
    local response
    
    response=$(curl -s "https://api.deepseek.com/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
        -d "{
            \"model\": \"deepseek-chat\",
            \"messages\": [{\"role\": \"user\", \"content\": $(echo "$prompt" | jq -R -s '.')}],
            \"temperature\": 0.1,
            \"max_tokens\": 4000
        }")
    
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
    echo -e "\n## Iteration $iteration of $MAX_ITERATIONS\n" >> "$LOG_FILE"
    
    # Run tests
    echo "Running tests..."
    test_output=$(cargo test 2>&1)
    test_exit_code=$?
    
    # Process test results
    error_output=$(extract_errors "$test_output")
    
    if [ $test_exit_code -eq 0 ]; then
        echo "All tests passing! Exiting..."
        echo "\n✅ All tests passing!" >> "$LOG_FILE"
        exit 0
    fi

    # Get project hierarchy
    hierarchy_content=$(cat docs/hierarchy.md)

    # Get files to examine
    echo -e "\nAnalyzing test failures..."
    files_json=$(call_deepseek "You are a Rust expert. Given these test failures:

$error_output

And this project hierarchy:
$hierarchy_content

Return ONLY a JSON array of file paths that need to be examined to fix these errors. Example: [\"src/file1.rs\",\"src/file2.rs\"]. Return ONLY the JSON array.")

    # Parse files or fallback to error messages
    if echo "$files_json" | jq -e 'if type=="array" then true else false end' >/dev/null 2>&1; then
        files_array=($(echo "$files_json" | jq -r '.[]'))
    else
        echo "Using files from error messages..."
        files_array=($(extract_files "$error_output"))
    fi

    if [ ${#files_array[@]} -eq 0 ]; then
        echo "No files found in errors, checking full output..."
        files_array=($(extract_files "$test_output"))
        
        if [ ${#files_array[@]} -eq 0 ]; then
            echo "No files found. Examining default locations..."
            files_array=("src/lib.rs" "src/main.rs")
        fi
    fi

    echo -e "\nFiles to examine: ${files_array[*]}"
    echo -e "\nExamining files: ${files_array[*]}\n" >> "$LOG_FILE"
    
    changes_made=0
    
    # Process each file
    for file in "${files_array[@]}"; do
        echo -e "\n### Analyzing $file..." | tee -a "$LOG_FILE"
        
        if [ ! -f "$file" ]; then
            echo "❌ File $file not found, skipping..." | tee -a "$LOG_FILE"
            continue
        fi
        
        file_content=$(cat "$file")
        
        # Check if file needs changes
        echo "Analyzing file for changes..." | tee -a "$LOG_FILE"
        response=$(call_deepseek "You are a Rust expert. Given this file:

$file_content

And these test failures:

$error_output

Does this file need changes to fix the failing tests? If yes, provide the complete updated file content. If no, respond with 'NO_CHANGES_NEEDED'.

Format your response to start with either 'CHANGES:' followed by the new content, or 'NO_CHANGES_NEEDED'.")
        
        # Debug the response
        echo -e "\nAI Response:" >> "$LOG_FILE"
        echo '```' >> "$LOG_FILE"
        echo "$response" >> "$LOG_FILE"
        echo '```' >> "$LOG_FILE"
        
        if [[ "$response" == NO_CHANGES_NEEDED* ]]; then
            echo "✓ SKIPPING: No changes needed" | tee -a "$LOG_FILE"
            continue
        elif [[ "$response" == CHANGES:* ]]; then
            # Get new content
            new_content="${response#CHANGES:}"
            
            # Get explanation
            explanation=$(call_deepseek "Explain in one line what changes were made to $file and why they fix the failing tests.")
            
            echo "🔨 FIXING: $explanation" | tee -a "$LOG_FILE"
            
            # Log changes
            {
                echo "Changes:"
                echo '```diff'
                diff -u "$file" <(echo "$new_content") || true
                echo '```'
                echo
            } >> "$LOG_FILE"
            
            # Update file and commit
            echo "$new_content" > "$file"
            git add "$file" "$LOG_FILE"
            git commit -m "fix($file): $explanation" -n
            
            ((changes_made++))
            echo "✅ Changes committed" | tee -a "$LOG_FILE"
        else
            echo "⚠️ ERROR: AI response did not start with CHANGES: or NO_CHANGES_NEEDED" | tee -a "$LOG_FILE"
            echo "Response was: ${response:0:100}..." | tee -a "$LOG_FILE"
        fi
    done
    
    echo -e "\nIteration $iteration summary:" | tee -a "$LOG_FILE"
    echo "- Files examined: ${#files_array[@]}" | tee -a "$LOG_FILE"
    echo "- Changes made: $changes_made" | tee -a "$LOG_FILE"
    echo "- Tests still failing: $test_exit_code" | tee -a "$LOG_FILE"
    echo
    
    if [ $changes_made -eq 0 ]; then
        echo "⚠️ WARNING: No changes were made this iteration!" | tee -a "$LOG_FILE"
        echo "This could mean:"
        echo "1. The AI failed to identify necessary changes"
        echo "2. The errors are in dependencies or configuration"
        echo "3. The errors require manual intervention"
    fi
    
    ((iteration++))
done

echo -e "\n## Final Status\n" >> "$LOG_FILE"
echo "Maximum iterations reached. Some tests may still be failing." | tee -a "$LOG_FILE"
echo "Review the changes in $LOG_FILE for details." | tee -a "$LOG_FILE"
exit 1