// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {GovernancePoll} from "../src/GovernancePoll.sol";

contract DeployGovernancePoll is Script {
    function run() external returns (GovernancePoll poll) {
        address owner = vm.envAddress("POLL_OWNER");

        vm.startBroadcast();

        poll = new GovernancePoll(owner);

        vm.stopBroadcast();

        console.log("GovernancePoll", address(poll));
    }
}
