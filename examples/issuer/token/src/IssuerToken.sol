// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

contract IssuerToken is ERC20, Pausable {
    address public immutable ics20;
    address public admin;

    mapping(address account => bool) public frozen;

    uint8 private immutable _decimals;

    event Frozen(address indexed account);
    event Unfrozen(address indexed account);
    event AdminChanged(address indexed previousAdmin, address indexed newAdmin);

    error NotIcs20(address caller);
    error NotAdmin(address caller);
    error AccountFrozen(address account);
    error ZeroAddress();

    modifier onlyIcs20() {
        _checkIcs20();
        _;
    }

    modifier onlyAdmin() {
        _checkAdmin();
        _;
    }

    constructor(string memory name_, string memory symbol_, uint8 decimals_, address ics20_, address admin_)
        ERC20(name_, symbol_)
    {
        require(ics20_ != address(0) && admin_ != address(0), ZeroAddress());

        ics20 = ics20_;
        admin = admin_;
        _decimals = decimals_;

        emit AdminChanged(address(0), admin_);
    }

    function decimals() public view override returns (uint8) {
        return _decimals;
    }

    function mint(address mintAddress, uint256 amount) external onlyIcs20 {
        _mint(mintAddress, amount);
    }

    function burn(address burnAddress, uint256 amount) external onlyIcs20 {
        _burn(burnAddress, amount);
    }

    function freeze(address account) external onlyAdmin {
        frozen[account] = true;
        emit Frozen(account);
    }

    function unfreeze(address account) external onlyAdmin {
        frozen[account] = false;
        emit Unfrozen(account);
    }

    function pause() external onlyAdmin {
        _pause();
    }

    function unpause() external onlyAdmin {
        _unpause();
    }

    function setAdmin(address newAdmin) external onlyAdmin {
        require(newAdmin != address(0), ZeroAddress());

        address previousAdmin = admin;
        admin = newAdmin;

        emit AdminChanged(previousAdmin, newAdmin);
    }

    function _checkIcs20() internal view {
        require(msg.sender == ics20, NotIcs20(msg.sender));
    }

    function _checkAdmin() internal view {
        require(msg.sender == admin, NotAdmin(msg.sender));
    }

    function _update(address from, address to, uint256 value) internal override whenNotPaused {
        require(!frozen[from], AccountFrozen(from));
        require(!frozen[to], AccountFrozen(to));

        super._update(from, to, value);
    }
}
