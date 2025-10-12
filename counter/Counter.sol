// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 number;

    function setNumber(uint256 newNumber) public {
        number = newNumber;
    }

    function mulNumber(uint256 newNumber) public {
        number = number * newNumber;
    }
    
    function addNumber(uint256 newNumber) public {
        number = number + newNumber;
    }
    
    function increment() public {
        number = number + 1;
    }
    
    function addFromMsgValue() public payable {
        number = number + msg.value;
    }

    function getNumber() view public returns (uint256) {
        return number;
    }
}