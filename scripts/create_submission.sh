#!/bin/bash

# KeOS Project Submission Script
# Creates tar.gz files for project submissions based on .grade-target whitelist files
# Each project includes dependencies from previous projects (e.g., project4 includes files from projects 1-4)

set -e

if [ $# -ne 3 ]; then
    echo "Usage: $0 <project_number> <student_id> <name>"
    echo "Example: $0 2 20250123 GildongHong"
    echo "Available projects: 1, 2, 3, 4, 5"
    exit 1
fi

PROJECT_NUM=$1
STUDENT_ID=$2
NAME=$3
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SUBMISSIONS_DIR="$ROOT_DIR/submissions"

# Validate project number
if [[ ! "$PROJECT_NUM" =~ ^[1-5]$ ]]; then
    echo "Error: Project number must be between 1 and 5"
    exit 1
fi

# Create submissions directory if it doesn't exist
mkdir -p "$SUBMISSIONS_DIR"

# Function to extract whitelist files from .grade-target
extract_whitelist() {
    local grade_target_file=$1
    if [ ! -f "$grade_target_file" ]; then
        echo "Warning: .grade-target not found at $grade_target_file"
        return 1
    fi
    
    # Fallback: use grep and sed to extract whitelist
    grep -A 10 '"whitelist"' "$grade_target_file" | \
    sed -n '/\[/,/\]/p' | \
    grep '"' | \
    sed 's/.*"\([^"]*\)".*/\1/' | \
    grep -v whitelist || {
        echo "Warning: Failed to parse whitelist from $grade_target_file using grep/sed"
        return 1
    }
}

# Function to get project directory name
get_project_dir() {
    local proj_num=$1
    case $proj_num in
        1) echo "keos-project1" ;;
        2) echo "keos-project2" ;;
        3) echo "keos-project3" ;;
        4) echo "keos-project4" ;;
        5) echo "keos-project5" ;;
        *) echo ""; return 1 ;;
    esac
}

# Collect all whitelist files for projects 1 through PROJECT_NUM
ALL_FILES=()
TEMP_DIR=$(mktemp -d)

echo "Creating submission for project $PROJECT_NUM..."
echo "Including files from projects 1 through $PROJECT_NUM"

for i in $(seq 1 $PROJECT_NUM); do
    PROJECT_DIR=$(get_project_dir $i)
    if [ -z "$PROJECT_DIR" ]; then
        echo "Error: Invalid project number $i"
        exit 1
    fi
    
    GRADE_TARGET="$ROOT_DIR/keos-projects/$PROJECT_DIR/grader/.grade-target"
    echo "Processing project $i ($PROJECT_DIR)..."
    
    if [ -f "$GRADE_TARGET" ]; then
        echo "  Reading whitelist from: $GRADE_TARGET"
        WHITELIST_FILES=$(extract_whitelist "$GRADE_TARGET")
        
        if [ $? -eq 0 ] && [ -n "$WHITELIST_FILES" ]; then
            while IFS= read -r file; do
                if [ -n "$file" ]; then
                    SOURCE_FILE="$ROOT_DIR/keos-projects/$PROJECT_DIR/$file"
                    if [ -f "$SOURCE_FILE" ]; then
                        # Create directory structure in temp dir
                        TARGET_DIR="$TEMP_DIR/$PROJECT_DIR/$(dirname "$file")"
                        mkdir -p "$TARGET_DIR"
                        
                        # Copy file
                        cp "$SOURCE_FILE" "$TARGET_DIR/"
                        echo "    Added: $PROJECT_DIR/$file"
                        ALL_FILES+=("$PROJECT_DIR/$file")
                    else
                        echo "    Warning: File not found: $SOURCE_FILE"
                    fi
                fi
            done <<< "$WHITELIST_FILES"
        else
            echo "    Warning: No whitelist files found or failed to parse"
        fi
    else
        echo "    Warning: .grade-target not found for project $i"
    fi
    echo ""
done

# Create tar.gz file
OUTPUT_FILE="$SUBMISSIONS_DIR/project${PROJECT_NUM}_${STUDENT_ID}_${NAME}.tar.gz"
echo "Creating submission archive: $OUTPUT_FILE"

if [ ${#ALL_FILES[@]} -eq 0 ]; then
    echo "Error: No files to include in submission"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Create tar.gz from temp directory
cd "$TEMP_DIR"
tar -czf "$OUTPUT_FILE" ./*

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo "✅ Submission created successfully!"
echo "📁 Archive: $OUTPUT_FILE"