#!/bin/bash
# Patch audiowmark for Windows (Cygwin) compilation

set -e

WORKDIR=${1:-.}
cd "$WORKDIR"

if [ ! -d "audiowmark" ]; then
    echo "audiowmark directory not found in $WORKDIR"
    exit 1
fi

cd audiowmark

echo "Applying Windows-specific patches..."

# Patch 1: Add _GNU_SOURCE to src/utils.cc
echo "Patch 1: Adding #define _GNU_SOURCE to src/utils.cc"
if ! head -n 1 src/utils.cc | grep -q "_GNU_SOURCE"; then
    sed -i '1i#define _GNU_SOURCE' src/utils.cc
    echo "✓ Patch 1 applied"
else
    echo "✓ Patch 1 already applied"
fi

# Patch 2: Add _GNU_SOURCE to src/wmcommon.cc if needed
echo "Patch 2: Checking src/wmcommon.cc"
if ! head -n 1 src/wmcommon.cc | grep -q "_GNU_SOURCE"; then
    sed -i '1i#define _GNU_SOURCE' src/wmcommon.cc
    echo "✓ Patch 2 applied"
else
    echo "✓ Patch 2 already applied"
fi

# Patch 3: Check for other potential compatibility issues
echo "Patch 3: Checking for compatibility issues..."
if grep -r "clock_gettime" src/*.cc 2>/dev/null | grep -v "// "; then
    echo "⚠ Warning: clock_gettime found, may need porting"
fi

echo "✓ All patches applied successfully"
