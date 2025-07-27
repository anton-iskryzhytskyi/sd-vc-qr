# SD-JWT VC Performance Benchmarks

Performance comparison of cryptographic algorithms for SD-JWT Verifiable Credentials.

## How to run
[locally](./data-analysis/README.md).  
[cloud](./infra/README.md).

## Algorithms

**Classical:** Ed25519, secp256k1, P-256  
**Post-Quantum:** Dilithium-2, Falcon-512, SPHINCS+-128s

## Results

- `results/` - Generated plots and reports
- `target/criterion/` - Raw benchmark data

## Author
Anton Iskyzhytskyi <a.iskryzhytskyi@gmail.com>