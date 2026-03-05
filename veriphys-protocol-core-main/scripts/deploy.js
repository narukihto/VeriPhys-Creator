const hrp = require("hardhat");

async function main() {
  console.log("🚀 Starting deployment of VeriPhys Protocol...");

  // Get the contract factory
  const VeriPhysLedger = await hrp.ethers.getContractFactory("VeriPhysLedger");

  // Deploy the contract
  const contract = await VeriPhysLedger.deploy();

  await contract.deployed();

  console.log("✅ VeriPhysLedger deployed to:", contract.address);
  console.log("📝 Copy this address and paste it into your .env file.");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
