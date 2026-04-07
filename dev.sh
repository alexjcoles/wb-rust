#!/usr/bin/env bash
set -euo pipefail

# Start both backend and frontend for development.
# Backend: Rust API on port 5656
# Frontend: Vite dev server on port 5173 (default)

cleanup() {
  echo ""
  echo "Shutting down..."
  kill $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
  wait $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
}
trap cleanup EXIT

# Build backend first (so we catch compile errors early)
echo "Building backend..."
cargo build -p wynn-api 2>&1

# Fetch item data if missing
if [ ! -f data/items.json ]; then
  echo "Item data not found, will be fetched on first run..."
fi

# Start backend
echo "Starting backend on :5656..."
cargo run -p wynn-api 2>&1 &
BACKEND_PID=$!

# Start frontend
echo "Starting frontend..."
cd frontend
npm run dev 2>&1 &
FRONTEND_PID=$!
cd ..

echo ""
echo "================================"
echo "  Backend:  http://localhost:5656"
echo "  Frontend: http://localhost:5173"
echo "================================"
echo "Press Ctrl+C to stop both."
echo ""

wait
