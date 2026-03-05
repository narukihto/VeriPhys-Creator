// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title VeriPhysLite
 * @dev Optimized for minimum gas consumption and maximum throughput.
 */
contract IntegrityLedger {
    
    // Mapping SHA3-256 hash -> Block Timestamp
    // Using uint256 directly is the cheapest way to store on-chain data.
    mapping(bytes32 => uint256) public ledger;

    // Custom errors save ~50-100 gas per transaction compared to require strings
    error AlreadyAnchored();

    // Event for off-chain indexing
    event Anchored(bytes32 indexed hash, uint256 time);

    /**
     * @notice Secure a fingerprint with minimum gas.
     * @param _hash The SHA3-256 hash from the Rust engine.
     */
    function anchorContent(bytes32 _hash) external {
        // Gas Check: If value is not 0, it's already there
        if (ledger[_hash] != 0) revert AlreadyAnchored();

        // Write to storage (The most expensive part, now minimized)
        ledger[_hash] = block.timestamp;

        emit Anchored(_hash, block.timestamp);
    }

    /**
     * @notice Instant verification.
     * @return timestamp The time it was secured (0 if not found).
     */
    function verify(bytes32 _hash) external view returns (uint256) {
        return ledger[_hash];
    }
}
