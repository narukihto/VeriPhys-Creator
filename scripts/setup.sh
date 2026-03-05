#!/bin/bash

echo "🛠️  VeriPhys Protocol: Automatic Setup Initialized..."

# 1. Install Node dependencies
echo "📦 Installing Node.js dependencies..."
npm install

# 2. Compile Contract
echo "⛓️  Compiling Smart Contracts..."
npx hardhat compile

# 3. Build Rust Engine
echo "🦀 Building Rust Hashing Engine..."
cargo build --release

# 4. Environment Check
if [ ! -f .env ]; then
    echo "📄 Creating .env file from example..."
    cp .env.example .env
fi

echo "✨ Setup Complete! Please update your .env and run 'cargo run'."
