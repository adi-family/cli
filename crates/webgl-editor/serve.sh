#!/bin/bash
# Simple development server for testing the WebGL editor
# Requires Python 3

PORT=${1:-8080}
echo "Starting server at http://localhost:$PORT/www/"
echo "Press Ctrl+C to stop"

cd "$(dirname "$0")"
python3 -m http.server $PORT --bind 127.0.0.1
