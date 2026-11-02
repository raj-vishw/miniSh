#!/bin/bash
set -e

# Compile latest build
cargo build

MINISH="./target/debug/minish"
# On Windows cargo builds minish.exe
[ -x "$MINISH.exe" ] && MINISH="$MINISH.exe"

# Scratch folder inside the project: native Windows binaries cannot resolve /tmp
TMPD=".test_tmp"
rm -rf "$TMPD" && mkdir -p "$TMPD"

echo "=== Running Output Verification Tests (diffed against bash) ==="

# Test 1: Whitespace collapsing
echo -n "Test 1 (whitespace collapsing): "
$MINISH -c "echo one two   three" > $TMPD/out_minish.txt
bash -c "echo one two   three" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 2: Quoting
echo -n "Test 2 (quoting): "
$MINISH -c "echo \"a  b\" 'c  d'" > $TMPD/out_minish.txt
bash -c "echo \"a  b\" 'c  d'" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 3: 3-stage pipeline
echo -n "Test 3 (3-stage pipeline): "
$MINISH -c "printf 'b\na\nc\n' | sort | head -2" > $TMPD/out_minish.txt
bash -c "printf 'b\na\nc\n' | sort | head -2" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 4: Redirection
echo -n "Test 4 (redirection): "
$MINISH -c "echo data > $TMPD/mt.txt"
$MINISH -c "cat < $TMPD/mt.txt" > $TMPD/out_minish.txt
bash -c "cat < $TMPD/mt.txt" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 5: cd state mutation
echo -n "Test 5 (cd state mutation): "
res=$(printf "cd $TMPD\npwd\n" | $MINISH | sed 's/minish>//g' | xargs)
case "$res" in *"$TMPD") ok=1 ;; *) ok=0 ;; esac
if [ "$ok" = 1 ]; then
    echo "OK"
else
    echo "FAIL (got $res)"
    exit 1
fi

# Test 6: Concurrency (yes | head -3)
echo -n "Test 6 (concurrency with yes | head -3): "
res=$(timeout 2 $MINISH -c "yes | head -3" | tr '\n' ' ' | xargs)
if [ "$res" = "y y y" ]; then
    echo "OK"
else
    echo "FAIL (got '$res')"
    exit 1
fi

# Test 7: Exit code 127
echo -n "Test 7 (exit code 127): "
set +e
$MINISH -c "nosuchcmd" 2>/dev/null
status=$?
set -e
if [ $status -eq 127 ]; then
    echo "OK"
else
    echo "FAIL (got status $status)"
    exit 1
fi

# Test 8: Chaining (&& / ||)
echo -n "Test 8 (chaining && and ||): "
$MINISH -c "true && echo success || echo fail" > $TMPD/out_minish.txt
bash -c "true && echo success || echo fail" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 9: $VAR and $? expansion
echo -n "Test 9 (\$VAR and \$? expansion): "
export TEST_VAR="hello_env"
$MINISH -c "echo \$TEST_VAR \$? \"\$TEST_VAR\" '\$TEST_VAR'" > $TMPD/out_minish.txt
bash -c "echo \$TEST_VAR \$? \"\$TEST_VAR\" '\$TEST_VAR'" > $TMPD/out_bash.txt
if diff $TMPD/out_minish.txt $TMPD/out_bash.txt; then
    echo "OK"
else
    echo "FAIL"
    exit 1
fi

# Test 10: Defunct/Zombie check
echo -n "Test 10 (zombie process check): "
# Let's run a pipeline and check if there are any defunct processes of ours
$MINISH -c "sleep 0.1 | sleep 0.1 | sleep 0.1"
zombies=$( (ps -ef 2>/dev/null | grep defunct | grep -v grep) || true)
if [ -z "$zombies" ]; then
    echo "OK (no zombies found)"
else
    echo "FAIL (found zombies: $zombies)"
    exit 1
fi

# Clean up
rm -rf "$TMPD"

echo ""
echo "=== All Acceptance Tests Passed Successfully ==="
