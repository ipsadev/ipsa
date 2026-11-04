// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

contract GovernancePoll {
    uint256 public constant MAXIMUM_TITLE_LENGTH = 140;
    uint256 public constant MAXIMUM_OPTION_LENGTH = 40;
    uint256 public constant MINIMUM_OPTIONS = 2;
    uint256 public constant MAXIMUM_OPTIONS = 8;

    struct Proposal {
        uint32 id;
        string title;
        string[] options;
        uint32[] tallies;
        uint32 voters;
        uint64 closesAt;
    }

    address public owner;

    Proposal[] private _proposals;

    mapping(uint32 proposal => mapping(address voter => uint32 optionPlusOne)) private _votes;

    event Proposed(uint32 indexed proposal, string title, uint64 closesAt);
    event Voted(uint32 indexed proposal, address indexed voter, uint32 option, uint32 tally);
    event OwnerChanged(address indexed previousOwner, address indexed newOwner);

    error NotOwner(address caller);
    error UnknownProposal(uint32 proposal);
    error UnknownOption(uint32 option);
    error Closed(uint32 proposal);
    error InvalidTitle();
    error InvalidOptions();
    error InvalidClosingTime();
    error ZeroAddress();

    modifier onlyOwner() {
        require(msg.sender == owner, NotOwner(msg.sender));
        _;
    }

    constructor(address owner_) {
        require(owner_ != address(0), ZeroAddress());

        owner = owner_;

        emit OwnerChanged(address(0), owner_);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), ZeroAddress());

        emit OwnerChanged(owner, newOwner);

        owner = newOwner;
    }

    function propose(string calldata title, string[] calldata options, uint64 closesAt)
        external
        onlyOwner
        returns (uint32 id)
    {
        uint256 titleLength = bytes(title).length;

        require(titleLength > 0 && titleLength <= MAXIMUM_TITLE_LENGTH, InvalidTitle());
        require(options.length >= MINIMUM_OPTIONS && options.length <= MAXIMUM_OPTIONS, InvalidOptions());

        for (uint256 index = 0; index < options.length; index++) {
            uint256 optionLength = bytes(options[index]).length;

            require(optionLength > 0 && optionLength <= MAXIMUM_OPTION_LENGTH, InvalidOptions());
        }

        require(closesAt > block.timestamp, InvalidClosingTime());

        id = uint32(_proposals.length);

        Proposal storage proposal = _proposals.push();

        proposal.id = id;
        proposal.title = title;
        proposal.tallies = new uint32[](options.length);

        for (uint256 index = 0; index < options.length; index++) {
            proposal.options.push(options[index]);
        }
        proposal.closesAt = closesAt;

        emit Proposed(id, title, closesAt);
    }

    function vote(uint32 proposalId, uint32 option) external returns (uint32 tally) {
        Proposal storage proposal = _proposal(proposalId);

        require(block.timestamp < proposal.closesAt, Closed(proposalId));
        require(option < proposal.options.length, UnknownOption(option));

        uint32 previous = _votes[proposalId][msg.sender];

        if (previous == option + 1) {
            return proposal.tallies[option];
        }

        if (previous == 0) {
            proposal.voters += 1;
        } else {
            proposal.tallies[previous - 1] -= 1;
        }

        tally = proposal.tallies[option] + 1;
        proposal.tallies[option] = tally;
        _votes[proposalId][msg.sender] = option + 1;

        emit Voted(proposalId, msg.sender, option, tally);
    }

    function proposal(uint32 proposalId) external view returns (Proposal memory) {
        return _proposal(proposalId);
    }

    function proposals() external view returns (Proposal[] memory) {
        return _proposals;
    }

    function proposalCount() external view returns (uint256) {
        return _proposals.length;
    }

    function voteOf(uint32 proposalId, address voter) external view returns (bool voted, uint32 option) {
        _proposal(proposalId);

        uint32 stored = _votes[proposalId][voter];

        if (stored == 0) {
            return (false, 0);
        }

        return (true, stored - 1);
    }

    function _proposal(uint32 proposalId) private view returns (Proposal storage) {
        require(proposalId < _proposals.length, UnknownProposal(proposalId));

        return _proposals[proposalId];
    }
}
