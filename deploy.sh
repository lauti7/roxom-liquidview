#!/bin/bash

# Deployment script for roxom-liquidview
# Designed to be run by cron every 5 minutes
# Compares current GitHub commit with last deployed commit and redeploys if different

set -e

# Configuration
REPO_DIR="./roxom-liquidview"
SERVICE_NAME="app"
LAST_COMMIT_FILE="/tmp/roxom-liquidview-last-commit"
LOG_FILE="./deploy.log"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# GitHub PAT from environment
if [ -z "$GIT_PAT" ]; then
    log "Error: GIT_PAT environment variable is not set"
    exit 1
fi

# Create auth URL
AUTH_REPO_URL="https://${GIT_PAT}@github.com/lauti7/roxom-liquidview"

# Function to get latest commit SHA from GitHub API
get_latest_commit() {
    curl -s -H "Authorization: token ${GIT_PAT}" \
         -H "Accept: application/vnd.github.v3+json" \
         "https://api.github.com/repos/lauti7/roxom-liquidview/commits/main" | \
         grep -o '"sha": "[^"]*"' | head -1 | cut -d'"' -f4
}

# Function to read last deployed commit
get_last_deployed_commit() {
    if [ -f "$LAST_COMMIT_FILE" ]; then
        cat "$LAST_COMMIT_FILE"
    else
        echo ""
    fi
}

# Function to save deployed commit
save_deployed_commit() {
    echo "$1" > "$LAST_COMMIT_FILE"
}

# Function to clone or update repository
update_repository() {
    log "Updating repository..."

    if [ -d "$REPO_DIR" ]; then
        # Repo exists, fetch and pull
        cd "$REPO_DIR"
        git fetch origin main
        git reset --hard origin/main
        cd ..
    else
        # Clone fresh
        git clone "$AUTH_REPO_URL" "$REPO_DIR"
    fi

    log "Repository updated successfully"
}

# Function to build and restart services
deploy() {
    log "Building and deploying..."

    # Check docker-compose.yml exists
    if [ ! -f "$REPO_DIR/tracker/docker-compose.yml" ]; then
        log "Error: docker-compose.yml not found in repository"
        exit 1
    fi

    # Navigate to tracker directory and rebuild
    cd "$REPO_DIR/tracker"

    # Pull latest base images
    docker-compose pull >> "$LOG_FILE" 2>&1

    # Build and restart the app service
    docker-compose up -d --build "$SERVICE_NAME" >> "$LOG_FILE" 2>&1

    # Also ensure timescaledb is running
    docker-compose up -d timescaledb >> "$LOG_FILE" 2>&1

    cd ../..

    log "Deployment completed successfully"
}

# Function to cleanup old images
cleanup_images() {
    log "Cleaning up old Docker images..."
    docker image prune -f --filter "dangling=true" >> "$LOG_FILE" 2>&1
}

# Main execution
log "Starting deployment check"

# Get current commit from GitHub
CURRENT_COMMIT=$(get_latest_commit)
if [ -z "$CURRENT_COMMIT" ]; then
    log "Error: Could not fetch latest commit from GitHub. Please check your GIT_PAT is valid."
    exit 1
fi

log "Current GitHub commit: $CURRENT_COMMIT"

# Get last deployed commit
LAST_DEPLOYED_COMMIT=$(get_last_deployed_commit)
if [ -n "$LAST_DEPLOYED_COMMIT" ]; then
    log "Last deployed commit: $LAST_DEPLOYED_COMMIT"
else
    log "No previous deployment found (first run)"
fi

# Check if we need to deploy
if [ "$CURRENT_COMMIT" = "$LAST_DEPLOYED_COMMIT" ]; then
    log "No new commits. Already up to date."
    exit 0
fi

# Deploy
if [ -n "$LAST_DEPLOYED_COMMIT" ]; then
    log "New commit detected! Deploying..."
    log "Previous: $LAST_DEPLOYED_COMMIT"
    log "Current:  $CURRENT_COMMIT"
else
    log "Initial deployment"
fi

update_repository
deploy
cleanup_images
save_deployed_commit "$CURRENT_COMMIT"

log "Deployment finished"
