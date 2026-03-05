// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title VeriPhysLite
 * @author Narukihto / VeriPhys Team
 * @notice High-performance anchoring system for the VeriPhys protocol.
 * @dev Optimized for EIP-1153 (where applicable) and minimal cold-storage access.
 */
contract IntegrityLedger {
    
    // Mapping of SHA3-256 fingerprints to their anchoring block timestamp
    // private visibility reduces contract size; handled via explicit getters
    mapping(bytes32 => uint256) private _ledger;

    // Custom errors: Save ~50-100 gas per revert compared to strings
    error HashAlreadyAnchored(bytes32 hash, uint256 anchoredAt);
    error InvalidHash();

    /**
     * @dev Anchored event includes indexed parameters for high-speed 
     * off-chain log filtering in the Rust engine.
     */
    event Anchored(bytes32 indexed hash, uint256 indexed timestamp);

    /**
     * @notice Securely anchors a unique fingerprint to the blockchain.
     * @param _hash The 32-byte cryptographic hash from the Rust engine.
     */
    function anchorContent(bytes32 _hash) external {
        // 1. Zero-check: Prevents anchoring of null data
        if (_hash == bytes32(0)) revert InvalidHash();
        
        // 2. Existence check: Using a single SLOAD to save gas
        uint256 existingTime = _ledger[_hash];
        if (existingTime != 0) {
            revert HashAlreadyAnchored(_hash, existingTime);
        }

        // 3. State update: Writing to storage (SSTORE)
        _ledger[_hash] = block.timestamp;

        // 4. Notification: Emitting event for indexers
        emit Anchored(_hash, block.timestamp);
    }

    /**
     * @notice Direct verification of a hash.
     * @param _hash The fingerprint to look up.
     * @return timestamp The Unix timestamp of when it was secured (0 if not found).
     */
    function verify(bytes32 _hash) external view returns (uint256) {
        return _ledger[_hash];
    }
    
    /**
     * @notice Boolean existence check for optimized backend validation logic.
     * @param _hash The fingerprint to check.
     */
    function exists(bytes32 _hash) external view returns (bool) {
        return _ledger[_hash] != 0;
    }

    /**
     * @notice Batch verification helper to reduce RPC calls from the Rust server.
     * @param _hashes Array of fingerprints to verify in one call.
     */
    function verifyBatch(bytes32[] calldata _hashes) external view returns (uint256[] memory) {
        uint256[] memory timestamps = new uint256[](_hashes.length);
        for (uint256 i = 0; i < _hashes.length; i++) {
            timestamps[i] = _ledger[_hashes[i]];
        }
        return timestamps;
    }
}
