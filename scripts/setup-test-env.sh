#!/bin/bash
# Setup Python testing environment
# Creates a virtual environment and installs required dependencies for Pub/Sub testing scripts

set -e

VENV_DIR=".venv"

echo "🔧 Setting up Python test environment..."
echo ""

# Check if python3 is available
if ! command -v python3 &> /dev/null; then
    echo "❌ python3 is not installed"
    echo "   Please install Python 3.7 or higher"
    exit 1
fi

echo "✓ Python version: $(python3 --version)"
echo ""

# Create virtual environment if it doesn't exist
if [ ! -d "$VENV_DIR" ]; then
    echo "📦 Creating virtual environment..."
    python3 -m venv "$VENV_DIR"
    echo "✓ Virtual environment created at $VENV_DIR"
else
    echo "✓ Virtual environment already exists at $VENV_DIR"
fi
echo ""

# Activate virtual environment
echo "🔌 Activating virtual environment..."
source "$VENV_DIR/bin/activate"
echo "✓ Virtual environment activated"
echo ""

# Upgrade pip
echo "⬆️  Upgrading pip..."
python -m pip install --upgrade pip -q
echo "✓ pip version: $(pip --version)"
echo ""

# Install dependencies
echo "📦 Installing Python dependencies..."
pip install -r scripts/requirements.txt -q
echo "✓ Dependencies installed"

echo ""
echo "✅ Test environment setup complete!"
echo ""
echo "Next steps:"
echo "  1. Activate the virtual environment: source $VENV_DIR/bin/activate"
echo "  2. Start the emulator: ./scripts/start-emulator.sh"
echo "  3. Run tests: ./scripts/run-test.sh e2e"
echo ""
echo "💡 Tip: The virtual environment will be automatically used by test scripts"
