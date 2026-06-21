#!/usr/bin/env bash
# Feature 115: Agent Notification Sound
# Plays a pleasant chime when the Claude Code agent needs user input.
# Runs on the Stop hook event. macOS only (uses afplay).

if [[ "$(uname)" == "Darwin" ]]; then
    afplay /System/Library/Sounds/Hero.aiff &
fi
