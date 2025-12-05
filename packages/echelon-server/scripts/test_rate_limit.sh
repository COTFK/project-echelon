#!/bin/bash
# Test script for rate limiting on the echelon-server
# Usage: ./test_rate_limit.sh [server_url]
#
# This script sends multiple upload requests to test the rate limiter.
# The server should allow 5 requests in a burst, then rate limit subsequent requests.

SERVER_URL="${1:-http://localhost:3000}"
UPLOAD_ENDPOINT="$SERVER_URL/upload"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Rate Limit Test for Echelon Server"
echo "=========================================="
echo "Server URL: $SERVER_URL"
echo ""

# Check if server is reachable
echo "Checking server connectivity..."
if ! curl -s --connect-timeout 5 "$SERVER_URL" > /dev/null 2>&1; then
    # Try once more - server might just not have a root endpoint
    if ! curl -s --connect-timeout 5 -o /dev/null -w "%{http_code}" "$SERVER_URL/status/test" 2>/dev/null | grep -qE "^[0-9]+$"; then
        echo -e "${RED}Error: Cannot connect to server at $SERVER_URL${NC}"
        echo "Make sure the server is running (docker compose up --build)"
        exit 1
    fi
fi
echo -e "${GREEN}Server is reachable!${NC}"
echo ""

# Create a valid yrpX test file (magic bytes + padding)
TEMP_FILE=$(mktemp)
printf 'yrpX' > "$TEMP_FILE"
dd if=/dev/zero bs=100 count=1 >> "$TEMP_FILE" 2>/dev/null

cleanup() {
    rm -f "$TEMP_FILE"
}
trap cleanup EXIT

echo "Test 1: Burst requests (should allow first 5, block 6th)"
echo "------------------------------------------"

success_count=0
rate_limited_count=0

for i in {1..8}; do
    response=$(curl -s -w "\n%{http_code}" -X POST "$UPLOAD_ENDPOINT" \
        -H "Content-Type: application/octet-stream" \
        --data-binary "@$TEMP_FILE" 2>/dev/null || echo -e "\n000")
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" == "200" ]; then
        echo -e "  Request $i: ${GREEN}200 OK${NC} - Job ID: $body"
        success_count=$((success_count + 1))
    elif [ "$http_code" == "429" ]; then
        echo -e "  Request $i: ${YELLOW}429 Too Many Requests${NC} - Rate limited!"
        rate_limited_count=$((rate_limited_count + 1))
    elif [ "$http_code" == "000" ]; then
        echo -e "  Request $i: ${RED}Connection failed${NC}"
    else
        echo -e "  Request $i: ${RED}$http_code${NC} - $body"
    fi
done

echo ""
echo "Results:"
echo "  Successful requests: $success_count"
echo "  Rate limited requests: $rate_limited_count"
echo ""

if [ "$success_count" -eq 5 ] && [ "$rate_limited_count" -ge 1 ]; then
    echo -e "${GREEN}✓ Rate limiting is working correctly!${NC}"
    echo "  - Allowed burst of 5 requests"
    echo "  - Blocked subsequent requests with 429"
elif [ "$success_count" -gt 5 ]; then
    echo -e "${RED}✗ Rate limiting may not be working!${NC}"
    echo "  - Expected max 5 successful requests, got $success_count"
elif [ "$rate_limited_count" -eq 0 ]; then
    echo -e "${YELLOW}⚠ No rate limiting detected${NC}"
    echo "  - All requests succeeded (queue might have filled up instead)"
else
    echo -e "${YELLOW}⚠ Unexpected results${NC}"
    echo "  - Expected 5 successes, got $success_count"
    echo "  - Check server configuration"
fi

echo ""
echo "=========================================="
echo "Test 2: Status endpoint (should NOT be rate limited)"
echo "------------------------------------------"

# Wait for rate limit to reset before trying to get a job ID
echo "  Waiting 65 seconds for rate limit to fully reset..."
sleep 65

# Get a job ID first
job_response=$(curl -s -w "\n%{http_code}" -X POST "$UPLOAD_ENDPOINT" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@$TEMP_FILE" 2>/dev/null || echo -e "\n000")

job_http_code=$(echo "$job_response" | tail -n1)
job_body=$(echo "$job_response" | sed '$d')

if [ "$job_http_code" != "200" ]; then
    echo -e "  ${RED}Failed to create job for status test (HTTP $job_http_code)${NC}"
    echo "  Skipping status endpoint test"
    JOB_ID=""
else
    JOB_ID="$job_body"
    echo "  Using job ID: $JOB_ID"
fi
echo ""

if [ -z "$JOB_ID" ]; then
    echo -e "${YELLOW}⚠ Skipped - could not create test job${NC}"
else
    status_success=0
    for i in {1..10}; do
        http_code=$(curl -s -o /dev/null -w "%{http_code}" "$SERVER_URL/status/$JOB_ID" 2>/dev/null || echo "000")
        
        if [ "$http_code" == "200" ]; then
            status_success=$((status_success + 1))
            echo -e "  Status request $i: ${GREEN}200 OK${NC}"
        elif [ "$http_code" == "429" ]; then
            echo -e "  Status request $i: ${RED}429 Rate Limited${NC} (unexpected!)"
        else
            echo -e "  Status request $i: $http_code"
        fi
    done

    echo ""
    if [ "$status_success" -eq 10 ]; then
        echo -e "${GREEN}✓ Status endpoint is NOT rate limited (correct!)${NC}"
    else
        echo -e "${RED}✗ Status endpoint appears to be rate limited (incorrect!)${NC}"
    fi
fi

echo ""
echo "=========================================="
echo "Test 3: Rate limit recovery"
echo "------------------------------------------"

# First, wait for full reset from Test 2
echo "  Waiting 65 seconds for rate limit to fully reset..."
sleep 65

# Exhaust exactly 5 tokens (the burst limit)
echo "  Sending exactly 5 requests to exhaust burst limit..."
for i in {1..5}; do
    curl -s -o /dev/null -X POST "$UPLOAD_ENDPOINT" \
        -H "Content-Type: application/octet-stream" \
        --data-binary "@$TEMP_FILE" 2>/dev/null
done

# Verify we're rate limited
http_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$UPLOAD_ENDPOINT" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@$TEMP_FILE")

if [ "$http_code" != "429" ]; then
    echo -e "  ${RED}Failed to exhaust rate limit (got $http_code)${NC}"
else
    echo "  Rate limit exhausted (429)"
    echo "  Waiting 65 seconds for rate limit to recover..."
    sleep 65
    
    http_code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$UPLOAD_ENDPOINT" \
        -H "Content-Type: application/octet-stream" \
        --data-binary "@$TEMP_FILE")

    if [ "$http_code" == "200" ]; then
        echo -e "  After waiting: ${GREEN}200 OK${NC} - Rate limit recovered!"
        echo -e "${GREEN}✓ Rate limit recovery is working correctly!${NC}"
    elif [ "$http_code" == "429" ]; then
        echo -e "  After waiting: ${RED}429 Still rate limited${NC}"
        echo -e "${RED}✗ Rate limit recovery FAILED - users may get locked out!${NC}"
    else
        echo -e "  After waiting: $http_code"
    fi
fi

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo "Rate limiting configuration:"
echo "  - Burst limit: 5 requests"
echo "  - Recovery period: 60 seconds (full reset)"
echo "  - Only /upload is rate limited"
echo ""
echo "All tests completed!"
echo "=========================================="
