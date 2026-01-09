#!/bin/bash

# Deployment script for PicoVerifier using Foundry
# Usage: ./deploy.sh <network>
# Example: ./deploy.sh sepolia

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if network is provided
if [ -z "$1" ]; then
    echo -e "${RED}Error: Network not specified${NC}"
    echo "Usage: ./deploy.sh <network>"
    echo "Supported networks: mainnet, sepolia"
    exit 1
fi

NETWORK=$1

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${RED}Error: .env file not found${NC}"
    echo "Please create a .env file with the following variables:"
    echo "  PRIVATE_KEY=your_private_key"
    echo "  MAINNET_RPC_URL=your_mainnet_rpc_url"
    echo "  SEPOLIA_RPC_URL=your_sepolia_rpc_url"
    echo "  ETHERSCAN_API_KEY=your_etherscan_api_key (optional, for verification)"
    exit 1
fi

# Load environment variables
source .env

# Check if PRIVATE_KEY is set
if [ -z "$PRIVATE_KEY" ]; then
    echo -e "${RED}Error: PRIVATE_KEY not set in .env${NC}"
    exit 1
fi

# Set RPC URL based on network
case $NETWORK in
    mainnet)
        RPC_URL=$MAINNET_RPC_URL
        EXPLORER="https://etherscan.io"
        ;;
    sepolia)
        RPC_URL=$SEPOLIA_RPC_URL
        EXPLORER="https://sepolia.etherscan.io"
        ;;
    *)
        echo -e "${RED}Error: Unsupported network '$NETWORK'${NC}"
        echo "Supported networks: mainnet, sepolia"
        exit 1
        ;;
esac

if [ -z "$RPC_URL" ]; then
    echo -e "${RED}Error: RPC_URL not set for network '$NETWORK'${NC}"
    echo "Please set ${NETWORK^^}_RPC_URL in your .env file"
    exit 1
fi

echo -e "${GREEN}Deploying PicoVerifier to $NETWORK...${NC}"
echo "RPC URL: $RPC_URL"

# Deploy the contract
forge script script/Deploy.s.sol:DeployPicoVerifier \
    --rpc-url $RPC_URL \
    --broadcast \
    --verify \
    -vvvv

echo -e "${GREEN}Deployment complete!${NC}"

# Extract the deployed address from the broadcast file
BROADCAST_FILE="broadcast/Deploy.s.sol/$NETWORK/run-latest.json"
if [ -f "$BROADCAST_FILE" ]; then
    CONTRACT_ADDRESS=$(jq -r '.transactions[0].contractAddress' "$BROADCAST_FILE")
    echo -e "${GREEN}PicoVerifier deployed to: $CONTRACT_ADDRESS${NC}"

    if [ ! -z "$EXPLORER" ]; then
        echo -e "${YELLOW}View on explorer: $EXPLORER/address/$CONTRACT_ADDRESS${NC}"
    fi
fi