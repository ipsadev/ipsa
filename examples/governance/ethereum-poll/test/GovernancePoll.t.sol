// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {GovernancePoll} from "../src/GovernancePoll.sol";

contract GovernancePollTest is Test {
    address internal owner = makeAddr("owner");
    address internal stellarVoter = makeAddr("derived account of a stellar sender");
    address internal other = makeAddr("other");

    uint64 internal constant CLOSES_AT = 5_000;

    GovernancePoll internal poll;

    function setUp() public {
        vm.warp(1_000);
        poll = new GovernancePoll(owner);
    }

    function options() internal pure returns (string[] memory list) {
        list = new string[](3);
        list[0] = "Yes";
        list[1] = "No";
        list[2] = "Abstain";
    }

    function propose() internal returns (uint32) {
        vm.prank(owner);
        return poll.propose("Raise the treasury limit", options(), CLOSES_AT);
    }

    function test_AProposalStartsWithEmptyTallies() public {
        uint32 id = propose();
        GovernancePoll.Proposal memory proposal = poll.proposal(id);

        assertEq(id, 0);
        assertEq(proposal.tallies.length, 3);
        assertEq(proposal.tallies[0], 0);
        assertEq(proposal.voters, 0);
        assertEq(poll.proposalCount(), 1);
    }

    function test_OnlyTheOwnerProposes() public {
        vm.expectRevert(abi.encodeWithSelector(GovernancePoll.NotOwner.selector, address(this)));
        poll.propose("Raise the treasury limit", options(), CLOSES_AT);
    }

    function test_AVoteCountsOncePerVoter() public {
        uint32 id = propose();

        vm.prank(stellarVoter);
        assertEq(poll.vote(id, 0), 1);

        vm.prank(stellarVoter);
        assertEq(poll.vote(id, 0), 1);

        vm.prank(other);
        assertEq(poll.vote(id, 0), 2);

        GovernancePoll.Proposal memory proposal = poll.proposal(id);

        assertEq(proposal.tallies[0], 2);
        assertEq(proposal.voters, 2);

        (bool voted, uint32 option) = poll.voteOf(id, stellarVoter);

        assertTrue(voted);
        assertEq(option, 0);
    }

    function test_AChangedVoteMovesBetweenOptions() public {
        uint32 id = propose();

        vm.startPrank(stellarVoter);
        poll.vote(id, 0);
        assertEq(poll.vote(id, 1), 1);
        vm.stopPrank();

        GovernancePoll.Proposal memory proposal = poll.proposal(id);

        assertEq(proposal.tallies[0], 0);
        assertEq(proposal.tallies[1], 1);
        assertEq(proposal.voters, 1);
    }

    function test_AVoteAfterClosingIsRefused() public {
        uint32 id = propose();

        vm.warp(CLOSES_AT);
        vm.expectRevert(abi.encodeWithSelector(GovernancePoll.Closed.selector, id));
        poll.vote(id, 0);
    }

    function test_AnUnknownProposalOrOptionIsRefused() public {
        uint32 id = propose();

        vm.expectRevert(abi.encodeWithSelector(GovernancePoll.UnknownProposal.selector, id + 1));
        poll.vote(id + 1, 0);

        vm.expectRevert(abi.encodeWithSelector(GovernancePoll.UnknownOption.selector, 3));
        poll.vote(id, 3);
    }

    function test_AMalformedProposalIsRefused() public {
        string[] memory one = new string[](1);
        one[0] = "Yes";

        vm.startPrank(owner);

        vm.expectRevert(GovernancePoll.InvalidTitle.selector);
        poll.propose("", options(), CLOSES_AT);

        vm.expectRevert(GovernancePoll.InvalidOptions.selector);
        poll.propose("Raise the treasury limit", one, CLOSES_AT);

        vm.expectRevert(GovernancePoll.InvalidClosingTime.selector);
        poll.propose("Raise the treasury limit", options(), 1_000);

        vm.stopPrank();
    }

    function test_ANotVotedAccountReportsNoVote() public {
        uint32 id = propose();
        (bool voted,) = poll.voteOf(id, other);

        assertFalse(voted);
    }
}
