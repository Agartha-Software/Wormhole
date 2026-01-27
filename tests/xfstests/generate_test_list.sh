#!/bin/bash
# tests/xfstests/generate_test_list.sh

XFSTESTS_DIR="/opt/xfstests-dev"
DOC_FILE="/tmp/xfstests_list.md"


# 1. Directory check
if [ ! -d "$XFSTESTS_DIR" ]; then
    echo "CRITICAL ERROR: Directory $XFSTESTS_DIR does not exist."
    echo "Contents of /opt :"
    ls -F /opt
    exit 1
fi

cd "$XFSTESTS_DIR" || exit 1


# 2. Detect group file (group or group.list)
if [ -f "tests/generic/group.list" ]; then
    GROUP_FILE="tests/generic/group.list"
elif [ -f "tests/generic/group" ]; then
    GROUP_FILE="tests/generic/group"
else
    echo "CRITICAL ERROR: Could not find 'group' or 'group.list' in tests/generic/."
    echo "Contents of tests/generic/ :"
    ls -F tests/generic/
    exit 1
fi


echo "Group file found: $GROUP_FILE"


# 3. Start generation
echo "# XFSTests Test Catalog (Group 'quick')" > "$DOC_FILE"
echo "" >> "$DOC_FILE"
echo "This document lists all tests included in the \`quick\` group of xfstests, sorted by main category." >> "$DOC_FILE"
echo "Generated on $(date)" >> "$DOC_FILE"
echo "" >> "$DOC_FILE"


# Function to extract the description from a test file
get_description() {
    local test_file=$1
    local desc=""
    
    if [ -f "$test_file" ]; then
        # We filter:
        # 1. SPDX license lines
        # 2. Copyright lines
        # 3. Standard header "FS QA Test..."
        # 4. Empty lines
        desc=$(grep "^# " "$test_file" | \
               grep -v -i "SPDX-License-Identifier" | \
               grep -v -i "Copyright" | \
               grep -v -i "FS QA Test" | \
               sed 's/^# *//' | \
               grep -v "^$" | \
               head -n 1 | \
               tr -d '\r')
    fi

    if [ -z "$desc" ]; then
        echo "Description not available"
    else
        echo "$desc"
    fi
}

TEMP_LIST=$(mktemp)


# 4. Extract tests (compatible with spaces and tabs)
# We look for 'quick' preceded by a space or tab
grep -E "[[:space:]]quick([[:space:]]|$)" "$GROUP_FILE" | while read -r line; do
    # $line contains e.g.: "001 rw quick auto"
    test_id=$(echo "$line" | awk '{print $1}')
    
    # Skip commented lines
    if [[ "$test_id" == \#* ]]; then continue; fi

    # Get all groups except the ID
    groups=$(echo "$line" | cut -d ' ' -f2- | tr '\t' ' ')
    
    # Determine the category (first word that is not auto, quick, or the number)
    category="misc"
    for g in $groups; do
        if [[ "$g" != "auto" && "$g" != "quick" && "$g" != "$test_id" ]]; then
            category="$g"
            break
        fi
    done
    
    desc=$(get_description "tests/generic/$test_id")
    echo "$category|generic/$test_id|$desc" >> "$TEMP_LIST"
done


# Check if we found anything
LINE_COUNT=$(wc -l < "$TEMP_LIST")
if [ "$LINE_COUNT" -eq 0 ]; then
    echo "WARNING: No 'quick' test found in $GROUP_FILE."
    echo "Here are the first 5 lines of the group file for debugging:"
    head -n 5 "$GROUP_FILE"
    rm "$TEMP_LIST"
    exit 1
fi


# 5. Write final Markdown
echo "## Category Summary" >> "$DOC_FILE"
cut -d '|' -f1 "$TEMP_LIST" | sort | uniq | while read -r cat; do
    echo "- [$cat](#category-$cat)" >> "$DOC_FILE"
done
echo "" >> "$DOC_FILE"

cut -d '|' -f1 "$TEMP_LIST" | sort | uniq | while read -r cat; do
    echo "## Category: $cat <a name=\"category-$cat\"></a>" >> "$DOC_FILE"
    echo "| Test ID | Description |" >> "$DOC_FILE"
    echo "|---|---|" >> "$DOC_FILE"
    
    grep "^$cat|" "$TEMP_LIST" | while IFS='|' read -r c id d; do
        echo "| **$id** | $d |" >> "$DOC_FILE"
    done
    echo "" >> "$DOC_FILE"
done

rm "$TEMP_LIST"
cat "$DOC_FILE"