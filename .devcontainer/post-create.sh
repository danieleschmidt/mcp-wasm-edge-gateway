#!/bin/bash
set -e

echo "🎯 Running post-creation setup..."

# Make setup script executable
chmod +x .devcontainer/setup.sh

# Install project dependencies (when Cargo.toml exists)
if [ -f "Cargo.toml" ]; then
    echo "📦 Installing project dependencies..."
    cargo fetch
    
    # Pre-build common dependencies to speed up development
    echo "🔧 Pre-building common dependencies..."
    cargo build --release --target wasm32-unknown-unknown || echo "WASM build not ready yet"
    cargo build || echo "Native build not ready yet"
fi

# Set up git hooks
echo "🪝 Setting up git hooks..."
if [ -f ".husky/pre-commit" ]; then
    chmod +x .husky/pre-commit
fi

# Create necessary directories
echo "📁 Creating project directories..."
mkdir -p {src,tests,benches,examples,docs/{guides,runbooks},scripts}

# Set up environment variables
echo "🌍 Setting up environment variables..."
if [ ! -f ".env" ]; then
    cp .env.example .env 2>/dev/null || echo "# MCP Gateway Environment Variables" > .env
fi

# Install pre-commit hooks if configured
if command -v pre-commit >/dev/null 2>&1; then
    echo "🔒 Installing pre-commit hooks..."
    pre-commit install
fi

echo "✅ Post-creation setup complete!"
echo "🚀 Ready to start development!"