// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {SafeCast} from "@openzeppelin/contracts/utils/math/SafeCast.sol";
import {IssuerToken} from "../src/IssuerToken.sol";

interface ICustomERC20Registry {
    function setCustomERC20(string calldata denom, address token) external;
}

contract DeployIssuerToken is Script {
    function run() external returns (IssuerToken token) {
        address ics20 = vm.envAddress("EVM_ICS20_TRANSFER");
        address admin = vm.envAddress("ISSUER_ADMIN");
        string memory denom = vm.envString("ISSUER_DENOM");

        vm.startBroadcast();

        token = new IssuerToken(
            vm.envString("ISSUER_TOKEN_NAME"),
            vm.envString("ISSUER_TOKEN_SYMBOL"),
            SafeCast.toUint8(vm.envOr("ISSUER_TOKEN_DECIMALS", uint256(7))),
            ics20,
            admin
        );

        ICustomERC20Registry(ics20).setCustomERC20(denom, address(token));

        vm.stopBroadcast();

        console.log("IssuerToken", address(token));
        console.log("registered for", denom);
    }
}
