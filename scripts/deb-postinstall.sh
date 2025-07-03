#!/bin/bash
# Post-install script for webly .deb package

set -e

echo "Setting up webly..."

# Run the setup script for all users who have a home directory
# This will create ~/.webly/profile if it doesn't exist
if [ -n "$SUDO_USER" ]; then
    # If installed with sudo, set up for the user who ran sudo
    USER_HOME=$(eval echo ~$SUDO_USER)
    if [ -d "$USER_HOME" ]; then
        sudo -u "$SUDO_USER" /usr/share/webly/setup-profile.sh
    fi
else
    # If not running with sudo, set up for current user
    /usr/share/webly/setup-profile.sh
fi

echo "Webly setup complete!"
echo "See documentation: https://github.com/samwisely75/webly"
