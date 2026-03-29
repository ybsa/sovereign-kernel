# Security Policy

## Supported Versions

Currently, the Sovereign Kernel is in active development (`v1.0.0`). Only the `main` branch and the latest tagged release receive security patches.

| Version | Supported          |
|---------|--------------------|
| v1.0.x  | :white_check_mark: |
| `main`  | :white_check_mark: |
| < v1.0  | :x:                |

## Security Architecture ("The Warden")

Sovereign Kernel provides deep layer isolation for LLM-driven agents to ensure safe autonomous operations.
Key defenses include:

- **Filesystem Sandbox:** `Landlock LSM` restricts file I/O to approved directories.
- **Syscall Filtering:** `seccomp-bpf` enforcement limits the kernel APIs agents can touch.
- **Approval Gates:** High-risk actions (code execution, financial trades, destructive edits) require pre-approval.
- **Audit Trails:** A tamper-evident Merkle tree logs every action an agent takes for forensics.

## Reporting a Vulnerability

If you discover a security vulnerability in the Sovereign Kernel or its sandboxing subsystems, please DO NOT open a public issue.

Instead, please send an email to the project maintainers directly or use GitHub's private vulnerability reporting feature.

- We will acknowledge receipt of your vulnerability report within 48 hours.
- We will provide regular updates on our progress in addressing the issue.
- Please allow up to 90 days for the vulnerability to be patched before disclosing it publicly.

We ask that you do not share details of the vulnerability with anyone outside of the core maintenance team until a patch has been released.
