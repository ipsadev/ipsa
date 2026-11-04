// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {IssuerToken} from "../src/IssuerToken.sol";

contract IssuerTokenTest is Test {
    address internal ics20 = makeAddr("ics20");
    address internal escrow = makeAddr("escrow");
    address internal console = makeAddr("console derived account");
    address internal holder = makeAddr("holder");
    address internal other = makeAddr("other");

    IssuerToken internal token;

    function setUp() public {
        token = new IssuerToken("Brazilian Real", "BRLA", 7, ics20, console);
    }

    function receiveFromStellar(address to, uint256 amount) internal {
        vm.prank(ics20);
        token.mint(escrow, amount);

        vm.prank(escrow);
        assertTrue(token.transfer(to, amount));
    }

    function test_KeepsStellarDecimals() public view {
        assertEq(token.decimals(), 7);
    }

    function test_OnlyIcs20MintsAndBurns() public {
        vm.expectRevert(abi.encodeWithSelector(IssuerToken.NotIcs20.selector, address(this)));
        token.mint(holder, 1);

        vm.expectRevert(abi.encodeWithSelector(IssuerToken.NotIcs20.selector, address(this)));
        token.burn(escrow, 1);
    }

    function test_AnArrivingTransferReachesTheHolder() public {
        receiveFromStellar(holder, 100);

        assertEq(token.balanceOf(holder), 100);
        assertEq(token.totalSupply(), 100);
    }

    function test_AReturningTransferIsBurnedFromTheEscrow() public {
        receiveFromStellar(holder, 100);

        vm.prank(holder);
        assertTrue(token.transfer(escrow, 40));

        vm.prank(ics20);
        token.burn(escrow, 40);

        assertEq(token.totalSupply(), 60);
    }

    function test_AFrozenHolderCanNeitherSendNorReceive() public {
        receiveFromStellar(holder, 100);

        vm.prank(console);
        token.freeze(holder);

        vm.prank(holder);
        vm.expectRevert(abi.encodeWithSelector(IssuerToken.AccountFrozen.selector, holder));
        token.transfer(other, 1);

        vm.prank(ics20);
        token.mint(escrow, 5);

        vm.prank(escrow);
        vm.expectRevert(abi.encodeWithSelector(IssuerToken.AccountFrozen.selector, holder));
        token.transfer(holder, 5);
    }

    function test_AnUnfrozenHolderCanSendAgain() public {
        receiveFromStellar(holder, 100);

        vm.startPrank(console);
        token.freeze(holder);
        token.unfreeze(holder);
        vm.stopPrank();

        vm.prank(holder);
        assertTrue(token.transfer(other, 1));

        assertEq(token.balanceOf(other), 1);
    }

    function test_PausingStopsEveryMovement() public {
        receiveFromStellar(holder, 100);

        vm.prank(console);
        token.pause();

        vm.prank(holder);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.transfer(other, 1);

        vm.prank(ics20);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.mint(escrow, 1);

        vm.prank(console);
        token.unpause();

        vm.prank(holder);
        assertTrue(token.transfer(other, 1));
    }

    function test_OnlyTheAdminFreezesPausesOrHandsOver() public {
        vm.startPrank(other);

        vm.expectRevert(abi.encodeWithSelector(IssuerToken.NotAdmin.selector, other));
        token.freeze(holder);

        vm.expectRevert(abi.encodeWithSelector(IssuerToken.NotAdmin.selector, other));
        token.pause();

        vm.expectRevert(abi.encodeWithSelector(IssuerToken.NotAdmin.selector, other));
        token.setAdmin(other);

        vm.stopPrank();
    }

    function test_ZeroAddressesAreRefused() public {
        vm.expectRevert(IssuerToken.ZeroAddress.selector);
        new IssuerToken("Brazilian Real", "BRLA", 7, address(0), console);

        vm.prank(console);
        vm.expectRevert(IssuerToken.ZeroAddress.selector);
        token.setAdmin(address(0));
    }

    function test_TheAdminCanHandOver() public {
        vm.prank(console);
        token.setAdmin(other);

        assertEq(token.admin(), other);

        vm.prank(other);
        token.pause();

        assertTrue(token.paused());
    }
}
