// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/PicoVerifier.sol";

contract DeployPicoVerifier is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");

        vm.startBroadcast(deployerPrivateKey);

        PicoVerifier picoVerifier = new PicoVerifier();

        console.log("PicoVerifier deployed to:", address(picoVerifier));

        vm.stopBroadcast();
    }
}
